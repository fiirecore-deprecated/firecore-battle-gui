use core::ops::{Deref, DerefMut};

use pokedex::{
    context::PokedexClientContext,
    engine::{graphics::ZERO, tetra::graphics::Color, EngineContext},
    texture::PokemonTexture,
    Identifiable, TrainerId,
};

use battle::player::{InitRemotePlayer, LocalPlayer};

use crate::{
    context::BattleGuiContext,
    ui::{
        pokemon::{flicker::Flicker, PokemonRenderer, PokemonStatusGui},
        BattleGuiPosition, BattleGuiPositionIndex,
    },
};

pub type ActiveRenderer = Vec<ActivePokemonRenderer>;
pub type GuiLocalPlayer<'d, ID> = ActivePlayer<LocalPlayer<'d, ID>>;
pub type GuiRemotePlayer<'d, ID> = ActivePlayer<InitRemotePlayer<'d, ID>>;

#[derive(Default)]
pub struct ActivePlayer<T: Default> {
    pub player: T,
    pub renderer: ActiveRenderer,
    pub trainer: Option<TrainerId>,
}

impl<T: Default> Deref for ActivePlayer<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.player
    }
}

impl<T: Default> DerefMut for ActivePlayer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.player
    }
}

pub struct ActivePokemonRenderer {
    pub renderer: PokemonRenderer,
    pub status: PokemonStatusGui,
}

impl ActivePokemonRenderer {
    pub fn local<ID>(
        ctx: &BattleGuiContext,
        dex: &PokedexClientContext,
        player: &LocalPlayer<ID>,
    ) -> ActiveRenderer {
        let size = player.active.len() as u8;
        player
            .active
            .iter()
            .enumerate()
            .map(|(i, index)| {
                let position =
                    BattleGuiPositionIndex::new(BattleGuiPosition::Bottom, i as u8, size);
                let pokemon = (*index).map(|index| &player.pokemon[index]);
                Self {
                    renderer: PokemonRenderer::with(
                        ctx,
                        dex,
                        position,
                        pokemon.map(|pokemon| *pokemon.pokemon.id()),
                        PokemonTexture::Back,
                    ),
                    status: PokemonStatusGui::with_known(ctx, dex, position, pokemon),
                }
            })
            .collect()
    }

    pub fn remote<'d, ID>(
        ctx: &BattleGuiContext,
        dex: &PokedexClientContext,
        party: &InitRemotePlayer<'d, ID>,
    ) -> ActiveRenderer {
        let size = party.active.len() as u8;
        party
            .active
            .iter()
            .enumerate()
            .map(|(i, index)| {
                let position = BattleGuiPositionIndex::new(BattleGuiPosition::Top, i as u8, size);
                let pokemon = (*index)
                    .map(|index| party.pokemon[index].as_ref())
                    .flatten();
                Self {
                    renderer: PokemonRenderer::with(
                        ctx,
                        dex,
                        position,
                        pokemon.map(|pokemon| *pokemon.pokemon.id()),
                        PokemonTexture::Front,
                    ),
                    status: PokemonStatusGui::with_unknown(ctx, dex, position, pokemon),
                }
            })
            .collect()
    }

    pub fn draw(&self, ctx: &mut EngineContext) {
        self.renderer.draw(ctx, ZERO, Color::WHITE);
        self.status.draw(
            ctx,
            0.0,
            if self.renderer.flicker.accumulator % Flicker::HALF > Flicker::HALF / 8.0
                && self.renderer.flicker.remaining > (Flicker::TIMES >> 1)
            {
                0.0
            } else {
                1.0
            },
        );
        self.renderer.moves.draw(ctx);
    }
}
