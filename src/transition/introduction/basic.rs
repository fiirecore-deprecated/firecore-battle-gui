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

use battle::BattleType;

use crate::{
    context::BattleGuiContext,
    ui::{
        pokemon::PokemonStatusGui,
        view::{ActiveRenderer, GuiLocalPlayer, GuiRemotePlayer},
    },
    view::PlayerView,
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
    pub(crate) fn concatenate<ID>(party: &impl PlayerView<ID>) -> String {
        let mut string = String::new();
        let len = party.active_len();
        for index in 0..len {
            if let Some(instance) = party.active(index) {
                if index != 0 {
                    if index == len - 2 {
                        string.push_str(", ");
                    } else if index == len - 1 {
                        string.push_str(" and ");
                    }
                }
                string.push_str(&instance.name());
            }
        }
        string
    }

    pub(crate) fn common_setup<ID: Default>(
        &mut self,
        text: &mut MessageBox,
        player: &GuiLocalPlayer<ID>,
    ) {
        text.push(MessagePage {
            lines: vec![format!("Go! {}!", Self::concatenate(&player.player))],
            wait: Some(0.5),
        });
    }

    pub(crate) fn draw_player(&self, ctx: &mut Context, player: &ActiveRenderer) {
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
                active.renderer.draw(ctx, ZERO, Color::WHITE);
            }
        }
    }

    pub(crate) fn draw_opponent(&self, ctx: &mut EngineContext, opponent: &ActiveRenderer) {
        for active in opponent.iter() {
            active.renderer.draw(ctx, ZERO, Color::WHITE);
            active.status.draw(ctx, self.offsets.0, 0.0);
        }
    }
}

impl<ID: Default> BattleIntroduction<ID> for BasicBattleIntroduction {
    fn spawn(
        &mut self,
        _: &PokedexClientContext,
        _: BattleType,
        player: &GuiLocalPlayer<ID>,
        opponent: &GuiRemotePlayer<ID>,
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
        player: &mut GuiLocalPlayer<ID>,
        opponent: &mut GuiRemotePlayer<ID>,
        text: &mut MessageBox,
    ) {
        text.update(ctx, delta);

        if text.page() + 1 == text.pages() && self.counter < Self::PLAYER_DESPAWN {
            self.counter += delta * 180.0;
        }

        if let Some(active) = opponent.renderer.get(0) {
            if active.status.alive() {
                if self.offsets.0 != 0.0 {
                    self.offsets.0 += delta * 240.0;
                    if self.offsets.0 > 0.0 {
                        self.offsets.0 = 0.0;
                    }
                }
            } else if text.waiting() && text.page() >= text.pages() - 2 {
                for active in opponent.renderer.iter_mut() {
                    active.status.spawn();
                }
            }
        }

        if let Some(active) = player.renderer.get(0) {
            if active.renderer.spawner.spawning() {
                for active in player.renderer.iter_mut() {
                    active.renderer.spawner.update(ctx, delta);
                }
            } else if active.status.alive() {
                if self.offsets.1 != 0.0 {
                    self.offsets.1 -= delta * 240.0;
                    if self.offsets.1 < 0.0 {
                        self.offsets.1 = 0.0;
                    }
                }
            } else if self.counter >= Self::PLAYER_T2 {
                for active in player.renderer.iter_mut() {
                    active.renderer.spawn();
                    active.status.spawn();
                }
            }
        }
    }

    fn draw(&self, ctx: &mut EngineContext, player: &ActiveRenderer, opponent: &ActiveRenderer) {
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
