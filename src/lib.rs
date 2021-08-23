pub extern crate firecore_pokedex_engine as pokedex;
pub extern crate firecore_battle as battle;

use std::{collections::VecDeque, rc::Rc, fmt::{Debug, Display}};

use context::BattleGuiContext;

use log::{warn, debug};

use pokedex::{Identifiable, battle_move::BattleMovedex, context::PokedexClientContext, gui::{bag::BagGui, party::PartyGui}, item::{Itemdex, bag::Bag, usage::{ItemUsageKind}}, moves::{MoveTarget, Movedex}, pokemon::{OwnedIdPokemon, OwnedRefPokemon, Party, Pokedex}};

use pokedex::engine::{
    graphics::ZERO, 
    tetra::{Context, math::Vec2, graphics::Color}, 
    util::{Entity, Completable, Reset},
    EngineContext,
};

use battle::{BattleData, BattleEndpoint, BattleType, BoundAction, message::{ClientMessage, ServerMessage}, moves::{client::{ClientAction, ClientMove, ClientActions}, usage::target::{MoveTargetInstance, MoveTargetLocation}, BattleMove}, pokemon::{PokemonIndex, battle::UninitUnknownPokemon}};

use self::{
    ui::{
        BattleGui,
        panels::BattlePanels,
        view::{
            GuiLocalPlayer,
            GuiRemotePlayer,
            ActivePokemonRenderer,
        },
    },
    view::PlayerView,
};

pub mod action;
pub mod view;
pub mod transition;
pub mod ui;
pub mod context;

use action::*;

use self::transition::TransitionState;

pub struct BattlePlayerGui<'d, ID: Default> {

    context: BattleGuiContext,

    party: Rc<PartyGui>,
    bag: Rc<BagGui>,
	pub gui: BattleGui<'d>,

    state: BattlePlayerState<'d, ID>,
    should_select: bool,

    pub data: BattleData,

    pub local: GuiLocalPlayer<'d, ID>,
    pub remote: GuiRemotePlayer<'d, ID>,

    messages: Messages<ID>,

    pokedex: &'d Pokedex,
    movedex: &'d Movedex,
    itemdex: &'d Itemdex,

}

struct Messages<ID> {
    client: Vec<ServerMessage<ID>>,
    server: Vec<ClientMessage>,
}

impl<ID> Messages<ID> {
    pub fn send(&mut self, message: ClientMessage) {
        self.server.insert(0, message)
    }
}

impl<ID> Default for Messages<ID> {
    fn default() -> Self {
        Self {
            client: Default::default(),
            server: Default::default(),
        }
    }
}

#[derive(Debug)]
struct MoveQueue<'d, ID> {
    actions: VecDeque<BoundAction<ID, BattleClientGuiAction<'d, ID>>>,
    current: Option<BoundAction<ID, BattleClientGuiCurrent<ID>>>,
}


#[derive(Debug)]
enum BattlePlayerState<'d, ID> {
    WaitToStart,
    Opening(TransitionState),
    Introduction(TransitionState),
    WaitToSelect,
    Select(usize),
    Moving(MoveQueue<'d, ID>),
    Winner(Option<ID>),
}

impl<'d, ID: Default> BattleEndpoint<ID> for BattlePlayerGui<'d, ID> {
    fn send(&mut self, message: ServerMessage<ID>) {
        self.messages.client.insert(0, message)
    }
    fn receive(&mut self) -> Option<ClientMessage> {
        self.messages.server.pop()
    }
}

impl<'d, ID: Sized + Default + Copy + Debug + Display + Eq + Ord> BattlePlayerGui<'d, ID> {

    pub fn new(ctx: &mut Context, dex: &PokedexClientContext<'d>, party: Rc<PartyGui>, bag: Rc<BagGui>) -> Self {
        let context = BattleGuiContext::new(ctx);
        Self {
            party,
            bag,
			gui: BattleGui::new(ctx, &context),
            state: BattlePlayerState::WaitToStart,
            should_select: false,
            data: Default::default(),
            local: Default::default(),
            remote: Default::default(),
            messages: Default::default(),
            context,
            pokedex: dex.pokedex,
            movedex: dex.movedex,
            itemdex: dex.itemdex,
        }
    }

    pub fn winner(&self) -> Option<Option<ID>> {
        if let BattlePlayerState::Winner(w) = self.state {
            Some(w)
        } else {
            None
        }
    }

    pub fn battling(&self) -> bool {
        !matches!(self.state, BattlePlayerState::WaitToStart | BattlePlayerState::Opening(..) | BattlePlayerState::Introduction(..))
    }

    pub fn start(&mut self, transition: bool) {
        self.state = match transition {
            true => BattlePlayerState::Opening(TransitionState::default()),
            false => BattlePlayerState::WaitToSelect,
        };
    }

    pub fn process(&mut self, random: &mut impl rand::Rng, dex: &PokedexClientContext, party: &mut Party<OwnedRefPokemon<'d>>) {
        while let Some(message) = self.messages.client.pop() {
            match message {
                ServerMessage::Begin(data) => {
                    self.local.player = battle::player::PlayerKnowable {
                        name: data.name,
                        party: battle::party::PlayerParty {
                            id: data.id,
                            active: data.active,
                            pokemon: party.clone(),
                        },
                    };
                    self.remote.player.party.active = data.remote.party.active;
                    self.remote.player.party.id = data.remote.party.id;
                    self.remote.player.name = data.remote.name;
                    self.remote.player.party.pokemon = data.remote.party.pokemon.into_iter().flat_map(|u| u.map(|u| UninitUnknownPokemon::init(u, self.pokedex))).collect();
                    self.data = data.data;
                },
                ServerMessage::StartSelecting => {
                    self.should_select = true;
                    self.gui.panel.despawn();
                },
                ServerMessage::TurnQueue(queue) => {
                    self.state = BattlePlayerState::Moving(MoveQueue {
                        actions: queue.into_iter().map(|a| BoundAction {
                            pokemon: a.pokemon,
                            action: BattleClientGuiAction::Action(a.action),
                        }).collect(),
                        current: None,
                    });
                    self.gui.text.clear();
                    self.gui.text.spawn();
                },
                ServerMessage::FaintReplace(pokemon, new) => {
                    match &mut self.state {
                        BattlePlayerState::Moving(queue) => {
                            queue.actions.push_back(BoundAction {
                                pokemon,
                                action: BattleClientGuiAction::Replace(Some(new)),
                            });
                        },
                        _ => {
                            let (renderer, pokemon) = match pokemon.team == self.local.id {
                                true => {
                                    self.local.party.replace(pokemon.index, Some(new));
                                    let renderer = &mut self.local.renderer[pokemon.index];
                                    let pokemon = self.local.player.party.active(pokemon.index);
                                    let id = pokemon.map(|p| p.pokemon.id);
                                    renderer.status.update_gui(pokemon, None, true);
                                    (renderer, id)
                                },
                                false => {
                                    self.remote.party.replace(pokemon.index, Some(new));
                                    let renderer = &mut self.remote.renderer[pokemon.index];
                                    let pokemon = self.remote.player.party.active(pokemon.index).map(|u| u as _);
                                    let id = pokemon.map(|v| view::GuiPokemonView::pokemon(v).id);
                                    renderer.status.update_gui_view(pokemon, None, true);
                                    (renderer, id)
                                }
                            };
                            renderer.renderer.new_pokemon(dex, pokemon);
                        }
                    }
                },
                ServerMessage::AddUnknown(index, unknown) => match UninitUnknownPokemon::init(unknown, self.pokedex) {
                    Some(unknown) => self.remote.party.add_unknown(index, unknown),
                    None => warn!("Could not initialize unknown pokemon at index {}", index),
                },
                ServerMessage::Winner(player) => {
                    self.state = BattlePlayerState::Winner(player);
                    for (index, pokemon) in self.local.party.pokemon.iter().enumerate() {
                        party[index] = pokemon.clone();
                    }
                }
                ServerMessage::ConfirmFaintReplace(index, can) => {
                    debug!("to - do: checking faint replace");
                    if !can {
                        debug!("cannot replace pokemon at active index {}", index);
                    }
                },
                ServerMessage::Catch(instance) => match OwnedIdPokemon::init(instance, random, self.pokedex, self.movedex, self.itemdex) {
                    Some(instance) => if let Ok(_) = party.try_push(instance) {},
                    None => warn!("Could not initialize caught pokemon.")
                }
                // ServerMessage::AddMove(pokemon, index, move_ref) => if pokemon.team == self.local.party.id {
                //     if let Some(pokemon) = self.local.party.pokemon.get_mut(pokemon.index) {
                //         debug!("to - do: set move to its index.");
                //         if let Err(err) = pokemon.moves.try_push(MoveInstance::new(move_ref)) {
                //             warn!("Cannot add moves to {} because it has maximum number of moves. error: {}", pokemon.name(), err)
                //         }
                //     }
                // }
            }
        }
    }

    pub fn update(&mut self, ctx: &EngineContext, dex: &PokedexClientContext, delta: f32, bag: &mut Bag<'d>) {
        match &mut self.state {
            BattlePlayerState::WaitToStart | BattlePlayerState::Winner(..) => (),
            BattlePlayerState::Opening(state) => match state {
                TransitionState::Begin => {
                    self.gui.opener.begin(dex, state, self.data.type_, &self.remote);
                    if !matches!(self.data.type_, BattleType::Wild) {
                        self.gui.trainer.spawn(self.local.party.pokemon.len(), self.remote.party.pokemon.len());
                    }
                    self.update(ctx, dex, delta, bag);
                }
                TransitionState::Run => self.gui.opener.update::<ID>(state, delta),
                TransitionState::End => {
                    self.state = BattlePlayerState::Introduction(TransitionState::default());
                    self.update(ctx, dex, delta, bag);
                }
            }
            BattlePlayerState::Introduction(state) => match state {
                TransitionState::Begin => {
                    self.gui.introduction.begin(dex, state, self.data.type_, &self.local, &self.remote, &mut self.gui.text);
                    self.update(ctx, dex, delta, bag);
                }
                TransitionState::Run => {
                    self.gui.introduction.update(state, ctx, delta, &mut self.local, &mut self.remote, &mut self.gui.text);
                    self.gui.trainer.update(delta);
                    if self.gui.text.page() > 0 && !self.gui.trainer.ending() && !matches!(self.data.type_, BattleType::Wild) {
                        self.gui.trainer.end();
                    }
                }
                TransitionState::End => {
                    self.gui.introduction.end(&mut self.gui.text);
                    self.gui.trainer.despawn();
                    self.state = BattlePlayerState::WaitToSelect;
                    self.update(ctx, dex, delta, bag);
                }
            }
            BattlePlayerState::WaitToSelect => if self.should_select {
                self.should_select = false;
                self.state = BattlePlayerState::Select(0);
            }
            BattlePlayerState::Select(active_index) => {
                self.gui.bounce.update(delta);
                match self.local.party.active.get(*active_index) {
                    Some(index) => match index {
                        Some(index) => {
                            let pokemon = &self.local.party.pokemon[*index];
                            match self.gui.panel.alive() {
                                true => {
        
                                    // Checks if a move is queued from an action done in the GUI
        
                                    if self.bag.alive() {
                                        self.bag.input(ctx, &mut bag.items);
                                        if let Some(item) = self.bag.take_selected_despawn(&mut bag.items) {
                                            match &item.usage.kind {
                                                ItemUsageKind::Actions(..) => todo!(),
                                                ItemUsageKind::Script => todo!("user targeting"),
                                                ItemUsageKind::Pokeball => self.gui.panel.active = BattlePanels::Target(MoveTarget::Opponent, Some(item)),
                                                ItemUsageKind::None => todo!("make item unusable"),
                                                // ItemUsageKind::Pokeball => ,
                                                // ItemUsageKind::Script(..) => ,
                                                // ItemUsageKind::None => ,
                                            }
                                        }
                                    } else if self.party.alive() {
                                        self.party.input(ctx, dex, self.local.party.pokemon.as_mut_slice());
                                        self.party.update(delta);
                                        if let Some(selected) = self.party.take_selected() {
                                            self.party.despawn();
                                            self.messages.server.push(
                                                ClientMessage::Move(
                                                    *active_index,
                                                    BattleMove::Switch(selected)
                                                )
                                            );
                                            *active_index += 1;
                                            self.gui.panel.despawn();
                                        }
                                    } else if let Some(panels) = self.gui.panel.input(ctx, pokemon) {
                                        match panels {
                                            BattlePanels::Main => {
                                                match self.gui.panel.battle.cursor {
                                                    0 => self.gui.panel.active = BattlePanels::Fight,
                                                    1 => self.bag.spawn(),
                                                    2 => self.party.spawn(dex, &self.local.party.pokemon, Some(false), true),
                                                    3 => if matches!(self.data.type_, BattleType::Wild) {
                                                        self.messages.send(ClientMessage::Forfeit);
                                                    },
                                                    _ => unreachable!(),
                                                }
                                            }
                                            BattlePanels::Fight => match pokemon.moves.get(self.gui.panel.fight.moves.cursor) {
                                                Some(instance) => match instance.try_use() {
                                                    Some(move_ref) => {
                                                        match move_ref.target {
                                                            MoveTarget::Opponent | MoveTarget::Any => {
                                                                self.gui.panel.target(&self.remote.player);
                                                                self.gui.panel.active = BattlePanels::Target(move_ref.target, None);
                                                            },
                                                            MoveTarget::Ally | MoveTarget::UserOrAlly => {
                                                                self.gui.panel.target(&self.local.player);
                                                                self.gui.panel.active = BattlePanels::Target(move_ref.target, None);
                                                            }
                                                            _ => {
                                                                self.messages.send(
                                                                    ClientMessage::Move(
                                                                        *active_index,
                                                                        BattleMove::Move(
                                                                            self.gui.panel.fight.moves.cursor,
                                                                            match move_ref.target {
                                                                                MoveTarget::User => MoveTargetInstance::User,
                                                                                MoveTarget::AllOtherPokemon => MoveTargetInstance::AllOtherPokemon,
                                                                                MoveTarget::AllOpponents => MoveTargetInstance::AllOpponents,
                                                                                MoveTarget::Allies => MoveTargetInstance::Allies,
                                                                                MoveTarget::RandomOpponent => MoveTargetInstance::RandomOpponent,
                                                                                MoveTarget::None => MoveTargetInstance::None,
                                                                                MoveTarget::UserAndAllies => MoveTargetInstance::UserAndAllies,
                                                                                MoveTarget::AllPokemon => MoveTargetInstance::AllPokemon,
                                                                                _ => unreachable!(),
                                                                            }
                                                                        )
                                                                    )
                                                                );
                                                                *active_index += 1;
                                                                self.gui.panel.despawn();
                                                            }
                                                        }
                                                    }
                                                    None => warn!("Pokemon is out of Power Points for this move!"),
                                                }
                                                None => warn!("Could not get move at cursor!"),
                                            }
                                            BattlePanels::Target(target, item) => {
                                                self.messages.send(
                                                    ClientMessage::Move(
                                                        *active_index,
                                                        match item {
                                                            Some(item) => BattleMove::UseItem(
                                                                *item.id(),
                                                                match target {
                                                                    MoveTarget::Opponent => self.gui.panel.targets.cursor,
                                                                    _ => unreachable!(),
                                                                }
                                                            ),
                                                            None => BattleMove::Move(
                                                                self.gui.panel.fight.moves.cursor, 
                                                                match target {
                                                                    MoveTarget::Opponent => MoveTargetInstance::Opponent(self.gui.panel.targets.cursor),
                                                                    MoveTarget::Ally => MoveTargetInstance::Ally(self.gui.panel.targets.cursor),
                                                                    MoveTarget::Any => MoveTargetInstance::Any(false, self.gui.panel.targets.cursor),
                                                                    _ => unreachable!(),
                                                                }
                                                            ),
                                                        }
                                                    )
                                                );
                                                *active_index += 1;
                                                self.gui.panel.despawn();
                                            }
                                        }
                                    }
                                }
                                false => {
                                    self.gui.panel.user(pokemon);
                                    self.gui.panel.spawn();
                                }
                            }
                        },
                        None => *active_index += 1,
                    },
                    None => {
                        self.gui.panel.despawn();
                    },
                }
            },
            BattlePlayerState::Moving(queue) => {

                match &mut queue.current {
                    None => {
                        match queue.actions.pop_front() {
                            None => {
                                self.messages.send(ClientMessage::FinishedTurnQueue);
                                self.state = BattlePlayerState::WaitToSelect;
                            }
                            Some(instance) => {

                                // to - do: better client checking

                                let (user, user_ui, other, other_ui) = if instance.pokemon.team == self.local.party.id {
                                    (&mut self.local.player as &mut dyn PlayerView<ID>, &mut self.local.renderer, &mut self.remote.player as &mut dyn PlayerView<ID>, &mut self.remote.renderer)
                                } else {
                                    (&mut self.remote.player as _, &mut self.remote.renderer, &mut self.local.player as _, &mut self.local.renderer)
                                };

                                self.gui.text.clear();
                                self.gui.text.reset();

                                if user.active(instance.pokemon.index).is_some() || !instance.action.requires_user() {

                                    if let Some(action) = match instance.action {
                                        BattleClientGuiAction::Action(action) => match action {
                                            ClientMove::Move(pokemon_move, targets) => {

                                                match self.movedex.try_get(&pokemon_move) {
                                                    Some(pokemon_move) => {
                                                        let user_active = user.active(instance.pokemon.index).unwrap();

                                                        // if targets.iter().any(|(t, _)| match &t {
                                                        //     MoveTargetLocation::Opponent(index) => other.active(*index),
                                                        //     MoveTargetLocation::Team(index) => user.active(*index),
                                                        //     MoveTargetLocation::User => user.active(instance.pokemon.index),
                                                        // }.map(|v| !v.fainted()).unwrap_or_default()) {
        
                                                        ui::text::on_move(&mut self.gui.text, &pokemon_move, user_active.name());
        
                                                        // }
            
                                                        for ClientActions { location, actions } in &targets {
            
                                                            {
            
                                                                let user_pokemon = user.active_mut(instance.pokemon.index).unwrap();
            
                                                                let user_pokemon_ui = &mut user_ui[instance.pokemon.index];
        
                                                                // if let Some(battle_move) = BattleMovedex::try_get(pokemon_move.id()) {
                                                                //     user_pokemon_ui.renderer.moves.init(battle_move.script());
                                                                // } 
        
                                                                for moves in actions {
                                                                    match moves {
                                                                        ClientAction::UserHP(damage) => user_pokemon.set_hp(*damage),
                                                                        ClientAction::Fail => ui::text::on_fail(&mut self.gui.text, vec![format!("{} cannot use move", user_pokemon.name()), format!("{} is unimplemented", pokemon_move.name)]),
                                                                        ClientAction::Miss => ui::text::on_miss(&mut self.gui.text, user_pokemon.name()),
                                                                        ClientAction::SetExp(experience, level) => {
                                                                            let previous = user_pokemon.level();
                                                                            user_pokemon.set_level(*level);
                                                                            user_pokemon.set_exp(*experience);
                                                                            if let Some(user_pokemon) = user_pokemon.instance() {
                                                                                let movedex = self.movedex;
                                                                                let moves = user_pokemon.on_level_up(previous).flat_map(|id| movedex.try_get(&id)).collect();
                                                                                queue.actions.push_front(BoundAction { pokemon: instance.pokemon, action: BattleClientGuiAction::SetExp(previous, *experience, moves) });
                                                                            }
                                                                        }
                                                                        _ => (),
                                                                    }
                                                                }
        
                                                                match user_pokemon.instance() {
                                                                    Some(pokemon) => user_pokemon_ui.status.update_gui(Some(pokemon), None, false),
                                                                    None => user_pokemon_ui.status.update_gui_view(Some(user_pokemon), None, false),
                                                                }
            
                                                            }
            
                                                            let (target, target_ui) = match location {
                                                                MoveTargetLocation::Opponent(index) => (other.active_mut(*index), &mut other_ui[*index]),
                                                                MoveTargetLocation::Team(index) => (user.active_mut(*index), &mut user_ui[*index]),
                                                                MoveTargetLocation::User => (user.active_mut(instance.pokemon.index), &mut user_ui[instance.pokemon.index]),
                                                            };
            
                                                            if let Some(target) = target {
                                                                for moves in actions {
                                                                    match moves {
                                                                        ClientAction::TargetHP(damage, crit) => {
                                                                            target.set_hp(*damage);
                                                                            if damage >= &0.0 {
                                                                                target_ui.renderer.flicker()
                                                                            }
                                                                            if *crit {
                                                                                ui::text::on_crit(&mut self.gui.text);
                                                                            }
                                                                        },
                                                                        ClientAction::Effective(effective) => ui::text::on_effective(&mut self.gui.text, &effective),
                                                                        ClientAction::Stat(stat, stage) => ui::text::on_stat_stage(&mut self.gui.text, target.name(), *stat, *stage),
                                                                        ClientAction::Faint(target_instance) => queue.actions.push_front(
                                                                            BoundAction {
                                                                                pokemon: *target_instance,
                                                                                action: BattleClientGuiAction::Faint,
                                                                            }
                                                                        ),
                                                                        ClientAction::Ailment(ailment) => {
                                                                            target.set_ailment(*ailment);
                                                                            ui::text::on_status(&mut self.gui.text, target.name(), ailment.ailment);
                                                                        }
                                                                        ClientAction::Miss | ClientAction::UserHP(..) | ClientAction::SetExp(..) | ClientAction::Fail => (),
                                                                    }
                                                                }
        
                                                                match target.instance() {
                                                                    Some(i) => target_ui.status.update_gui(Some(i), None, false),
                                                                    None => target_ui.status.update_gui_view(Some(target), None, false),
                                                                }
                                                            } else {
                                                                target_ui.status.update_gui(None, None, false);
                                                            }
                                                        }
                                                        Some(BattleClientGuiCurrent::Move(targets))
                                                    }
                                                    None => None,
                                                }
                                            }
                                            ClientMove::UseItem(item, index) => {
                                                if let Some(item) = self.itemdex.try_get(&item) {
                                                    if let Some((id, target)) = match &item.usage.kind {
                                                        ItemUsageKind::Script | ItemUsageKind::Actions(..) => user.active(index).map(|v| (user.id(), v)),
                                                        ItemUsageKind::Pokeball => other.active(index).map(|v| (other.id(), v)),
                                                        ItemUsageKind::None => None,
                                                    } {
                                                        if let ItemUsageKind::Pokeball = &item.usage.kind {
                                                            // self.messages.push(ClientMessage::RequestPokemon(index));
                                                            queue.actions.push_front(BoundAction {
                                                                pokemon: PokemonIndex { team: *id, index },
                                                                action: BattleClientGuiAction::Catch,
                                                            });
                                                        }
                                                        ui::text::on_item(&mut self.gui.text, target.name(), &item);
                                                    }
                                                    Some(BattleClientGuiCurrent::UseItem(match &item.usage.kind {
                                                        ItemUsageKind::Script | ItemUsageKind::Actions(..) | ItemUsageKind::None => match index == instance.pokemon.index {
                                                            true => MoveTargetLocation::User,
                                                            false => MoveTargetLocation::Team(index),
                                                        },
                                                        ItemUsageKind::Pokeball => MoveTargetLocation::Opponent(index),
                                                    }))
                                                } else {
                                                    None
                                                }
                                            }
                                            ClientMove::Switch(index) => {
                                                let coming = user.pokemon(index).map(|v| v.name()).unwrap_or("Unknown");
                                                ui::text::on_switch(&mut self.gui.text, user.active(instance.pokemon.index).map(|v| v.name()).unwrap_or("Unknown"), coming);
                                                Some(BattleClientGuiCurrent::Switch(index))
                                            }
                                        }
                                        BattleClientGuiAction::Faint => {
                                            let is_player = &instance.pokemon.team == user.id();
                                            let target = user.active_mut(instance.pokemon.index).unwrap();
                                            target.set_hp(0.0);
                                            ui::text::on_faint(&mut self.gui.text, matches!(self.data.type_, BattleType::Wild), is_player, target.name());
                                            user_ui[instance.pokemon.index].renderer.faint();
                                            Some(BattleClientGuiCurrent::Faint)
                                        },
                                        BattleClientGuiAction::Catch => {
                                            if let Some(pokemon) = self.remote.active(instance.pokemon.index) {
                                                ui::text::on_catch(&mut self.gui.text, pokemon.name());
                                            }
                                            // if let Some(pokemon) = pokemon {
                                            self.remote.replace(instance.pokemon.index, None);
                                            let renderer = &mut self.remote.renderer[instance.pokemon.index];
                                            renderer.status.update_gui_view(None, None, false);
                                            renderer.renderer.new_pokemon(dex, None);
                                            // }
                                            Some(BattleClientGuiCurrent::Catch)
                                        }
                                        BattleClientGuiAction::Replace(new) => {
                                            ui::text::on_replace(&mut self.gui.text, user.name(), new.map(|index| user.pokemon(index).map(|v| v.name())).flatten());
                                            user.replace(instance.pokemon.index, new);
                                            Some(BattleClientGuiCurrent::Replace(false))
                                        }
                                        // To - do: experience spreading
                                        BattleClientGuiAction::SetExp(previous, experience, moves) => match user.active_mut(instance.pokemon.index) {
                                            Some(pokemon) => {    
                                                ui::text::on_gain_exp(&mut self.gui.text, pokemon.name(), experience, pokemon.level());
                                                let status = &mut user_ui[instance.pokemon.index].status;
                                                match pokemon.instance() {
                                                    Some(p) => status.update_gui(Some(p), Some(previous), false),
                                                    None => status.update_gui_view(Some(pokemon), Some(previous), false),
                                                }
                                                queue.actions.push_front(BoundAction { pokemon: instance.pokemon, action: BattleClientGuiAction::LevelUp(moves) });
                                                Some(BattleClientGuiCurrent::SetExp)
                                            }
                                            None => None,
                                        }
                                        BattleClientGuiAction::LevelUp(moves) => match user.active_mut(instance.pokemon.index).map(|v| v.instance()).flatten() {
                                            Some(instance) => {
                                                match moves.is_empty() {
                                                    false => {
                                                        self.gui.level_up.spawn(instance, &mut self.gui.text, moves);
                                                        Some(BattleClientGuiCurrent::LevelUp)
                                                    }
                                                    true => None,
                                                }
                                            }
                                            None => None,
                                        }
                                        // ClientMove::Catch(index) => {
                                        //     if let Some(target) = match index.team {
                                        //         Team::Player => &user.active[index.active],
                                        //         Team::Opponent => &other.active[index.active],
                                        //     }.pokemon.as_ref() {
                                        //         ui::text::on_catch(text, target);
                                        //     }
                                        // }
                                    } {
                                        queue.current = Some(BoundAction {
                                            pokemon: instance.pokemon,
                                            action
                                        });
                                    } else {
                                        self.update(ctx, dex, delta, bag);
                                    }
                                }
                            },
                        }
                    },
                    Some(instance) => {

                        let (user, user_ui, other_ui) = if instance.pokemon.team == self.local.party.id {
                            (&mut self.local.player as &mut dyn PlayerView<ID>, &mut self.local.renderer, &mut self.remote.renderer)
                        } else {
                            (&mut self.remote.player as _, &mut self.remote.renderer, &mut self.local.renderer)
                        };
                        

                        match &mut instance.action {
                            BattleClientGuiCurrent::Move(targets) => {

                                user_ui[instance.pokemon.index].renderer.moves.update(delta);

                                match self.gui.text.finished() {
                                    false => self.gui.text.update(ctx, delta),
                                    true => if (self.gui.text.page() > 0 || self.gui.text.waiting()) && user_ui[instance.pokemon.index].renderer.moves.finished() {
                                        let index = instance.pokemon.index;
                                        targets.retain(|a| {
                                            let ui = match a.location {
                                                MoveTargetLocation::Opponent(i) => &other_ui[i],
                                                MoveTargetLocation::Team(i) => &user_ui[i],
                                                MoveTargetLocation::User => &user_ui[index],
                                            };
                                            ui.renderer.flicker.flickering() || ui.status.health_moving()
                                        });
                                        if targets.is_empty() {
                                            queue.current = None;
                                        } else {
                                            for target in targets {
                                                let ui = match target.location {
                                                    MoveTargetLocation::Opponent(i) => &mut other_ui[i],
                                                    MoveTargetLocation::Team(i) => &mut user_ui[i],
                                                    MoveTargetLocation::User => &mut user_ui[instance.pokemon.index],
                                                };
                                                ui.renderer.flicker.update(delta);
                                                ui.status.update_hp(delta);
                                            }
                                        }
                                    }
                                }
                            },
                            BattleClientGuiCurrent::Switch(new) => match self.gui.text.finished() {
                                false => {
                                    self.gui.text.update(ctx, delta);

                                    if self.gui.text.page() == 1 && !user.active_eq(instance.pokemon.index, &Some(*new)) {
                                        user.replace(instance.pokemon.index, Some(*new));
                                        let renderer = &mut user_ui[instance.pokemon.index];
                                        let id = match user.active_mut(instance.pokemon.index) {
                                            Some(user) => Some(match user.instance() {
                                                Some(i) => {
                                                    renderer.status.update_gui(Some(i), None, true);
                                                    i.pokemon.id
                                                },
                                                None => {
                                                    renderer.status.update_gui_view(Some(user), None, true);
                                                    user.pokemon().id
                                                }
                                            }),
                                            None => None,
                                        };
                                        renderer.renderer.new_pokemon(dex, id);
                                    }
                                }
                                true => queue.current = None,
                            },
                            BattleClientGuiCurrent::UseItem(target) => {
                                let target = match target {
                                    MoveTargetLocation::Opponent(i) => &mut other_ui[*i],
                                    MoveTargetLocation::Team(i) => &mut user_ui[*i],
                                    MoveTargetLocation::User => &mut user_ui[instance.pokemon.index],
                                };
                                if !self.gui.text.finished() {
                                    self.gui.text.update(ctx, delta)
                                } else if target.status.health_moving() {
                                    target.status.update_hp(delta);
                                } else {
                                    queue.current = None;
                                }
                            },
                            BattleClientGuiCurrent::Faint => {
                                let ui = &mut user_ui[instance.pokemon.index];
                                if ui.renderer.faint.fainting() {
                                	ui.renderer.faint.update(delta);
                                } else if !self.gui.text.finished() {
                                	self.gui.text.update(ctx, delta);
                                } else {
                                    match instance.pokemon.team == self.local.player.party.id && self.local.player.party.any_inactive() {
                                        true => match self.party.alive() {
                                            true => {
                                                self.party.input(ctx, dex, self.local.player.party.pokemon.as_mut_slice());
                                                self.party.update(delta);
                                                if let Some(selected) = self.party.take_selected() {
                                                    if !self.local.player.party.pokemon[selected].fainted() {
                                                        // user.queue_replace(index, selected);
                                                        self.party.despawn();
                                                        self.messages.send(ClientMessage::ReplaceFaint(instance.pokemon.index, selected));
                                                        self.local.player.party.replace(instance.pokemon.index, Some(selected));
                                                        let pokemon = self.local.player.party.active(instance.pokemon.index);
                                                        ui.status.update_gui(pokemon, None, true);
                                                        ui.renderer.new_pokemon(dex, pokemon.map(|p| p.pokemon.id));
                                                        queue.current = None;
                                                    }
                                                }
                                            },
                                            false => self.party.spawn(dex, &self.local.party.pokemon, Some(false), false),
                                        },
                                        false => {
                                            self.remote.party.replace(instance.pokemon.index, None);
                                            let ui = &mut self.remote.renderer[instance.pokemon.index];
                                            ui.status.update_gui(None, None, true);
                                            ui.renderer.new_pokemon(dex, None);
                                            queue.current = None;
                                        }
                                    }
                                }
                            }
                            BattleClientGuiCurrent::Replace(replaced) => {
                                if self.gui.text.waiting() || self.gui.text.finished() && !*replaced {
                                    let ui = &mut user_ui[instance.pokemon.index];
                                    let id = match user.active_mut(instance.pokemon.index) {
                                        Some(v) => Some(match v.instance() {
                                            Some(i) => {
                                                ui.status.update_gui(Some(i), None, true);
                                                i.pokemon.id
                                            },
                                            None => {
                                                ui.status.update_gui_view(Some(v), None, true);
                                                v.pokemon().id
                                            },
                                        }),
                                        None => None,
                                    };
                                    ui.renderer.new_pokemon(dex, id);
                                    *replaced = true;
                                }
                                match self.gui.text.finished() {
                                    false => self.gui.text.update(ctx, delta),
                                    true => queue.current = None,
                                }
                            }
                            BattleClientGuiCurrent::Catch => match self.gui.text.finished() {
                                false => self.gui.text.update(ctx, delta),
                                true => queue.current = None,
                            }
                            BattleClientGuiCurrent::SetExp => {
                                match !self.gui.text.finished() || self.local.renderer[instance.pokemon.index].status.exp_moving() {
                                    true => {
                                        self.gui.text.update(ctx, delta);
                                        match self.local.player.party.active(instance.pokemon.index) {
                                            Some(pokemon) => self.local.renderer[instance.pokemon.index].status.update_exp(delta, pokemon),
                                            None => {
                                                warn!("Could not get pokemon gaining exp at {}", instance.pokemon);
                                                queue.current = None;
                                            }
                                        }
                                    }
                                    false => queue.current = None,
                                }
                            }
                            BattleClientGuiCurrent::LevelUp => match self.gui.level_up.alive() {
                                true => match self.local.party.pokemon.get_mut(instance.pokemon.index) {
                                    Some(pokemon) => if let Some((index, move_ref)) = self.gui.level_up.update(ctx, &mut self.gui.text, delta, pokemon) {
                                        self.messages.send(ClientMessage::LearnMove(instance.pokemon.index, move_ref.id, index as _));
                                    }
                                    None => {
                                        warn!("Could not get user's active pokemon at {}", instance.pokemon);
                                        queue.current = None;
                                    },
                                },
                                false => queue.current = None,
                            }
                        }
                    },
                }
            }
        }
    }

    pub fn on_begin(&mut self, dex: &PokedexClientContext) {
        self.local.renderer = ActivePokemonRenderer::local(&self.context, dex, &self.local);
        self.remote.renderer = ActivePokemonRenderer::remote(&self.context, dex, &self.remote);
    }

    pub fn draw(&self, ctx: &mut EngineContext, dex: &PokedexClientContext, party: &Party<OwnedRefPokemon<'d>>, bag: &Bag) {
        if !matches!(self.state, BattlePlayerState::WaitToStart) {
            self.gui.background.draw(ctx, 0.0);
            self.remote.renderer.iter().for_each(|active| active.draw(ctx));
            match &self.state {
                BattlePlayerState::WaitToStart => unreachable!(),
                BattlePlayerState::Opening(..) => {
                    self.gui.background.draw(ctx, self.gui.opener.offset::<ID>());
                    self.gui.opener.draw_below_panel::<ID>(ctx, &self.local.renderer, &self.remote.renderer);
                    self.gui.trainer.draw(ctx);
                    self.gui.draw_panel(ctx);
                    self.gui.opener.draw::<ID>(ctx);
                }
                BattlePlayerState::Introduction(..) => {
                    self.gui.background.draw(ctx, 0.0);
                    self.gui.introduction.draw::<ID>(ctx, &self.local.renderer, &self.remote.renderer);
                    self.gui.trainer.draw(ctx);
                    self.gui.draw_panel(ctx);
                    self.gui.text.draw(ctx);
                }
                BattlePlayerState::Select(index) => {
                    if self.party.alive() {
                        self.party.draw(ctx, &party);
                    } else if self.bag.alive() {
                        self.bag.draw(ctx, dex, &bag.items);
                    } else {
                        for (current, active) in self.local.renderer.iter().enumerate() {
                            if &current == index {
                                active.renderer.draw(ctx, Vec2::new(0.0, self.gui.bounce.offset), Color::WHITE);
                                active.status.draw(ctx, 0.0, -self.gui.bounce.offset);
                            } else {
                                active.renderer.draw(ctx, ZERO, Color::WHITE);
                                active.status.draw(ctx, 0.0, 0.0);
                            }
                        }
                        self.gui.draw_panel(ctx);
                        self.gui.panel.draw(ctx);
                    }
                },
                // BattlePlayerState::Faint(..) => if self.party.alive() {
                //     self.party.draw(ctx)
                // },
                BattlePlayerState::WaitToSelect | BattlePlayerState::Moving(..) => {
                    self.local.renderer.iter().for_each(|active| active.draw(ctx));
                    self.gui.draw_panel(ctx);
                    self.gui.text.draw(ctx);
                    self.gui.level_up.draw(ctx);
                    if self.party.alive() {
                        self.party.draw(ctx, party)
                    }
                },
                BattlePlayerState::Winner(..) => {
                    self.local.renderer.iter().for_each(|active| active.draw(ctx));
                    self.gui.draw_panel(ctx);
                    self.gui.text.draw(ctx);
                }
            }
        }
    }
}