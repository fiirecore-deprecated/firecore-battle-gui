pub extern crate firecore_pokedex_engine as pokedex;
pub extern crate firecore_battle as battle;

use std::{collections::VecDeque, rc::Rc, fmt::{Debug, Display}};

use context::BattleGuiContext;
use pokedex::{context::PokedexClientContext, id::Identifiable, item::bag::Bag};

use log::{warn, debug};

use pokedex::{
    gui::{bag::BagGui, party::PartyGui},
    battle_move::BattleMovedex,
    item::ItemUseType, 
    moves::{MoveTarget, Movedex},
    pokemon::PokemonParty,
    id::Dex,
};

use pokedex::engine::{
    graphics::ZERO, 
    tetra::{Context, math::Vec2, graphics::Color}, 
    util::{Entity, Completable, Reset},
    EngineContext,
};

use battle::{
    BattleType, BattleData,
    moves::{client::{ClientAction, ClientMove, ClientActions}, MoveTargetInstance, MoveTargetLocation, BattleMove},
    BattleEndpoint,
    message::{ClientMessage, ServerMessage},
    BoundAction,
    pokemon::PokemonIndex,
};

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
    party::battle_party_gui,
    view::PlayerView,
};

pub mod action;
pub mod view;
pub mod transition;
pub mod ui;
pub mod party;
pub mod context;

use action::*;

use self::transition::TransitionState;

pub struct BattlePlayerGui<ID: Default> {
    context: BattleGuiContext,

    party: Rc<PartyGui>,
    bag: Rc<BagGui>,
	pub gui: BattleGui,

    state: BattlePlayerState<ID>,
    should_select: bool,

    pub battle_data: BattleData,

    pub player: GuiLocalPlayer<ID>,
    pub opponent: GuiRemotePlayer<ID>,

    pub end_party: Option<PokemonParty>,

    messages: Messages<ID>,

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
struct MoveQueue<ID> {
    actions: VecDeque<BoundAction<ID, BattleClientGuiAction<ID>>>,
    current: Option<BoundAction<ID, BattleClientGuiCurrent<ID>>>,
}


#[derive(Debug)]
enum BattlePlayerState<ID> {
    WaitToStart,
    Opening(TransitionState),
    Introduction(TransitionState),
    WaitToSelect,
    Select(usize),
    Moving(MoveQueue<ID>),
    Winner(ID),
}

impl<ID: Default> BattleEndpoint<ID> for BattlePlayerGui<ID> {
    fn send(&mut self, message: ServerMessage<ID>) {
        self.messages.client.insert(0, message)
    }
    fn receive(&mut self) -> Option<ClientMessage> {
        self.messages.server.pop()
    }
}

impl<ID: Sized + Default + Copy + Debug + Display + Eq + Ord> BattlePlayerGui<ID> {

    pub fn new(ctx: &mut Context, party: Rc<PartyGui>, bag: Rc<BagGui>) -> Self {
        let context = BattleGuiContext::new(ctx);
        Self {
            party,
            bag,
			gui: BattleGui::new(ctx, &context),
            state: BattlePlayerState::WaitToStart,
            should_select: false,
            battle_data: Default::default(),
            player: Default::default(),
            opponent: Default::default(),
            end_party: None,
            messages: Default::default(),
            context,
        }
    }

    pub fn winner(&self) -> Option<ID> {
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

    pub fn process(&mut self, dex: &PokedexClientContext) {
        while let Some(message) = self.messages.client.pop() {
            match message {
                ServerMessage::User(data, user) => {
                    self.player.player = user;
                    self.battle_data = data;
                },
                ServerMessage::Opponents(opponent) => {
                    self.opponent.player = opponent;
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
                ServerMessage::PokemonRequest(index, instance) => self.opponent.add_instance(index, instance),
                ServerMessage::FaintReplace(pokemon, new) => {
                    match &mut self.state {
                        BattlePlayerState::Moving(queue) => {
                            queue.actions.push_back(BoundAction {
                                pokemon,
                                action: BattleClientGuiAction::Replace(new),
                            });
                        },
                        _ => {
                            let (player, player_ui) = match pokemon.team == self.player.party.id {
                                true => (&mut self.player.player as &mut dyn PlayerView<ID>, &mut self.player.renderer),
                                false => (&mut self.opponent.player as _, &mut self.opponent.renderer),
                            };
                            player.replace(pokemon.index, new);
                            player_ui[pokemon.index].update(dex, player.active(pokemon.index));
                        }
                    }
                },
                ServerMessage::AddUnknown(index, unknown) => self.opponent.party.add_unknown(index, unknown),
                ServerMessage::Winner(player) => {
                    self.state = BattlePlayerState::Winner(player);
                }
                ServerMessage::PartyRequest(party) => {
                    self.end_party = Some(party);
                }
                ServerMessage::CanFaintReplace(index, can) => {
                    debug!("to - do: checking faint replace");
                    if !can {
                        debug!("cannot replace pokemon at active index {}", index);
                    }
                },
                // ServerMessage::AddMove(pokemon, index, move_ref) => if pokemon.team == self.player.party.id {
                //     if let Some(pokemon) = self.player.party.pokemon.get_mut(pokemon.index) {
                //         debug!("to - do: set move to its index.");
                //         if let Err(err) = pokemon.moves.try_push(MoveInstance::new(move_ref)) {
                //             warn!("Cannot add moves to {} because it has maximum number of moves. error: {}", pokemon.name(), err)
                //         }
                //     }
                // }
            }
        }
    }

    pub fn update(&mut self, ctx: &EngineContext, dex: &PokedexClientContext, delta: f32, bag: &mut Bag) {
        match &mut self.state {
            BattlePlayerState::WaitToStart | BattlePlayerState::Winner(..) => (),
            BattlePlayerState::Opening(state) => match state {
                TransitionState::Begin => {
                    self.gui.opener.begin(dex, state, self.battle_data.type_, &self.opponent);
                    if !matches!(self.battle_data.type_, BattleType::Wild) {
                        self.gui.trainer.spawn(self.player.len(), self.opponent.len());
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
                    self.gui.introduction.begin(dex, state, self.battle_data.type_, &self.player, &self.opponent, &mut self.gui.text);
                    self.update(ctx, dex, delta, bag);
                }
                TransitionState::Run => {
                    self.gui.introduction.update(state, ctx, delta, &mut self.player, &mut self.opponent, &mut self.gui.text);
                    self.gui.trainer.update(delta);
                    if self.gui.text.page() > 0 && !self.gui.trainer.ending() && !matches!(self.battle_data.type_, BattleType::Wild) {
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
                match self.player.party.active.get(*active_index) {
                    Some(index) => match index {
                        Some(index) => {
                            let pokemon = &self.player.party.pokemon[*index];
                            match self.gui.panel.alive() {
                                // true => match self.player.party.active.len() <= *active_index {
                                    true => {
            
                                        // Checks if a move is queued from an action done in the GUI
            
                                        if self.bag.alive() {
                                            self.bag.input(ctx);
                                            if let Some(item) = self.bag.take_selected_despawn() {
                                                match &item.usage {
                                                    ItemUseType::Pokeball => self.gui.panel.active = BattlePanels::Target(MoveTarget::Opponent, Some(item)),
                                                    ItemUseType::Script(..) => todo!("user targeting"),
                                                    ItemUseType::None => todo!("make item unusable"),
                                                }
                                            }
                                        } else if self.party.alive() {
                                            self.party.input(ctx, dex, self.player.party.pokemon.as_mut_slice());
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
                                                        1 => self.bag.spawn(bag),
                                                        2 => battle_party_gui(dex, &self.party, &self.player.party.pokemon, true),
                                                        3 => if matches!(self.battle_data.type_, BattleType::Wild) {
                                                            self.messages.send(ClientMessage::Forfeit);
                                                        },
                                                        _ => unreachable!(),
                                                    }
                                                }
                                                BattlePanels::Fight => match pokemon.moves.get(self.gui.panel.fight.moves.cursor) {
                                                    Some(instance) => match instance.get() {
                                                        Some(move_ref) => {
                                                            match move_ref.target {
                                                                MoveTarget::Opponent | MoveTarget::Any => {
                                                                    self.gui.panel.target(&self.opponent.player);
                                                                    self.gui.panel.active = BattlePanels::Target(move_ref.target, None);
                                                                },
                                                                MoveTarget::Ally | MoveTarget::UserOrAlly => {
                                                                    self.gui.panel.target(&self.player.player);
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
                                                                                    MoveTarget::Todo => MoveTargetInstance::Todo,
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

                                let (user, user_ui, other, other_ui) = if instance.pokemon.team == self.player.party.id {
                                    (&mut self.player.player as &mut dyn PlayerView<ID>, &mut self.player.renderer, &mut self.opponent.player as &mut dyn PlayerView<ID>, &mut self.opponent.renderer)
                                } else {
                                    (&mut self.opponent.player as _, &mut self.opponent.renderer, &mut self.player.player as _, &mut self.player.renderer)
                                };

                                self.gui.text.clear();
                                self.gui.text.reset();

                                if user.active(instance.pokemon.index).is_some() || !instance.action.requires_user() {

                                    if let Some(action) = match instance.action {
                                        BattleClientGuiAction::Action(action) => match action {
                                            ClientMove::Move(pokemon_move, targets) => {
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

                                                        if let Some(battle_move) = BattleMovedex::try_get(pokemon_move.id()) {
                                                            user_pokemon_ui.renderer.moves.init(battle_move.script());
                                                        } 

                                                        for moves in actions {
                                                            match moves {
                                                                ClientAction::UserHP(damage) => user_pokemon.set_hp(*damage),
                                                                ClientAction::Fail => ui::text::on_fail(&mut self.gui.text, vec![format!("{} cannot use move", user_pokemon.name()), format!("{} is unimplemented", pokemon_move.name)]),
                                                                ClientAction::Miss => ui::text::on_miss(&mut self.gui.text, user_pokemon.name()),
                                                                ClientAction::SetExp(experience, level) => {
                                                                    let previous = user_pokemon.level();
                                                                    user_pokemon.set_level(*level);
                                                                    if let Some(user_pokemon) = user_pokemon.instance_mut() {
                                                                        user_pokemon.experience = *experience;
                                                                        user_pokemon.level = *level;
                                                                        let moves = user_pokemon.on_level_up(previous).flat_map(|ref id| Movedex::try_get(id)).collect();
                                                                        queue.actions.push_front(BoundAction { pokemon: instance.pokemon, action: BattleClientGuiAction::SetExp(previous, *experience, moves) });
                                                                    }
                                                                }
                                                                _ => (),
                                                            }
                                                        }
        
                                                        user_pokemon_ui.update_status(Some(user_pokemon), false);
    
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
                                                                ClientAction::StatStage(stat) => ui::text::on_stat_stage(&mut self.gui.text, target.name(), stat),
                                                                ClientAction::Faint(target_instance) => queue.actions.push_front(
                                                                    BoundAction {
                                                                        pokemon: *target_instance,
                                                                        action: BattleClientGuiAction::Faint,
                                                                    }
                                                                ),
                                                                ClientAction::Status(effect) => {
                                                                    target.set_effect(*effect);
                                                                    ui::text::on_status(&mut self.gui.text, target.name(), &effect.status);
                                                                }
                                                                ClientAction::Miss | ClientAction::UserHP(..) | ClientAction::SetExp(..) | ClientAction::Fail => (),
                                                            }
                                                        }
                                                        target_ui.update_status(Some(target), false);
                                                    } else {
                                                        target_ui.update_status(None, false);
                                                    }
                                                }
                                                Some(BattleClientGuiCurrent::Move(targets))
                                            }
                                            ClientMove::UseItem(item, index) => {
                                                if let Some((id, target)) = match &item.usage {
                                                    ItemUseType::Script(..) => user.active(index).map(|v| (user.id(), v)),
                                                    ItemUseType::Pokeball => other.active(index).map(|v| (other.id(), v)),
                                                    ItemUseType::None => None,
                                                } {
                                                    if let ItemUseType::Pokeball = &item.usage {
                                                        // self.messages.push(ClientMessage::RequestPokemon(index));
                                                        queue.actions.push_front(BoundAction {
                                                            pokemon: PokemonIndex { team: *id, index },
                                                            action: BattleClientGuiAction::Catch,
                                                        });
                                                    }
                                                    ui::text::on_item(&mut self.gui.text, target.name(), &item);
                                                }
                                                Some(BattleClientGuiCurrent::UseItem(match &item.usage {
                                                    ItemUseType::Script(..) | ItemUseType::None => match index == instance.pokemon.index {
                                                        true => MoveTargetLocation::User,
                                                        false => MoveTargetLocation::Team(index),
                                                    },
                                                    ItemUseType::Pokeball => MoveTargetLocation::Opponent(index),
                                                }))
                                            }
                                            ClientMove::Switch(index) => {
                                                let coming = user.pokemon(index).unwrap().name();
                                                ui::text::on_switch(&mut self.gui.text, user.active(instance.pokemon.index).unwrap().name(), coming);
                                                Some(BattleClientGuiCurrent::Switch(index))
                                            }
                                        }
                                        BattleClientGuiAction::Faint => {
                                            let is_player = &instance.pokemon.team == user.id();
                                            let target = user.active_mut(instance.pokemon.index).unwrap();
                                            target.set_hp(0.0);
                                            ui::text::on_faint(&mut self.gui.text, matches!(self.battle_data.type_, BattleType::Wild), is_player, target.name());
                                            user_ui[instance.pokemon.index].renderer.faint();
                                            Some(BattleClientGuiCurrent::Faint)
                                        },
                                        BattleClientGuiAction::Catch => {
                                            let pokemon = self.opponent.pokemon(instance.pokemon.index).map(|v| v.instance()).flatten();
                                            ui::text::on_catch(&mut self.gui.text, pokemon);
                                            // if let Some(pokemon) = pokemon {
                                            self.opponent.replace(instance.pokemon.index, None);
                                            self.opponent.renderer[instance.pokemon.index].update(dex, None);
                                            // }
                                            Some(BattleClientGuiCurrent::Catch)
                                        }
                                        BattleClientGuiAction::Replace(new) => {
                                            ui::text::on_replace(&mut self.gui.text, user.name(), new.map(|index| user.pokemon(index).map(|v| v.name())).flatten());
                                            user.replace(instance.pokemon.index, new);
                                            Some(BattleClientGuiCurrent::Replace(false))
                                        }
                                        // To - do: experience spreading
                                        BattleClientGuiAction::SetExp(previous, experience, moves) => match user.active(instance.pokemon.index).map(|v| v.instance()).flatten() {
                                            Some(pokemon) => {    
                                                ui::text::on_gain_exp(&mut self.gui.text, pokemon, experience, pokemon.level);
                                                user_ui[instance.pokemon.index].status.update_gui_ex(Some((previous, pokemon)), false);
                                                queue.actions.push_front(BoundAction { pokemon: instance.pokemon, action: BattleClientGuiAction::LevelUp(moves) });
                                                Some(BattleClientGuiCurrent::SetExp)
                                            }
                                            None => None,
                                        }
                                        BattleClientGuiAction::LevelUp(moves) => match user.active(instance.pokemon.index).map(|v| v.instance()).flatten() {
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

                        let (user, user_ui, other_ui) = if instance.pokemon.team == self.player.party.id {
                            (&mut self.player.player as &mut dyn PlayerView<ID>, &mut self.player.renderer, &mut self.opponent.renderer)
                        } else {
                            (&mut self.opponent.player as _, &mut self.opponent.renderer, &mut self.player.renderer)
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
                                        user_ui[instance.pokemon.index].update(dex, user.active(instance.pokemon.index));
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
                                    match instance.pokemon.team == self.player.player.party.id && self.player.player.party.any_inactive() {
                                        true => match self.party.alive() {
                                            true => {
                                                self.party.input(ctx, dex, self.player.player.party.pokemon.as_mut_slice());
                                                self.party.update(delta);
                                                if let Some(selected) = self.party.take_selected() {
                                                    if !self.player.player.party.pokemon[selected].fainted() {
                                                        // user.queue_replace(index, selected);
                                                        self.party.despawn();
                                                        self.messages.send(ClientMessage::FaintReplace(instance.pokemon.index, selected));
                                                        self.player.player.party.replace(instance.pokemon.index, Some(selected));
                                                        ui.update(dex, self.player.player.party.active(instance.pokemon.index).map(|p| p as _));
                                                        queue.current = None;
                                                    }
                                                }
                                            },
                                            false => battle_party_gui(dex, &self.party, &self.player.party.pokemon, false)
                                        },
                                        false => {
                                            let user = match instance.pokemon.team == self.player.player.party.id {
                                                true => &mut self.player.player as &mut dyn PlayerView<ID>,
                                                false => &mut self.opponent.player as _,
                                            };
                                            user.replace(instance.pokemon.index, None);
                                            user_ui[instance.pokemon.index].update(dex, None);
                                            queue.current = None;
                                        }
                                    }
                                }
                            }
                            BattleClientGuiCurrent::Replace(replaced) => {
                                if self.gui.text.waiting() || self.gui.text.finished() && !*replaced {
                                    user_ui[instance.pokemon.index].update(dex, user.active(instance.pokemon.index));
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
                                match !self.gui.text.finished() || self.player.renderer[instance.pokemon.index].status.exp_moving() {
                                    true => {
                                        self.gui.text.update(ctx, delta);
                                        match self.player.player.party.active(instance.pokemon.index) {
                                            Some(pokemon) => self.player.renderer[instance.pokemon.index].status.update_exp(delta, pokemon),
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
                                true => match self.player.party.pokemon.get_mut(instance.pokemon.index) {
                                    Some(pokemon) => if let Some((index, move_ref)) = self.gui.level_up.update(ctx, &mut self.gui.text, delta, pokemon) {
                                        self.messages.send(ClientMessage::AddLearnedMove(instance.pokemon.index, index, move_ref.id));
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
        self.player.renderer = ActivePokemonRenderer::local(&self.context, dex, &self.player);
        self.opponent.renderer = ActivePokemonRenderer::remote(&self.context, dex, &self.opponent);
    }

    pub fn draw(&self, ctx: &mut EngineContext, dex: &PokedexClientContext) {
        if !matches!(self.state, BattlePlayerState::WaitToStart) {
            self.gui.background.draw(ctx, 0.0);
            self.opponent.renderer.iter().for_each(|active| active.draw(ctx));
            match &self.state {
                BattlePlayerState::WaitToStart => unreachable!(),
                BattlePlayerState::Opening(..) => {
                    self.gui.background.draw(ctx, self.gui.opener.offset::<ID>());
                    self.gui.opener.draw_below_panel::<ID>(ctx, &self.player.renderer, &self.opponent.renderer);
                    self.gui.trainer.draw(ctx);
                    self.gui.draw_panel(ctx);
                    self.gui.opener.draw::<ID>(ctx);
                }
                BattlePlayerState::Introduction(..) => {
                    self.gui.background.draw(ctx, 0.0);
                    self.gui.introduction.draw::<ID>(ctx, &self.player.renderer, &self.opponent.renderer);
                    self.gui.trainer.draw(ctx);
                    self.gui.draw_panel(ctx);
                    self.gui.text.draw(ctx);
                }
                BattlePlayerState::Select(index) => {
                    if self.party.alive() {
                        self.party.draw(ctx);
                    } else if self.bag.alive() {
                        self.bag.draw(ctx, dex);
                    } else {
                        for (current, active) in self.player.renderer.iter().enumerate() {
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
                    self.player.renderer.iter().for_each(|active| active.draw(ctx));
                    self.gui.draw_panel(ctx);
                    self.gui.text.draw(ctx);
                    self.gui.level_up.draw(ctx);
                    if self.party.alive() {
                        self.party.draw(ctx)
                    }
                },
                BattlePlayerState::Winner(..) => {
                    self.player.renderer.iter().for_each(|active| active.draw(ctx));
                    self.gui.draw_panel(ctx);
                    self.gui.text.draw(ctx);
                }
            }
        }
    }
}