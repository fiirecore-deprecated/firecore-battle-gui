use pokedex::{battle::party::knowable::{BattlePartyKnown, BattlePartyUnknown}, context::PokedexClientContext, engine::{
        gui::MessageBox,
        util::{Entity, Completable},
        EngineContext,
    }};

use crate::{battle::data::BattleType, context::BattleGuiContext};

use crate::ui::view::{ActiveRenderer, ActivePokemonParty};

use super::TransitionState;

mod basic;
mod trainer;

pub use basic::*;
pub use trainer::*;

pub enum Introductions {

    Basic,
    Trainer,

}

impl Default for Introductions {
    fn default() -> Self {
        Self::Basic
    }
}

pub(crate) trait BattleIntroduction<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord>: Completable {

    fn spawn(&mut self, ctx: &PokedexClientContext, battle_type: BattleType, player: &BattlePartyKnown<ID>, opponent: &BattlePartyUnknown<ID>, text: &mut MessageBox);

    fn update(&mut self, ctx: &EngineContext, delta: f32, player: &mut ActivePokemonParty<BattlePartyKnown<ID>>, opponent: &mut ActivePokemonParty<BattlePartyUnknown<ID>>, text: &mut MessageBox);

    fn draw(&self, ctx: &mut EngineContext, player: &ActiveRenderer, opponent: &ActiveRenderer);

}

pub struct BattleIntroductionManager {
    current: Introductions,

    basic: BasicBattleIntroduction,
    trainer: TrainerBattleIntroduction,
}

impl BattleIntroductionManager {

    pub fn new(ctx: &BattleGuiContext) -> Self {
        Self {
            current: Introductions::default(),

            basic: BasicBattleIntroduction::new(ctx),
            trainer: TrainerBattleIntroduction::new(ctx),
        }
    }

    pub fn begin<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord>(&mut self, ctx: &PokedexClientContext, state: &mut TransitionState, battle_type: BattleType, player: &BattlePartyKnown<ID>, opponent: &BattlePartyUnknown<ID>, text: &mut MessageBox) {
        *state = TransitionState::Run;
        match battle_type {
            BattleType::Wild => self.current = Introductions::Basic,
            _ => self.current = Introductions::Trainer,
        }
        let current = self.get_mut();
        current.reset();
        current.spawn(ctx, battle_type, player, opponent, text);
        text.spawn();
    }

    pub fn end(&mut self, text: &mut MessageBox) {
        text.clear();
    }

    pub fn update<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord>(&mut self, state: &mut TransitionState, ctx: &EngineContext, delta: f32, player: &mut ActivePokemonParty<BattlePartyKnown<ID>>, opponent: &mut ActivePokemonParty<BattlePartyUnknown<ID>>, text: &mut MessageBox) {
        let current = self.get_mut();
        current.update(ctx, delta, player, opponent, text);
        if current.finished() {
            *state = TransitionState::End;
        }
    }

    pub fn draw<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord>(&self, ctx: &mut EngineContext, player: &ActiveRenderer, opponent: &ActiveRenderer) {
        self.get::<ID>().draw(ctx, player, opponent);
    }

    fn get<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord>(&self) -> &dyn BattleIntroduction<ID> {
        match self.current {
            Introductions::Basic => &self.basic,
            Introductions::Trainer => &self.trainer,
        }
    }

    fn get_mut<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord>(&mut self) -> &mut dyn BattleIntroduction<ID> {
        match self.current {
            Introductions::Basic => &mut self.basic,
            Introductions::Trainer => &mut self.trainer,
        }
    }

}