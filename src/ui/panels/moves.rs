use pokedex::{
    engine::{
        graphics::{draw_cursor, draw_text_left},
        gui::Panel,
        input::{pressed, Control},
        text::TextColor,
        util::Reset,
        EngineContext,
    },
    moves::Move,
    pokemon::owned::OwnedPokemon,
};

pub struct MovePanel<'d> {
    pub cursor: usize,
    pub names: [Option<(&'d Move, TextColor)>; 4],
}

impl<'d> MovePanel<'d> {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            names: [None; 4],
        }
    }

    pub fn update_names(&mut self, instance: &OwnedPokemon<'d>) {
        for (index, instance) in instance.moves.iter().enumerate() {
            self.names[index] = Some((
                instance.0,
                if instance.empty() {
                    TextColor::Red
                } else {
                    TextColor::Black
                },
            ));
        }
    }

    pub fn input(&mut self, ctx: &EngineContext) -> bool {
        if {
            if pressed(ctx, Control::Up) {
                if self.cursor >= 2 {
                    self.cursor -= 2;
                    true
                } else {
                    false
                }
            } else if pressed(ctx, Control::Down) {
                if self.cursor <= 2 {
                    self.cursor += 2;
                    true
                } else {
                    false
                }
            } else if pressed(ctx, Control::Left) {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    true
                } else {
                    false
                }
            } else if pressed(ctx, Control::Right) {
                if self.cursor < 3 {
                    self.cursor += 1;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } {
            if self.cursor >= self.names.len() {
                self.cursor = self.names.len() - 1;
            }
            true
        } else {
            false
        }
    }

    pub fn draw(&self, ctx: &mut EngineContext) {
        Panel::draw(ctx, 0.0, 113.0, 160.0, 47.0);
        for (index, (pokemon_move, color)) in self.names.iter().flatten().enumerate() {
            let x_offset = if index % 2 == 1 { 72.0 } else { 0.0 };
            let y_offset = if index >> 1 == 1 { 17.0 } else { 0.0 };
            draw_text_left(
                ctx,
                &0,
                &pokemon_move.name,
                *color,
                16.0 + x_offset,
                121.0 + y_offset,
            );
            if index == self.cursor {
                draw_cursor(ctx, 10.0 + x_offset, 123.0 + y_offset);
            }
        }
    }
}

impl<'d> Reset for MovePanel<'d> {
    fn reset(&mut self) {
        self.cursor = 0;
    }
}
