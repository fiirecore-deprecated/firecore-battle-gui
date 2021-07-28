use pokedex::engine::{graphics::byte_texture, tetra::{Context, graphics::Texture}};

pub struct BattleGuiContext {
    pub panel: Texture,
    pub pokeball: Texture,
    pub smallui: Texture,
    pub padding: Texture,
    pub largeui: Texture,
    pub player: Texture,
}

impl BattleGuiContext {
    pub fn new(ctx: &mut Context) -> Self {
        Self {
            panel: byte_texture(ctx, include_bytes!("../assets/gui/panel.png")),
            pokeball: byte_texture(ctx, include_bytes!("../assets/thrown_pokeball.png")),
            smallui: byte_texture(ctx, include_bytes!("../assets/gui/small.png")),
            padding: byte_texture(ctx, include_bytes!("../assets/gui/padding.png")),
            largeui: byte_texture(ctx, include_bytes!("../assets/gui/large.png")),
            player: byte_texture(ctx, include_bytes!("../assets/player.png")),
        }
    }
}