use core::ops::{Deref, DerefMut};

use pokedex::{
    context::PokedexClientContext,
    engine::{graphics::ZERO, tetra::graphics::Color, EngineContext},
    pokemon::owned::OwnedPokemon,
    texture::PokemonTexture,
    Identifiable, TrainerId,
};

use battle::party::PlayerParty;

use crate::{
    context::BattleGuiContext,
    ui::{
        pokemon::{flicker::Flicker, PokemonRenderer, PokemonStatusGui},
        BattleGuiPosition, BattleGuiPositionIndex,
    },
    view::InitUnknownPokemon,
};

pub type InitLocalPlayer<'d, ID, const AS: usize> = PlayerParty<ID, usize, OwnedPokemon<'d>, AS>;
pub type InitRemotePlayer<'d, ID, const AS: usize> =
    PlayerParty<ID, usize, Option<InitUnknownPokemon<'d>>, AS>;

pub type ActiveRenderer<const AS: usize> = [ActivePokemonRenderer; AS];
pub type GuiLocalPlayer<'d, ID, const AS: usize> = ActivePlayer<ID, OwnedPokemon<'d>, AS>;
pub type GuiRemotePlayer<'d, ID, const AS: usize> =
    ActivePlayer<ID, Option<InitUnknownPokemon<'d>>, AS>;

pub struct ActivePlayer<ID, P, const AS: usize> {
    pub player: PlayerParty<ID, usize, P, AS>,
    pub renderer: ActiveRenderer<AS>,
    pub trainer: Option<TrainerId>,
}

impl<ID, P, const AS: usize> ActivePlayer<ID, P, AS> {
    pub fn new(player: PlayerParty<ID, usize, P, AS>) -> Self {
        let mut arr: [ActivePokemonRenderer; AS] =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };

        for a in &mut arr {
            *a = Default::default()
        }

        Self {
            player,
            renderer: arr,
            trainer: Default::default(),
        }
    }

}

pub struct ActivePokemonRenderer {
    pub pokemon: PokemonRenderer,
    /// to - do: make non-optional
    pub status: PokemonStatusGui,
}

impl ActivePokemonRenderer {
    pub fn draw(&self, ctx: &mut EngineContext) {
        self.pokemon.draw(ctx, ZERO, Color::WHITE);
        self.status.draw(
            ctx,
            0.0,
            if self.pokemon.flicker.accumulator % Flicker::HALF > Flicker::HALF / 8.0
                && self.pokemon.flicker.remaining > (Flicker::TIMES >> 1)
            {
                0.0
            } else {
                1.0
            },
        );
        // self.renderer.moves.draw(ctx);
    }
}

impl Default for ActivePokemonRenderer {
    fn default() -> Self {
        log::debug!("fix maybe uninit ActivePokemonRenderer");
        Self {
            pokemon: Default::default(),
            status: unsafe { std::mem::MaybeUninit::zeroed().assume_init() },
        }
    }
}

impl<'d, ID, const AS: usize> ActivePlayer<ID, OwnedPokemon<'d>, AS> {
    pub fn init(&mut self, ctx: &BattleGuiContext, dex: &PokedexClientContext) {
        let size = self.player.active.len() as u8;

        for (i, index) in self.player.active.iter().enumerate() {
            let position = BattleGuiPositionIndex::new(BattleGuiPosition::Bottom, i as u8, size);
            let pokemon = (*index).map(|index| &self.player.pokemon[index]);
            self.renderer[i] = ActivePokemonRenderer {
                pokemon: PokemonRenderer::with(
                    ctx,
                    dex,
                    position,
                    pokemon.map(|pokemon| *pokemon.pokemon.id()),
                    PokemonTexture::Back,
                ),
                status: PokemonStatusGui::with_known(ctx, dex, position, pokemon),
            }
        }
    }
}

impl<'d, ID, const AS: usize> ActivePlayer<ID, Option<InitUnknownPokemon<'d>>, AS> {
    pub fn init(&mut self, ctx: &BattleGuiContext, dex: &PokedexClientContext) {
        let size = self.player.active.len() as u8;

        for (i, index) in self.player.active.iter().enumerate() {
            let position = BattleGuiPositionIndex::new(BattleGuiPosition::Top, i as u8, size);
            let pokemon = (*index)
                .map(|index| self.player.pokemon[index].as_ref())
                .flatten();
            self.renderer[i] = ActivePokemonRenderer {
                pokemon: PokemonRenderer::with(
                    ctx,
                    dex,
                    position,
                    pokemon.map(|pokemon| *pokemon.pokemon.id()),
                    PokemonTexture::Front,
                ),
                status: PokemonStatusGui::with_unknown(ctx, dex, position, pokemon),
            };
        }
    }
}
