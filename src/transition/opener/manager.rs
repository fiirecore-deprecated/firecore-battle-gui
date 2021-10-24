use pokedex::{
    context::PokedexClientContext,
    engine::{tetra::Context, EngineContext},
};

use battle::BattleType;

use crate::{
    context::BattleGuiContext,
    transition::TransitionState,
    ui::view::{ActiveRenderer, GuiRemotePlayer},
};

use super::{BattleOpener, Openers, TrainerBattleOpener, WildBattleOpener};

pub struct BattleOpenerManager {
    current: Openers,

    wild: WildBattleOpener,
    trainer: TrainerBattleOpener,
}

impl BattleOpenerManager {
    pub fn new(ctx: &mut Context, gui: &BattleGuiContext) -> Self {
        Self {
            current: Openers::default(),

            wild: WildBattleOpener::new(ctx, gui),
            trainer: TrainerBattleOpener::new(gui),
        }
    }

    pub fn begin<ID: Default, const AS: usize>(
        &mut self,
        ctx: &PokedexClientContext,
        state: &mut TransitionState,
        battle_type: BattleType,
        opponent: &GuiRemotePlayer<ID, AS>,
    ) {
        *state = TransitionState::Run;
        self.current = match battle_type {
            BattleType::Wild => Openers::Wild,
            BattleType::Trainer => Openers::Trainer,
            BattleType::GymLeader => Openers::Trainer,
        };
        let current = self.get_mut::<ID, AS>();
        current.reset();
        current.spawn(ctx, opponent);
    }

    // pub fn end(&mut self, state: &mut TransitionState) {
    //     *state = TransitionState::Begin;
    // }

    pub fn update<ID: Default, const AS: usize>(&mut self, state: &mut TransitionState, delta: f32) {
        let current = self.get_mut::<ID, AS>();
        current.update(delta);
        if current.finished() {
            *state = TransitionState::End;
        }
    }

    pub fn draw_below_panel<ID: Default, const AS: usize>(
        &self,
        ctx: &mut EngineContext,
        player: &ActiveRenderer<AS>,
        opponent: &ActiveRenderer<AS>,
    ) {
        self.get::<ID, AS>().draw_below_panel(ctx, player, opponent);
    }

    pub fn draw<ID: Default, const AS: usize>(&self, ctx: &mut EngineContext) {
        self.get::<ID, AS>().draw(ctx);
    }

    pub fn offset<ID: Default, const AS: usize>(&self) -> f32 {
        self.get::<ID, AS>().offset()
    }

    fn get<ID: Default, const AS: usize>(&self) -> &dyn BattleOpener<ID, AS> {
        match self.current {
            Openers::Wild => &self.wild,
            Openers::Trainer => &self.trainer,
        }
    }

    fn get_mut<ID: Default, const AS: usize>(&mut self) -> &mut dyn BattleOpener<ID, AS> {
        match self.current {
            Openers::Wild => &mut self.wild,
            Openers::Trainer => &mut self.trainer,
        }
    }
}
