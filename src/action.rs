use battle::moves::{
    client::{ClientActions, ClientMove},
    usage::target::MoveTargetLocation,
};
use pokedex::{
    moves::MoveRef,
    pokemon::{Experience, Level},
};

#[derive(Debug, Clone)]
pub enum BattleClientGuiAction<'d, ID> {
    Action(ClientMove<ID>),
    Faint,
    Catch,
    SetExp(Level, Experience, Vec<MoveRef<'d>>),
    LevelUp(Vec<MoveRef<'d>>),
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

impl<'d, ID> BattleClientGuiAction<'d, ID> {
    pub fn requires_user(&self) -> bool {
        matches!(self, Self::Faint)
    }
}
