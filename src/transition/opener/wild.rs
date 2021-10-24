use pokedex::{
    context::PokedexClientContext,
    engine::{
        graphics::{byte_texture, position, LIGHTGRAY},
        tetra::{graphics::Texture, math::Vec2, Context},
        util::{Completable, Reset},
        EngineContext,
    },
};

use crate::{
    context::BattleGuiContext,
    ui::view::{ActiveRenderer, GuiRemotePlayer},
};

use super::{BattleOpener, DefaultBattleOpener};

pub struct WildBattleOpener {
    opener: DefaultBattleOpener,

    grass: Texture,
    offset: Vec2<f32>,
}

impl WildBattleOpener {
    const GRASS_WIDTH: f32 = 128.0;
    const GRASS_HEIGHT: f32 = 47.0;
    pub fn new(ctx: &mut Context, gui: &BattleGuiContext) -> Self {
        Self {
            opener: DefaultBattleOpener::new(gui),
            grass: byte_texture(ctx, include_bytes!("../../../assets/grass.png")),
            offset: Vec2::new(Self::GRASS_WIDTH, Self::GRASS_HEIGHT),
        }
    }
}

impl<ID, const AS: usize> BattleOpener<ID, AS> for WildBattleOpener {
    fn spawn(&mut self, _: &PokedexClientContext, _: &GuiRemotePlayer<ID, AS>) {}

    fn update(&mut self, delta: f32) {
        self.opener.update(delta);
        if self.offset.y > 0.0 {
            self.offset.x -= 360.0 * delta;
            if self.offset.x < 0.0 {
                self.offset.x += Self::GRASS_WIDTH;
            }
            if self.opener.offset() <= 130.0 {
                self.offset.y -= 60.0 * delta;
            }
        }
    }

    fn offset(&self) -> f32 {
        self.opener.offset
    }

    fn draw_below_panel(
        &self,
        ctx: &mut EngineContext,
        player: &ActiveRenderer<AS>,
        opponent: &ActiveRenderer<AS>,
    ) {
        for active in opponent.iter() {
            active
                .pokemon
                .draw(ctx, Vec2::new(-self.opener.offset, 0.0), LIGHTGRAY);
        }
        self.opener.draw_below_panel(ctx, player, opponent);
        if self.offset.y > 0.0 {
            let y = 114.0 - self.offset.y;
            self.grass
                .draw(ctx, position(self.offset.x - Self::GRASS_WIDTH, y));
            self.grass.draw(ctx, position(self.offset.x, y));
            self.grass
                .draw(ctx, position(self.offset.x + Self::GRASS_WIDTH, y));
        }
    }

    fn draw(&self, ctx: &mut EngineContext) {
        self.opener.draw(ctx);
    }
}

impl Reset for WildBattleOpener {
    fn reset(&mut self) {
        self.offset = Vec2::new(Self::GRASS_WIDTH, Self::GRASS_HEIGHT);
        self.opener.reset();
    }
}
impl Completable for WildBattleOpener {
    fn finished(&self) -> bool {
        self.opener.finished() && self.offset.y <= 0.0
    }
}
