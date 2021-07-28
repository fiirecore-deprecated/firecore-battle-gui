use pokedex::{
    context::PokedexClientContext,
    engine::{graphics::ZERO, tetra::graphics::Color, EngineContext},
};

use pokedex::{
    battle::party::knowable::{BattlePartyKnown, BattlePartyUnknown},
    id::Identifiable,
    texture::PokemonTexture,
};

use crate::{
    context::BattleGuiContext,
    ui::{
        pokemon::{flicker::Flicker, PokemonRenderer, PokemonStatusGui},
        BattleGuiPosition, BattleGuiPositionIndex,
    },
    view::PokemonView,
};

pub type ActiveRenderer = Vec<ActivePokemonRenderer>;

#[derive(Default)]
pub struct ActivePokemonParty<T> {
    pub party: T,
    pub renderer: ActiveRenderer,
}

pub struct ActivePokemonRenderer {
    pub renderer: PokemonRenderer,
    pub status: PokemonStatusGui,
}

impl ActivePokemonRenderer {
    pub fn init_known<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord>(
        ctx: &BattleGuiContext,
        dex: &PokedexClientContext,
        party: &BattlePartyKnown<ID>,
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

    pub fn init_unknown<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord>(
        ctx: &BattleGuiContext,
        dex: &PokedexClientContext,
        party: &BattlePartyUnknown<ID>,
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
