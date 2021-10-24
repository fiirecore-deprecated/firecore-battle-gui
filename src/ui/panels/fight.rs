use pokedex::{
    engine::{
        EngineContext,
        util::Reset,
    },
    pokemon::owned::OwnedPokemon,
};

use super::{move_info::MoveInfoPanel, moves::MovePanel};

pub struct FightPanel<'d> {
    pub moves: MovePanel<'d>,
    info: MoveInfoPanel,
}

impl<'d> FightPanel<'d> {
    pub fn new() -> Self {
        Self {
            moves: MovePanel::new(),
            info: MoveInfoPanel::new(),
        }
    }

    pub fn user(&mut self, instance: &OwnedPokemon<'d>) {
        self.moves.update_names(instance);
        self.update_move(instance);
    }

    pub fn update_move(&mut self, pokemon: &OwnedPokemon<'d>) {
        if let Some(pmove) = pokemon.moves.get(self.moves.cursor) {
            self.info.update_move(pmove);
        }
    }

    pub fn draw(&self, ctx: &mut EngineContext) {
        self.moves.draw(ctx);
        self.info.draw(ctx);
    }

    pub fn input(&mut self, ctx: &EngineContext, pokemon: &OwnedPokemon<'d>) {
        if self.moves.input(ctx) {
            self.update_move(pokemon);
        }
    }
}

impl<'d> Reset for FightPanel<'d> {
    fn reset(&mut self) {
        if self.moves.cursor >= self.moves.names.len() {
            self.moves.cursor = 0;
        }
    }
}
