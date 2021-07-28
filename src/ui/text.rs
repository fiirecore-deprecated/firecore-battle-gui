use pokedex::{
    engine::{
        gui::MessageBox,
        tetra::math::Vec2,
        text::{MessagePage, TextColor},
    },
    item::Item,
    moves::Move,
    pokemon::{instance::PokemonInstance, stat::StatStage, Experience, Level},
    status::Status,
    types::Effective,
};

use crate::view::PokemonView;

pub fn new() -> MessageBox {
    let mut messagebox = MessageBox::new(
        super::PANEL_ORIGIN.position + Vec2::new(11.0, 11.0),
        1,
    );
    messagebox.color(TextColor::White);
    messagebox.message.pages.reserve(6);
    messagebox
}

pub(crate) fn on_move(text: &mut MessageBox, pokemon_move: &Move, user: &dyn PokemonView) {
    text.push(MessagePage {
        lines: vec![format!("{} used {}!", user.name(), pokemon_move.name)],
        wait: Some(0.5),
    });
}

pub(crate) fn on_effective(text: &mut MessageBox, effective: &Effective) {
    if effective != &Effective::Effective {
        text.push(MessagePage {
            lines: vec![format!(
                "It was {}{}",
                effective,
                if &Effective::SuperEffective == effective {
                    "!"
                } else {
                    "..."
                }
            )],
            wait: Some(0.5),
        });
    }
}

pub(crate) fn on_crit(text: &mut MessageBox) {
    text.push(MessagePage {
        lines: vec!["It was a critical hit!".to_owned()],
        wait: Some(0.5),
    })
}

pub(crate) fn on_stat_stage(text: &mut MessageBox, pokemon: &dyn PokemonView, stat: &StatStage) {
    text.push(MessagePage {
        lines: vec![
            format!("{}'s {:?} was", pokemon.name(), stat.stat),
            format!(
                "{} by {}!",
                if stat.stage.is_positive() {
                    "raised"
                } else {
                    "lowered"
                },
                stat.stage.abs()
            ),
        ],
        wait: Some(0.5),
    })
}

pub(crate) fn on_status(text: &mut MessageBox, pokemon: &dyn PokemonView, status: &Status) {
    text.push(MessagePage {
        lines: vec![
            format!("{} was afflicted", pokemon.name()),
            format!("with {:?}", status),
        ],
        wait: Some(0.5),
})
}

pub(crate) fn on_miss(text: &mut MessageBox, pokemon: &dyn PokemonView) {
    text.push(MessagePage {
        lines: vec![format!("{} missed!", pokemon.name())],
        wait: Some(0.5),
    });
}

pub(crate) fn on_item(text: &mut MessageBox, pokemon: Option<&dyn PokemonView>, item: &Item) {
    text.push(MessagePage {
        lines: vec![format!(
            "A {} was used on {}",
            item.name,
            pokemon.map(|p| p.name()).unwrap_or("None")
        )],
        wait: Some(0.5),
    });
}

fn on_leave(text: &mut MessageBox, leaving: &dyn PokemonView) {
    text.push(MessagePage {
        lines: vec![format!("Come back, {}!", leaving.name())],
        wait: Some(0.5),
    });
}

pub(crate) fn on_switch(
    text: &mut MessageBox,
    leaving: &dyn PokemonView,
    coming: &dyn PokemonView,
) {
    on_leave(text, leaving);
    on_go(text, coming);
}

pub(crate) fn on_go(text: &mut MessageBox, coming: &dyn PokemonView) {
    text.push(MessagePage {
        lines: vec![format!("Go, {}!", coming.name())],
        wait: Some(0.5),
    });
}

pub(crate) fn on_replace(text: &mut MessageBox, user: &str, coming: Option<&dyn PokemonView>) {
    // if let Some(leaving) = leaving {
    //     on_leave(text, leaving);
    // }
    if let Some(coming) = coming {
        text.push(MessagePage {
            lines: vec![format!("{} sent out {}!", user, coming.name())],
            wait: Some(0.5),
        });
    }
}

pub(crate) fn on_faint(
    text: &mut MessageBox,
    is_wild: bool,
    is_player: bool,
    pokemon: &dyn PokemonView,
) {
    text.push(MessagePage {
        lines: vec![
            match is_player {
                true => pokemon.name().to_string(),
                false => format!(
                    "{} {}",
                    match is_wild {
                        true => "Wild",
                        false => "Foe",
                    },
                    pokemon.name(),
                ),
            },
            String::from("fainted!"),
        ],
        wait: Some(1.0),
    });
}

pub(crate) fn on_catch(text: &mut MessageBox, pokemon: Option<&PokemonInstance>) {
    text.push(match pokemon {
        Some(pokemon) => MessagePage {
            lines: vec![
                String::from("Gotcha!"),
                format!("{} was caught!", pokemon.name()),
            ],
            wait: None,
        },
        None => MessagePage { lines: vec![String::from("Could not catch pokemon!")], wait: Some(2.0) },
    });
}

pub(crate) fn on_gain_exp(
    text: &mut MessageBox,
    pokemon: &PokemonInstance,
    experience: Experience,
    level: Level,
) {
    text.push(MessagePage {
        lines: vec![
            format!("{} gained {} EXP. points", pokemon.name(), experience),
            format!("and {} levels!", level),
        ],
        wait: Some(1.0),
    });
}

// pub(crate) fn on_level_up(text: &mut MessageBox, pokemon: &PokemonInstance, level: Level) {
//     text.push(MessagePage::new(
//         vec![
//             format!("{} grew to", pokemon.name()),
//             format!("LV. {}!", level),
//         ],
//         Some(0.5),
//     ));
// }

pub(crate) fn on_fail(text: &mut MessageBox, lines: Vec<String>) {
    text.push(MessagePage { lines, wait: Some(0.5) });
}
