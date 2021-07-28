use pokedex::engine::{
    graphics::{byte_texture, position},
    tetra::{
        Context,
        graphics::Texture,
    }
};

use crate::context::BattleGuiContext;

pub struct BattleBackground {

	background: Texture,
	ground: Texture,
	pub panel: Texture,

}

impl BattleBackground {

    pub fn new(ctx: &mut Context, gui: &BattleGuiContext) -> Self {
        Self {
            background: byte_texture(ctx, include_bytes!("../../assets/background.png")),
            ground: byte_texture(ctx, include_bytes!("../../assets/ground.png")),
            panel: gui.panel.clone(),
        }

    }

    pub fn draw(&self, ctx: &mut Context, offset: f32) {
        self.background.draw(ctx, position(0.0, 1.0));
        self.ground.draw(ctx, position(113.0 - offset, 50.0));
		self.ground.draw(ctx, position(offset, 103.0));
    }

}