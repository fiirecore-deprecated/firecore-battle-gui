use pokedex::{
    engine::{
        util::{Reset, Completable},
        graphics::{draw_o_bottom, TextureManager}, 
        tetra::{
            Context,
            graphics::Texture,
        },
    },
    texture::TrainerTextures,
    trainer::TrainerData,
};

use crate::ui::view::ActiveRenderer;

use super::{BattleOpener, DefaultBattleOpener};

pub struct TrainerBattleOpener {
    opener: DefaultBattleOpener,
    trainer: Option<Texture>,
}

impl TrainerBattleOpener {
    pub fn new(ctx: &mut Context) -> Self {
        Self {
            opener: DefaultBattleOpener::new(ctx),
            trainer: None,
        }
    }
}

impl BattleOpener for TrainerBattleOpener {

    fn spawn(&mut self, opponent: Option<&TrainerData>) {
        if let Some(trainer) = opponent {
            self.trainer = Some(TrainerTextures::get(&trainer.npc_type).clone());
        }
    }

    fn update(&mut self, delta: f32) {
        self.opener.update(delta);
    }

    fn draw_below_panel(&self, ctx: &mut Context, player: &ActiveRenderer, opponent: &ActiveRenderer) {
        draw_o_bottom(ctx, self.trainer.as_ref(), 144.0 - self.opener.offset, 74.0);
        self.opener.draw_below_panel(ctx, player, opponent);
    }

    fn draw(&self, ctx: &mut Context) {
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