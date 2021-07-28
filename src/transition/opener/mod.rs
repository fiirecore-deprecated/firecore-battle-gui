use pokedex::{
    context::PokedexClientContext,
    engine::{
        graphics::{draw_rectangle, position},
        tetra::graphics::{Color, Rectangle, Texture},
        util::{Completable, Reset, WIDTH},
        EngineContext,
    },
    trainer::TrainerData,
};

use crate::{context::BattleGuiContext, ui::view::ActiveRenderer};

mod manager;

mod trainer;
mod wild;

pub use manager::BattleOpenerManager;
pub use trainer::TrainerBattleOpener;
pub use wild::WildBattleOpener;

pub enum Openers {
    Wild,
    Trainer,
}

impl Default for Openers {
    fn default() -> Self {
        Self::Wild
    }
}

pub(crate) trait BattleOpener: Completable {
    fn spawn(&mut self, ctx: &PokedexClientContext, opponent: Option<&TrainerData>);

    fn update(&mut self, delta: f32);

    fn draw_below_panel(
        &self,
        ctx: &mut EngineContext,
        player: &ActiveRenderer,
        opponent: &ActiveRenderer,
    );

    fn draw(&self, ctx: &mut EngineContext);

    fn offset(&self) -> f32;
}

pub struct DefaultBattleOpener {
    wait: f32,

    offset: f32,

    rect_size: f32,
    shrink_by: f32,

    player: Texture,
}

impl DefaultBattleOpener {
    const RECT_SIZE: f32 = 80.0;
    const SHRINK_BY_DEF: f32 = 1.0;
    const SHRINK_BY_FAST: f32 = 4.0;
    const OFFSET: f32 = 153.0 * 2.0;
    const WAIT: f32 = 0.5;

    pub fn new(ctx: &BattleGuiContext) -> Self {
        Self {
            wait: Self::WAIT,
            rect_size: Self::RECT_SIZE,
            shrink_by: Self::SHRINK_BY_DEF,
            offset: Self::OFFSET,
            player: ctx.player.clone(),
        }
    }
}

impl BattleOpener for DefaultBattleOpener {
    fn spawn(&mut self, _: &PokedexClientContext, _: Option<&TrainerData>) {}

    fn update(&mut self, delta: f32) {
        match self.wait < 0.0 {
            false => self.wait -= delta,
            true => {
                if self.offset > 0.0 {
                    self.offset -= 120.0 * delta;
                    if self.offset < 0.0 {
                        self.offset = 0.0;
                    }
                }
                if self.rect_size > 0.0 {
                    if self.rect_size > 0.0 {
                        self.rect_size -= self.shrink_by * 60.0 * delta;
                        if self.rect_size < 0.0 {
                            self.rect_size = 0.0;
                        }
                    } else {
                        self.rect_size = 0.0;
                    }
                    if self.rect_size <= 58.0 && self.shrink_by != Self::SHRINK_BY_FAST {
                        self.shrink_by = Self::SHRINK_BY_FAST;
                    }
                }
            }
        }
    }

    fn draw_below_panel(
        &self,
        ctx: &mut EngineContext,
        _player: &ActiveRenderer,
        _opponent: &ActiveRenderer,
    ) {
        self.player.draw_region(
            ctx,
            Rectangle::new(0.0, 0.0, 64.0, 64.0),
            position(41.0 + self.offset, 49.0),
        )
    }

    fn draw(&self, ctx: &mut EngineContext) {
        draw_rectangle(ctx, 0.0, 0.0, WIDTH, self.rect_size, Color::BLACK);
        draw_rectangle(
            ctx,
            0.0,
            160.0 - self.rect_size,
            WIDTH,
            self.rect_size,
            Color::BLACK,
        );
    }

    fn offset(&self) -> f32 {
        self.offset
    }
}

impl Reset for DefaultBattleOpener {
    fn reset(&mut self) {
        self.offset = Self::OFFSET;
        self.rect_size = Self::RECT_SIZE;
        self.shrink_by = Self::SHRINK_BY_DEF;
        self.wait = Self::WAIT;
    }
}

impl Completable for DefaultBattleOpener {
    fn finished(&self) -> bool {
        self.offset <= 0.0
    }
}
