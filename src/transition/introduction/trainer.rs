use pokedex::{
    battle::party::knowable::{BattlePartyKnown, BattlePartyUnknown},
    context::PokedexClientContext,
    engine::{
        graphics::{draw_o_bottom, TextureManager},
        gui::MessageBox,
        tetra::graphics::Texture,
        text::MessagePage,
        util::{Completable, Reset},
        EngineContext,
    },
};

use battle::data::BattleType;

use crate::{
    context::BattleGuiContext,
    ui::view::{ActivePokemonParty, ActiveRenderer},
};

use super::{basic::BasicBattleIntroduction, BattleIntroduction};

pub struct TrainerBattleIntroduction {
    introduction: BasicBattleIntroduction,

    texture: Option<Texture>,
    offset: f32,
    leaving: bool,
}

impl TrainerBattleIntroduction {
    const FINAL_TRAINER_OFFSET: f32 = 126.0;

    pub fn new(ctx: &BattleGuiContext) -> Self {
        Self {
            introduction: BasicBattleIntroduction::new(ctx),
            texture: None,
            offset: 0.0,
            leaving: false,
        }
    }
}

impl<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord> BattleIntroduction<ID>
    for TrainerBattleIntroduction
{
    fn spawn(
        &mut self,
        ctx: &PokedexClientContext,
        _battle_type: BattleType,
        player: &BattlePartyKnown<ID>,
        opponent: &BattlePartyUnknown<ID>,
        text: &mut MessageBox,
    ) {
        text.clear();

        if let Some(trainer) = &opponent.trainer {
            self.texture = Some(ctx.trainer_textures.get(&trainer.npc_type).clone());

            let name = format!("{} {}", trainer.prefix, trainer.name);

            text.push(MessagePage {
                lines: vec![name.clone(), String::from("would like to battle!")],
                wait: None,
            });

            text.push(MessagePage {
                lines: vec![
                    name + " sent",
                    format!("out {}", BasicBattleIntroduction::concatenate(opponent)),
                ],
                wait: Some(0.5),
            });
        } else {
            text.push(MessagePage {
                lines: vec![String::from("No trainer data found!")],
                wait: None,
            });
        }

        self.introduction.common_setup(text, player);
    }

    fn update(
        &mut self,
        ctx: &EngineContext,
        delta: f32,
        player: &mut ActivePokemonParty<BattlePartyKnown<ID>>,
        opponent: &mut ActivePokemonParty<BattlePartyUnknown<ID>>,
        text: &mut MessageBox,
    ) {
        self.introduction.update(ctx, delta, player, opponent, text);
        if text.waiting() && text.page() == text.pages() - 2 {
            self.leaving = true;
        }
        if self.leaving && self.offset < Self::FINAL_TRAINER_OFFSET {
            self.offset += 300.0 * delta;
        }
    }

    fn draw(&self, ctx: &mut EngineContext, player: &ActiveRenderer, opponent: &ActiveRenderer) {
        if self.offset < Self::FINAL_TRAINER_OFFSET {
            draw_o_bottom(ctx, self.texture.as_ref(), 144.0 + self.offset, 74.0);
        } else {
            self.introduction.draw_opponent(ctx, opponent);
        }
        self.introduction.draw_player(ctx, player);
    }
}

impl Completable for TrainerBattleIntroduction {
    fn finished(&self) -> bool {
        self.introduction.finished()
    }
}

impl Reset for TrainerBattleIntroduction {
    fn reset(&mut self) {
        self.introduction.reset();
        self.offset = 0.0;
        self.leaving = false;
    }
}
