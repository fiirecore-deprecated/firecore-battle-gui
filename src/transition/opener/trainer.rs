use pokedex::{
    context::PokedexClientContext,
    engine::{
        graphics::{draw_o_bottom},
        tetra::graphics::Texture,
        util::{Completable, Reset},
        EngineContext,
    },
};

use crate::{context::BattleGuiContext, ui::view::{ActiveRenderer, GuiRemotePlayer}};

use super::{BattleOpener, DefaultBattleOpener};

pub struct TrainerBattleOpener {
    opener: DefaultBattleOpener,
    trainer: Option<Texture>,
}

impl TrainerBattleOpener {
    pub fn new(ctx: &BattleGuiContext) -> Self {
        Self {
            opener: DefaultBattleOpener::new(ctx),
            trainer: None,
        }
    }
}

impl<ID: Default> BattleOpener<ID> for TrainerBattleOpener {
    fn spawn(&mut self, ctx: &PokedexClientContext, opponent: &GuiRemotePlayer<ID>) {
        if let Some(id) = &opponent.trainer {
            self.trainer = Some(ctx.trainer_textures.get(id).clone());
        }
    }

    fn update(&mut self, delta: f32) {
        self.opener.update(delta);
    }

    fn draw_below_panel(
        &self,
        ctx: &mut EngineContext,
        player: &ActiveRenderer,
        opponent: &ActiveRenderer,
    ) {
        draw_o_bottom(ctx, self.trainer.as_ref(), 144.0 - self.opener.offset, 74.0);
        self.opener.draw_below_panel(ctx, player, opponent);
    }

    fn draw(&self, ctx: &mut EngineContext) {
        self.opener.draw(ctx);
    }

    fn offset(&self) -> f32 {
        self.opener.offset
    }
}

impl Reset for TrainerBattleOpener {
    fn reset(&mut self) {
        self.opener.reset();
        self.trainer = None;
    }
}

impl Completable for TrainerBattleOpener {
    fn finished(&self) -> bool {
        self.opener.finished()
    }
}
