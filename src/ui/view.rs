use core::ops::{Deref, DerefMut};

use pokedex::{
    id::Identifiable,
    context::PokedexClientContext,
    texture::PokemonTexture,
    engine::{graphics::ZERO, tetra::graphics::Color, EngineContext},
    TrainerId,
};

use battle::player::{RemotePlayer, LocalPlayer};

use crate::{
    context::BattleGuiContext,
    ui::{
        pokemon::{flicker::Flicker, PokemonRenderer, PokemonStatusGui},
        BattleGuiPosition, BattleGuiPositionIndex,
    },
    view::PokemonView,
};

pub type ActiveRenderer = Vec<ActivePokemonRenderer>;
pub type GuiLocalPlayer<ID> = ActivePlayer<LocalPlayer<ID>>;
pub type GuiRemotePlayer<ID> = ActivePlayer<RemotePlayer<ID>>;

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
        party: &LocalPlayer<ID>,
    ) -> ActiveRenderer {
        let size = party.active.len() as u8;
        party
            .active
            .iter()
            .enumerate()
            .map(|(i, index)| {
                let position =
                    BattleGuiPositionIndex::new(BattleGuiPosition::Bottom, i as u8, size);
                let pokemon = (*index).map(|index| &party.pokemon[index]);
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

    pub fn remote<ID>(
        ctx: &BattleGuiContext,
        dex: &PokedexClientContext,
        party: &RemotePlayer<ID>,
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
                        pokemon.map(|pokemon| *pokemon.pokemon().id()),
                        PokemonTexture::Front,
                    ),
                    status: PokemonStatusGui::with_unknown(ctx, dex, position, pokemon),
                }
            })
            .collect()
    }

    pub fn update(&mut self, dex: &PokedexClientContext, pokemon: Option<&dyn PokemonView>) {
        self.update_status(pokemon, true);
        self.renderer
            .new_pokemon(dex, pokemon.map(|pokemon| *pokemon.pokemon().id()));
    }

    pub fn update_status(&mut self, pokemon: Option<&dyn PokemonView>, reset: bool) {
        self.status.update_gui(pokemon, reset);
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
