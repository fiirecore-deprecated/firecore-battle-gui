use pokedex::{
    engine::{
        gui::MessageBox,
        input::{pressed, Control},
        text::{MessagePage, TextColor},
        util::{Completable, Entity},
        EngineContext,
    },
    moves::Move,
    pokemon::owned::OwnedPokemon,
};

use super::moves::MovePanel;

pub struct LevelUpMovePanel<'d> {
    
    state: LevelUpState,

    move_panel: MovePanel<'d>,

    moves: Vec<&'d Move>,
}

enum LevelUpState {
    NotAlive,
    Text,
    Moves,
}

impl<'d> LevelUpMovePanel<'d> {
    pub fn new() -> Self {
        Self {
            state: LevelUpState::NotAlive,
            move_panel: MovePanel::new(),
            moves: Vec::new(),
        }
    }

    pub fn spawn(&mut self, instance: &OwnedPokemon<'d>, text: &mut MessageBox, moves: Vec<&'d Move>) {
        self.state = LevelUpState::Text;
        self.moves = moves;
        self.move_panel.update_names(instance);
        text.despawn();
    }

    pub fn update(&mut self, ctx: &EngineContext, text: &mut MessageBox, delta: f32, pokemon: &mut OwnedPokemon<'d>) -> Option<(usize, &'d Move)> {
        match self.state {
            LevelUpState::Text => {
                match text.alive() {
                    true => {
                        text.update(ctx, delta);
                        if text.finished() {
                            self.state = LevelUpState::Moves;
                            text.despawn();
                        }
                        None
                    },
                    false => match self.moves.first() {
                        Some(move_ref) => {
                            text.spawn();
                            text.push(MessagePage {
                                lines: vec![
                                    format!("{} is trying to", pokemon.name()),
                                    format!("learn {}", move_ref.name),
                                ],
                                wait: None,
                            });
                            self.update(ctx, text, delta, pokemon)
                        }
                        None => {
                            self.state = LevelUpState::NotAlive;
                            None
                        }
                    }
                }
            },
            LevelUpState::Moves => {
                self.move_panel.input(ctx);
                let a = pressed(ctx, Control::A);
                if pressed(ctx, Control::B) || a {
                    self.state = LevelUpState::Text;
                    let pokemon_move = self.moves.remove(0);
                    if a {
                        self.move_panel.names[self.move_panel.cursor] =
                        Some((pokemon_move, TextColor::Black));
                        pokemon.moves.add(Some(self.move_panel.cursor), &pokemon_move.id);
                        return Some((self.move_panel.cursor, pokemon_move));
                    }
                }
                None
            },
            LevelUpState::NotAlive => None,
        }
    }

    pub fn draw(&self, ctx: &mut EngineContext) {
        match self.state {
            LevelUpState::Moves => self.move_panel.draw(ctx),
            LevelUpState::Text | LevelUpState::NotAlive => (),
        }
    }

    pub fn alive(&self) -> bool {
        !matches!(self.state, LevelUpState::NotAlive)
    }

}
