pub extern crate firecore_pokedex_client as pokedex;
pub extern crate firecore_battle as battle;

use std::{collections::VecDeque, rc::Rc};

use pokedex::{Identifiable, BorrowableMut};

use log::warn;

use pokedex::{
    gui::{bag::BagGui, party::PartyGui},
    battle::{
        Active,
        PokemonIndex,
        ActionInstance,
        BattleMove,
        party::knowable::{BattlePartyKnown, BattlePartyUnknown},
    }, 
    battle2::BattleMove as BMove, 
    item::{ItemUseType, bag::Bag}, 
    moves::target::{MoveTarget, MoveTargetInstance, MoveTargetLocation}, 
    pokemon::party::PokemonParty
};

use pokedex::engine::{
    graphics::ZERO, 
    tetra::{Context, math::Vec2, graphics::Color}, 
    util::{Entity, Completable, Reset}
};

use battle::{
    data::{BattleType, BattleData},
    client::action::{
        BattleClientMove,
        BattleClientAction,
    },
    client::{BattleEndpoint, BattleClient},
    message::{ClientMessage, ServerMessage},
};

use self::{
    ui::{
        BattleGui,
        panels::BattlePanels,
        view::{
            ActivePokemonParty,
            ActivePokemonRenderer,
        },
    },
    party::battle_party_gui,
    view::BattlePartyView,
};

pub mod action;
pub mod view;

pub mod transition;

pub mod ui;

pub mod party;

use action::*;

use self::transition::TransitionState;

pub struct BattlePlayerGui<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord> {

    party: Rc<PartyGui>,
    bag: Rc<BagGui>,
	pub gui: BattleGui,

    state: BattlePlayerState<ID>,
    should_select: bool,

    pub battle_data: BattleData,

    pub player: ActivePokemonParty<BattlePartyKnown<ID>>,
    pub opponent: ActivePokemonParty<BattlePartyUnknown<ID>>,

    player_bag: BorrowableMut<'static, Bag>,

    pub end_party: Option<Box<PokemonParty>>,

    messages: Vec<ClientMessage>,

}

#[derive(Debug)]
struct MoveQueue<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord> {
    actions: VecDeque<ActionInstance<ID, BattleClientGuiAction<ID>>>,
    current: Option<ActionInstance<ID, BattleClientGuiCurrent<ID>>>,
}


#[derive(Debug)]
enum BattlePlayerState<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord> {
    WaitToStart,
    Opening(TransitionState),
    Introduction(TransitionState),
    WaitToSelect,
    Select(Active),
    Moving(MoveQueue<ID>),
    Winner(ID),
}

impl<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord> BattleEndpoint<ID> for BattlePlayerGui<ID> {
    fn give_client(&mut self, message: ServerMessage<ID>) {
        match message {
            ServerMessage::User(data, user) => {
                self.player.party = user;
                self.battle_data = data;
            },
            ServerMessage::Opponents(opponent) => {
                self.opponent.party = opponent;
            },
            ServerMessage::StartSelecting => {
                self.should_select = true;
                self.gui.panel.despawn();
            },
            ServerMessage::TurnQueue(queue) => {
                self.state = BattlePlayerState::Moving(MoveQueue {
                    actions: queue.into_iter().map(|a| ActionInstance {
                        pokemon: a.pokemon,
                        action: BattleClientGuiAction::Action(a.action),
                    }).collect(),
                    current: None,
                });
                self.gui.text.clear();
                self.gui.text.spawn();
            },
            ServerMessage::PokemonRequest(index, instance) => self.opponent.party.add_instance(index, instance),
            ServerMessage::FaintReplace(pokemon, new) => {
                match &mut self.state {
                    BattlePlayerState::Moving(queue) => {
                        queue.actions.push_back(ActionInstance {
                            pokemon,
                            action: BattleClientGuiAction::Replace(new),
                        });
                    },
                    _ => {
                        let (player, player_ui) = match pokemon.team == self.player.party.id {
                            true => (&mut self.player.party as &mut dyn BattlePartyView<ID>, &mut self.player.renderer),
                            false => (&mut self.opponent.party as _, &mut self.opponent.renderer),
                        };
                        player.replace(pokemon.index, new);
                        player_ui[pokemon.index].update(self.opponent.party.active(pokemon.index))
                    }
                }
            },
            ServerMessage::AddUnknown(index, unknown) => self.opponent.party.add_unknown(index, unknown),
            ServerMessage::Winner(player, party) => {
                self.state = BattlePlayerState::Winner(player);
                self.end_party = party;
            }
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

impl<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord> BattleClient<ID> for BattlePlayerGui<ID> {

    fn give_server(&mut self) -> Option<ClientMessage> {
        self.messages.pop()
    }
}

impl<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord> BattlePlayerGui<ID> {

    pub fn new(ctx: &mut Context, party: Rc<PartyGui>, bag: Rc<BagGui>, player_bag: BorrowableMut<'static, Bag>, id_default: ID) -> Self {
        Self {
            party,
            bag,
			gui: BattleGui::new(ctx),
            state: BattlePlayerState::WaitToStart,
            should_select: false,
            battle_data: Default::default(),
            player: ActivePokemonParty {
                party: BattlePartyKnown::default_with_id(id_default),
                renderer: Default::default(),
            },
            opponent: ActivePokemonParty {
                party: BattlePartyUnknown::default_with_id(id_default),
                renderer: Default::default(),
            },
            player_bag,
            end_party: None,
            messages: Default::default(),
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

    pub fn update(&mut self, ctx: &Context, delta: f32) {

        match &mut self.state {
            BattlePlayerState::WaitToStart | BattlePlayerState::Winner(..) => (),
            BattlePlayerState::Opening(state) => match state {
                TransitionState::Begin => {
                    self.gui.opener.begin(state, self.battle_data.type_, self.opponent.party.trainer.as_ref());
                    if !matches!(self.battle_data.type_, BattleType::Wild) {
                        self.gui.trainer.spawn(self.player.party.len(), self.opponent.party.len());
                    }
                    self.update(ctx, delta);
                }
                TransitionState::Run => self.gui.opener.update(state, delta),
                TransitionState::End => {
                    self.state = BattlePlayerState::Introduction(TransitionState::default());
                    self.update(ctx, delta);
                }
            }
            BattlePlayerState::Introduction(state) => match state {
                TransitionState::Begin => {
                    self.gui.introduction.begin(state, self.battle_data.type_, &self.player.party, &self.opponent.party, &mut self.gui.text);
                    self.update(ctx, delta);
                }
                TransitionState::Run => {
                    self.gui.introduction.update(state, ctx, delta, &mut self.player, &mut self.opponent, &mut self.gui.text);
                    self.gui.trainer.update(delta);
                    if self.gui.text.current() > 0 && !self.gui.trainer.ending() && !matches!(self.battle_data.type_, BattleType::Wild) {
                        self.gui.trainer.end();
                    }
                }
                TransitionState::End => {
                    self.gui.introduction.end(&mut self.gui.text);
                    self.gui.trainer.despawn();
                    self.state = BattlePlayerState::WaitToSelect;
                    self.update(ctx, delta);
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
                                            self.party.input(ctx, self.player.party.pokemon.as_mut_slice());
                                            self.party.update(delta);
                                            if let Some(selected) = self.party.take_selected() {
                                                self.party.despawn();
                                                self.messages.push(
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
                                                        1 => self.bag.spawn(&mut self.player_bag),
                                                        2 => battle_party_gui(&self.party, &self.player.party, true),
                                                        3 => if matches!(self.battle_data.type_, BattleType::Wild) {
                                                            self.messages.push(ClientMessage::Forfeit);
                                                        },
                                                        _ => unreachable!(),
                                                    }
                                                }
                                                BattlePanels::Fight => match pokemon.moves.get(self.gui.panel.fight.moves.cursor) {
                                                    Some(instance) => match instance.get() {
                                                        Some(move_ref) => {
                                                            match move_ref.target {
                                                                MoveTarget::Opponent | MoveTarget::Any => {
                                                                    self.gui.panel.target(&self.opponent.party);
                                                                    self.gui.panel.active = BattlePanels::Target(move_ref.target, None);
                                                                },
                                                                MoveTarget::Ally | MoveTarget::UserOrAlly => {
                                                                    self.gui.panel.target(&self.player.party);
                                                                    self.gui.panel.active = BattlePanels::Target(move_ref.target, None);
                                                                }
                                                                _ => {
                                                                    self.messages.push(
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
                                                    self.messages.push(
                                                        ClientMessage::Move(
                                                            *active_index,
                                                            match item {
                                                                Some(item) => BattleMove::UseItem(
                                                                    item,
                                                                    match target {
                                                                        MoveTarget::Opponent => MoveTargetLocation::Opponent(self.gui.panel.targets.cursor),
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
                                self.messages.push(ClientMessage::FinishedTurnQueue);
                                self.state = BattlePlayerState::WaitToSelect;
                            }
                            Some(instance) => {

                                // to - do: better client checking

                                let (user, user_ui, other, other_ui) = if instance.pokemon.team == self.player.party.id {
                                    (&mut self.player.party as &mut dyn BattlePartyView<ID>, &mut self.player.renderer, &mut self.opponent.party as &mut dyn BattlePartyView<ID>, &mut self.opponent.renderer)
                                } else {
                                    (&mut self.opponent.party as _, &mut self.opponent.renderer, &mut self.player.party as _, &mut self.player.renderer)
                                };

                                self.gui.text.clear();
                                self.gui.text.reset();

                                if user.active(instance.pokemon.index).is_some() || !instance.action.requires_user() {

                                    if let Some(action) = match instance.action {
                                        BattleClientGuiAction::Action(action) => match action {
                                            BattleClientAction::Move(pokemon_move, targets) => {
                                                match user.active(instance.pokemon.index) {
                                                    Some(user_active) => {

                                                        // if targets.iter().any(|(t, _)| match &t {
                                                        //     MoveTargetLocation::Opponent(index) => other.active(*index),
                                                        //     MoveTargetLocation::Team(index) => user.active(*index),
                                                        //     MoveTargetLocation::User => user.active(instance.pokemon.index),
                                                        // }.map(|v| !v.fainted()).unwrap_or_default()) {

                                                        ui::text::on_move(&mut self.gui.text, &pokemon_move, user_active);

                                                        // }
            
                                                        for (target, moves) in &targets {
            
                                                            {
            
                                                                let user_pokemon = user.active_mut(instance.pokemon.index).unwrap();
            
                                                                let user_pokemon_ui = &mut user_ui[instance.pokemon.index];

                                                                if let Some(battle_move) = BMove::try_get(pokemon_move.id()) {
                                                                    user_pokemon_ui.renderer.moves.init(battle_move.script());
                                                                } 

                                                                for moves in moves {
                                                                    match moves {
                                                                        BattleClientMove::UserHP(damage) => user_pokemon.set_hp(*damage),
                                                                        BattleClientMove::Fail => ui::text::on_fail(&mut self.gui.text, vec![format!("{} cannot use move", user_pokemon.name()), format!("{} is unimplemented", pokemon_move.name)]),
                                                                        BattleClientMove::Miss => ui::text::on_miss(&mut self.gui.text, user_pokemon),
                                                                        BattleClientMove::SetExp(experience, level) => {
                                                                            let previous = user_pokemon.level();
                                                                            user_pokemon.set_level(*level);
                                                                            if let Some(user_pokemon) = user_pokemon.instance_mut() {
                                                                                user_pokemon.experience = *experience;
                                                                                user_pokemon.level = *level;
                                                                                let moves = user_pokemon.on_level_up(previous);
                                                                                queue.actions.push_front(ActionInstance { pokemon: instance.pokemon, action: BattleClientGuiAction::SetExp(previous, *experience, moves.collect()) });
                                                                            }
                                                                        }
                                                                        _ => (),
                                                                    }
                                                                }
                
                                                                user_pokemon_ui.update_status(Some(user_pokemon), false);
            
                                                            }
            
                                                            let (target, target_ui) = match target {
                                                                MoveTargetLocation::Opponent(index) => (other.active_mut(*index), &mut other_ui[*index]),
                                                                MoveTargetLocation::Team(index) => (user.active_mut(*index), &mut user_ui[*index]),
                                                                MoveTargetLocation::User => (user.active_mut(instance.pokemon.index), &mut user_ui[instance.pokemon.index]),
                                                            };
            
                                                            if let Some(target) = target {
                                                                for moves in moves {
                                                                    match moves {
                                                                        BattleClientMove::TargetHP(damage, crit) => {
                                                                            target.set_hp(*damage);
                                                                            if damage >= &0.0 {
                                                                                target_ui.renderer.flicker()
                                                                            }
                                                                            if *crit {
                                                                                ui::text::on_crit(&mut self.gui.text);
                                                                            }
                                                                        },
                                                                        BattleClientMove::Effective(effective) => ui::text::on_effective(&mut self.gui.text, &effective),
                                                                        BattleClientMove::StatStage(stat) => ui::text::on_stat_stage(&mut self.gui.text, target, stat),
                                                                        BattleClientMove::Faint(target_instance) => queue.actions.push_front(
                                                                            ActionInstance {
                                                                                pokemon: *target_instance,
                                                                                action: BattleClientGuiAction::Faint,
                                                                            }
                                                                        ),
                                                                        BattleClientMove::Status(effect) => {
                                                                            target.set_effect(*effect);
                                                                            ui::text::on_status(&mut self.gui.text, target, &effect.status);
                                                                        }
                                                                        BattleClientMove::Miss | BattleClientMove::UserHP(..) | BattleClientMove::SetExp(..) | BattleClientMove::Fail => (),
                                                                    }
                                                                }
                                                                target_ui.update_status(Some(target), false);
                                                            } else {
                                                                target_ui.update_status(None, false);
                                                            }
                                                        }
                                                        Some(BattleClientGuiCurrent::Move(targets))
                                                    }
                                                    None => None,
                                                }
                                            }
                                            BattleClientAction::UseItem(item, target) => {
                                                let (index, player) = match target {
                                                    MoveTargetLocation::Opponent(i) => (other.index(i), other),
                                                    MoveTargetLocation::Team(i) => (user.index(i), user),
                                                    MoveTargetLocation::User => (user.index(instance.pokemon.index), user),
                                                };
                                                if let Some(index) = index {
                                                    if let ItemUseType::Pokeball = item.usage {
                                                        // self.messages.push(ClientMessage::RequestPokemon(index));
                                                        queue.actions.push_front(ActionInstance {
                                                            pokemon: PokemonIndex { team: *player.id(), index },
                                                            action: BattleClientGuiAction::Catch,
                                                        });
                                                    }
                                                    ui::text::on_item(&mut self.gui.text, player.pokemon(index), &item);
                                                }
                                                Some(BattleClientGuiCurrent::UseItem(target))
                                            }
                                            BattleClientAction::Switch(index) => {
                                                let coming = user.pokemon(index).unwrap();
                                                ui::text::on_switch(&mut self.gui.text, user.active(instance.pokemon.index).unwrap(), coming);
                                                Some(BattleClientGuiCurrent::Switch(index))
                                            }
                                        }
                                        BattleClientGuiAction::Faint => {
                                            let is_player = &instance.pokemon.team == user.id();
                                            let target = user.active_mut(instance.pokemon.index).unwrap();
                                            target.set_hp(0.0);
                                            ui::text::on_faint(&mut self.gui.text, matches!(self.battle_data.type_, BattleType::Wild), is_player, target);
                                            user_ui[instance.pokemon.index].renderer.faint();
                                            Some(BattleClientGuiCurrent::Faint)
                                        },
                                        BattleClientGuiAction::Catch => {
                                            let pokemon = self.opponent.party.pokemon(instance.pokemon.index).map(|v| v.instance()).flatten();
                                            ui::text::on_catch(&mut self.gui.text, pokemon);
                                            // if let Some(pokemon) = pokemon {
                                            self.opponent.party.replace(instance.pokemon.index, None);
                                            self.opponent.renderer[instance.pokemon.index].update(None);
                                            // }
                                            Some(BattleClientGuiCurrent::Catch)
                                        }
                                        BattleClientGuiAction::Replace(new) => {
                                            ui::text::on_replace(&mut self.gui.text, user.name(), new.map(|index| user.pokemon(index)).flatten());
                                            user.replace(instance.pokemon.index, new);
                                            Some(BattleClientGuiCurrent::Replace(false))
                                        }
                                        // To - do: experience spreading
                                        BattleClientGuiAction::SetExp(previous, experience, moves) => match user.active(instance.pokemon.index).map(|v| v.instance()).flatten() {
                                            Some(pokemon) => {    
                                                ui::text::on_gain_exp(&mut self.gui.text, pokemon, experience, pokemon.level);
                                                user_ui[instance.pokemon.index].status.update_gui_ex(Some((previous, pokemon)), false);
                                                queue.actions.push_front(ActionInstance { pokemon: instance.pokemon, action: BattleClientGuiAction::LevelUp(moves) });
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
                                        // BattleClientAction::Catch(index) => {
                                        //     if let Some(target) = match index.team {
                                        //         Team::Player => &user.active[index.active],
                                        //         Team::Opponent => &other.active[index.active],
                                        //     }.pokemon.as_ref() {
                                        //         ui::text::on_catch(text, target);
                                        //     }
                                        // }
                                    } {
                                        queue.current = Some(ActionInstance {
                                            pokemon: instance.pokemon,
                                            action
                                        });
                                    } else {
                                        self.update(ctx, delta);
                                    }
                                }                                
                            },
                        }
                    },
                    Some(instance) => {

                        let (user, user_ui, other_ui) = if instance.pokemon.team == self.player.party.id {
                            (&mut self.player.party as &mut dyn BattlePartyView<ID>, &mut self.player.renderer, &mut self.opponent.renderer)
                        } else {
                            (&mut self.opponent.party as _, &mut self.opponent.renderer, &mut self.player.renderer)
                        };
                        

                        match &mut instance.action {
                            BattleClientGuiCurrent::Move(targets) => {

                                user_ui[instance.pokemon.index].renderer.moves.update(delta);

                                match self.gui.text.finished() {
                                    false => self.gui.text.update(ctx, delta),
                                    true => if (self.gui.text.current > 0 || self.gui.text.can_continue) && user_ui[instance.pokemon.index].renderer.moves.finished() {
                                        let index = instance.pokemon.index;
                                        targets.retain(|(t, _)| {
                                            let ui = match *t {
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
                                                let ui = match target.0 {
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

                                    if self.gui.text.current() == 1 && !user.active_eq(instance.pokemon.index, Some(*new)) {
                                        user.replace(instance.pokemon.index, Some(*new));
                                        user_ui[instance.pokemon.index].update(user.active(instance.pokemon.index));
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
                                    match instance.pokemon.team == self.player.party.id && self.player.party.any_inactive() {
                                        true => match self.party.alive() {
                                            true => {
                                                self.party.input(ctx, self.player.party.pokemon.as_mut_slice());
                                                self.party.update(delta);
                                                if let Some(selected) = self.party.take_selected() {
                                                    if !self.player.party.pokemon[selected].fainted() {
                                                        // user.queue_replace(index, selected);
                                                        self.party.despawn();
                                                        self.messages.push(ClientMessage::FaintReplace(instance.pokemon.index, selected));
                                                        self.player.party.replace(instance.pokemon.index, Some(selected));
                                                        ui.update(self.player.party.active(instance.pokemon.index));
                                                        queue.current = None;
                                                    }
                                                }
                                            },
                                            false => battle_party_gui(&self.party, &self.player.party, false)
                                        },
                                        false => {
                                            let user = match instance.pokemon.team == self.player.party.id {
                                                true => &mut self.player.party as &mut dyn BattlePartyView<ID>,
                                                false => &mut self.opponent.party as _,
                                            };
                                            user.replace(instance.pokemon.index, None);
                                            user_ui[instance.pokemon.index].update(None);
                                            queue.current = None;
                                        }
                                    }
                                }
                            }
                            BattleClientGuiCurrent::Replace(replaced) => {
                                if self.gui.text.can_continue() || self.gui.text.finished() && !*replaced {
                                    user_ui[instance.pokemon.index].update(user.active(instance.pokemon.index));
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
                                        match self.player.party.active(instance.pokemon.index).map(|v| v.instance()).flatten() {
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
                                        self.messages.push(ClientMessage::AddLearnedMove(instance.pokemon.index, index, move_ref));
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

    pub fn on_begin(&mut self, ctx: &mut Context) {
        self.player.renderer = ActivePokemonRenderer::init_known(ctx, &self.player.party);
        self.opponent.renderer = ActivePokemonRenderer::init_unknown(ctx, &self.opponent.party);
    }

    pub fn draw(&self, ctx: &mut Context) {
        if !matches!(self.state, BattlePlayerState::WaitToStart) {
            self.gui.background.draw(ctx, 0.0);
            self.opponent.renderer.iter().for_each(|active| active.draw(ctx));
            match &self.state {
                BattlePlayerState::WaitToStart => unreachable!(),
                BattlePlayerState::Opening(..) => {
                    self.gui.background.draw(ctx, self.gui.opener.offset());
                    self.gui.opener.draw_below_panel(ctx, &self.player.renderer, &self.opponent.renderer);
                    self.gui.trainer.draw(ctx);
                    self.gui.draw_panel(ctx);
                    self.gui.opener.draw(ctx);
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
                        self.bag.draw(ctx);
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