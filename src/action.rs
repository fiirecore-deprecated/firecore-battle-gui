use battle::moves::{
    client::{ClientActions, ClientMove},
    MoveTargetLocation,
};
use pokedex::{
    moves::MoveRef,
    pokemon::{Experience, Level},
};

#[derive(Debug, Clone)]
pub enum BattleClientGuiAction<ID> {
    Action(ClientMove<ID>),
    Faint,
    Catch,
    SetExp(Level, Experience, Vec<MoveRef>),
    LevelUp(Vec<MoveRef>),
    Replace(Option<usize>),
}

#[derive(Debug)]
pub enum BattleClientGuiCurrent<ID> {
    Move(Vec<ClientActions<ID>>),
    Switch(usize),
    UseItem(MoveTargetLocation),
    Faint,
    Catch,
    Replace(bool),
    SetExp,
    LevelUp,
}

impl<ID> BattleClientGuiAction<ID> {
    pub fn requires_user(&self) -> bool {
        matches!(self, Self::Faint)
    }
}
