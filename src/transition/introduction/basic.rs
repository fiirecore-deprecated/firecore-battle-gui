use pokedex::{
    context::PokedexClientContext,
    engine::{
        graphics::{position, ZERO},
        gui::MessageBox,
        tetra::{
            graphics::{Color, Rectangle, Texture},
            Context,
        },
        text::MessagePage,
        util::{Completable, Entity, Reset},
        EngineContext,
    },
};

use battle::{party::PlayerParty, BattleType};

use crate::{
    context::BattleGuiContext,
    ui::{
        pokemon::PokemonStatusGui,
        view::{ActiveRenderer, GuiLocalPlayer, GuiRemotePlayer},
    },
    view::GuiPokemonView,
};

use super::BattleIntroduction;

pub struct BasicBattleIntroduction {
    player: Texture,
    counter: f32,
    offsets: (f32, f32),
}
impl BasicBattleIntroduction {
    const OFFSETS: (f32, f32) = (
        -PokemonStatusGui::BATTLE_OFFSET,
        PokemonStatusGui::BATTLE_OFFSET,
    );

    const PLAYER_T1: f32 = 42.0;
    const PLAYER_T2: f32 = Self::PLAYER_T1 + 18.0;
    const PLAYER_T3: f32 = Self::PLAYER_T2 + 18.0;
    const PLAYER_DESPAWN: f32 = 104.0;

    pub fn new(ctx: &BattleGuiContext) -> Self {
        Self {
            player: ctx.player.clone(),
            counter: 0.0,
            offsets: Self::OFFSETS, // opponent, player
        }
    }

    #[deprecated(note = "bad code, return vec of string (lines)")]
    pub(crate) fn concatenate<'d, ID, P: GuiPokemonView<'d>, const AS: usize>(party: &PlayerParty<ID, usize, P, AS>) -> String {
        let mut string = String::with_capacity(
            party
                .active_iter()
                .map(|(.., p)| p.name().len() + 2)
                .sum::<usize>()
                + 2,
        );
        let len = party.active.len();
        for (index, instance) in party.active_iter() {
            if index != 0 {
                if index == len - 2 {
                    string.push_str(", ");
                } else if index == len - 1 {
                    string.push_str(" and ");
                }
            }
            string.push_str(&instance.name());
        }
        string
    }

    pub(crate) fn common_setup<ID: Default, const AS: usize>(
        &mut self,
        text: &mut MessageBox,
        player: &GuiLocalPlayer<ID, AS>,
    ) {
        text.push(MessagePage {
            lines: vec![format!("Go! {}!", Self::concatenate(&player.player))],
            wait: Some(0.5),
        });
    }

    pub(crate) fn draw_player<const AS: usize>(&self, ctx: &mut Context, player: &ActiveRenderer<AS>) {
        if self.counter < Self::PLAYER_DESPAWN {
            self.player.draw_region(
                ctx,
                Rectangle::new(
                    0.0,
                    if self.counter >= Self::PLAYER_T3 {
                        // 78.0
                        256.0
                    } else if self.counter >= Self::PLAYER_T2 {
                        // 60.0
                        192.0
                    } else if self.counter >= Self::PLAYER_T1 {
                        // 42.0
                        128.0
                    } else if self.counter > 0.0 {
                        64.0
                    } else {
                        0.0
                    },
                    64.0,
                    64.0,
                ),
                position(41.0 + -self.counter, 49.0),
            )
        } else {
            for active in player.iter() {
                active.pokemon.draw(ctx, ZERO, Color::WHITE);
            }
        }
    }

    pub(crate) fn draw_opponent<const AS: usize>(&self, ctx: &mut EngineContext, opponent: &ActiveRenderer<AS>) {
        for active in opponent.iter() {
            active.pokemon.draw(ctx, ZERO, Color::WHITE);
            active.status.draw(ctx, self.offsets.0, 0.0);
        }
    }

    fn offsets0(&mut self, delta: f32) {
        if self.offsets.0 != 0.0 {
            self.offsets.0 += delta * 240.0;
            if self.offsets.0 > 0.0 {
                self.offsets.0 = 0.0;
            }
        }
    }

    fn offsets1(&mut self, delta: f32) {
        if self.offsets.1 != 0.0 {
            self.offsets.1 -= delta * 240.0;
            if self.offsets.1 < 0.0 {
                self.offsets.1 = 0.0;
            }
        }
    }

}

impl<ID: Default, const AS: usize> BattleIntroduction<ID, AS> for BasicBattleIntroduction {
    fn spawn(
        &mut self,
        _: &PokedexClientContext,
        _: BattleType,
        player: &GuiLocalPlayer<ID, AS>,
        opponent: &GuiRemotePlayer<ID, AS>,
        text: &mut MessageBox,
    ) {
        text.clear();
        text.push(MessagePage {
            lines: vec![format!(
                "Wild {} appeared!",
                Self::concatenate(&opponent.player)
            )],
            wait: None,
        });
        self.common_setup(text, player);
    }

    fn update(
        &mut self,
        ctx: &EngineContext,
        delta: f32,
        player: &mut GuiLocalPlayer<ID, AS>,
        opponent: &mut GuiRemotePlayer<ID, AS>,
        text: &mut MessageBox,
    ) {
        text.update(ctx, delta);

        if text.page() + 1 == text.pages() && self.counter < Self::PLAYER_DESPAWN {
            self.counter += delta * 180.0;
        }

        if let Some(active) = opponent.renderer.get(0) {
            if active.status.alive() {
                self.offsets0(delta);
            } else if text.waiting() && text.page() >= text.pages() - 2 {
                for active in opponent.renderer.iter_mut() {
                    active.status.spawn();
                }
            }
        } else {
            self.offsets0(delta);
        }

        if let Some(active) = player.renderer.get(0) {
            if active.pokemon.spawner.spawning() {
                for active in player.renderer.iter_mut() {
                    active.pokemon.spawner.update(ctx, delta);
                }
            } else if active.status.alive() {
                self.offsets1(delta);
            } else if self.counter >= Self::PLAYER_T2 {
                for active in player.renderer.iter_mut() {
                    active.pokemon.spawn();
                    active.status.spawn();
                }
            }
        } else {
            self.offsets1(delta);
        }
    }

    fn draw(&self, ctx: &mut EngineContext, player: &ActiveRenderer<AS>, opponent: &ActiveRenderer<AS>) {
        self.draw_opponent(ctx, opponent);
        self.draw_player(ctx, player);
    }
}

impl Reset for BasicBattleIntroduction {
    fn reset(&mut self) {
        self.counter = 0.0;
        self.offsets = Self::OFFSETS;
    }
}
impl Completable for BasicBattleIntroduction {
    fn finished(&self) -> bool {
        self.counter >= Self::PLAYER_DESPAWN && self.offsets.1 == 0.0
    }
}
