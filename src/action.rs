use battle::moves::{target::TargetLocation, ClientMove, ClientMoveAction};
use pokedex::{
    moves::MoveRef,
    pokemon::{Experience, Level},
};

#[derive(Debug, Clone)]
pub enum BattleClientGuiAction<'d> {
    Action(ClientMove),
    Faint,
    Catch,
    SetExp(Level, Experience, Vec<MoveRef<'d>>),
    LevelUp(Vec<MoveRef<'d>>),
    Replace(Option<usize>),
}

#[derive(Debug)]
pub enum BattleClientGuiCurrent {
    Move(Vec<(TargetLocation, ClientMoveAction)>),
    Switch(usize),
    UseItem(TargetLocation),
    Faint,
    Catch,
    Replace(bool),
    SetExp,
    LevelUp,
}

impl<'d> BattleClientGuiAction<'d> {
    pub fn requires_user(&self) -> bool {
        matches!(self, Self::Faint)
    }
}
