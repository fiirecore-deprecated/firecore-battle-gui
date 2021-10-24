use battle::{
    moves::{ClientMove, ClientMoveAction},
    pokemon::{Indexed, PokemonIdentifier},
};
use pokedex::{
    moves::Move,
    pokemon::{Experience, Level},
};

#[derive(Debug, Clone)]
pub enum BattleClientGuiAction<'d, ID> {
    Action(ClientMove<ID>),
    Faint,
    Catch,
    SetExp(Level, Experience, Vec<&'d Move>),
    LevelUp(Vec<&'d Move>),
    Replace(Option<usize>),
}

#[derive(Debug)]
pub enum BattleClientGuiCurrent<ID> {
    Move(Vec<Indexed<ID, ClientMoveAction>>),
    Switch(usize),
    UseItem(PokemonIdentifier<ID>),
    Faint,
    Catch,
    Replace(bool),
    SetExp,
    LevelUp,
}

impl<'d, ID> BattleClientGuiAction<'d, ID> {
    pub fn requires_user(&self) -> bool {
        matches!(self, Self::Faint)
    }
}
