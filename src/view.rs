use pokedex::{
    pokemon::{Experience, Health, Level, PokemonInstance},
    status::StatusEffectInstance,
};

use battle::{
    party::PlayerParty,
    player::PlayerKnowable,
    pokemon::{PokemonView, UnknownPokemon},
};

type Active = usize;
type PartyIndex = usize;

#[deprecated]
pub trait PlayerView<ID> {
    fn id(&self) -> &ID;

    fn name(&self) -> &str;

    fn active(&self, active: Active) -> Option<&dyn GuiPokemonView>;

    fn active_mut(&mut self, active: Active) -> Option<&mut dyn GuiPokemonView>;

    fn active_eq(&self, active: Active, index: &Option<PartyIndex>) -> bool;

    fn pokemon(&self, index: PartyIndex) -> Option<&dyn GuiPokemonView>;

    fn replace(&mut self, active: Active, new: Option<PartyIndex>);
}

impl<ID, P: GuiPokemonView> PlayerView<ID> for PlayerKnowable<ID, P> {
    fn id(&self) -> &ID {
        &self.id
    }

    fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("Unknown")
    }

    fn active(&self, active: usize) -> Option<&dyn GuiPokemonView> {
        PlayerParty::active(self, active).map(|p| p as _)
    }

    fn active_mut(&mut self, active: usize) -> Option<&mut dyn GuiPokemonView> {
        PlayerParty::active_mut(self, active).map(|p| p as _)
    }

    fn active_eq(&self, active: usize, index: &Option<usize>) -> bool {
        self.active
            .get(active)
            .map(|i| i == index)
            .unwrap_or_default()
    }

    fn pokemon(&self, index: usize) -> Option<&dyn GuiPokemonView> {
        self.pokemon.get(index).map(|p| p as _)
    }

    fn replace(&mut self, active: usize, new: Option<usize>) {
        PlayerParty::replace(self, active, new)
    }

}

pub trait GuiPokemonView: PokemonView {
    fn set_level(&mut self, level: Level);

    fn set_hp(&mut self, hp: f32);
    fn hp(&self) -> f32;

    fn set_effect(&mut self, effect: StatusEffectInstance);
    fn effect(&mut self) -> Option<&mut StatusEffectInstance>;

    fn set_exp(&mut self, experience: Experience);

    fn instance(&mut self) -> Option<&mut PokemonInstance>;

    fn view(&self) -> &dyn PokemonView;

    fn exp(&self) -> Experience;
}

impl GuiPokemonView for PokemonInstance {
    fn set_level(&mut self, level: Level) {
        self.level = level;
    }

    fn set_hp(&mut self, hp: f32) {
        self.current_hp = (hp.max(0.0) * self.max_hp() as f32) as Health
    }

    fn hp(&self) -> f32 {
        self.percent_hp()
    }

    fn set_effect(&mut self, effect: StatusEffectInstance) {
        self.effect = Some(effect);
    }

    fn effect(&mut self) -> Option<&mut StatusEffectInstance> {
        self.effect.as_mut()
    }

    fn set_exp(&mut self, experience: Experience) {
        self.experience = experience;
    }

    fn instance(&mut self) -> Option<&mut PokemonInstance> {
        Some(self)
    }

    fn view(&self) -> &dyn PokemonView {
        self as _
    }

    fn exp(&self) -> Experience {
        self.experience
    }
}

impl GuiPokemonView for Option<UnknownPokemon> {
    fn set_level(&mut self, level: Level) {
        if let Some(u) = self.as_mut() {
            u.level = level;
        }
    }

    fn set_hp(&mut self, hp: f32) {
        if let Some(u) = self.as_mut() {
            u.hp = hp;
        }
    }

    fn hp(&self) -> f32 {
        self.as_ref().map(|v| v.hp).unwrap_or_default()
    }

    fn set_effect(&mut self, effect: StatusEffectInstance) {
        if let Some(u) = self {
            u.effect = Some(effect);
        }
    }

    fn effect(&mut self) -> Option<&mut StatusEffectInstance> {
        self.as_mut().map(|u| u.effect.as_mut()).flatten()
    }

    fn instance(&mut self) -> Option<&mut PokemonInstance> {
        None
    }

    fn view(&self) -> &dyn PokemonView {
        self as _
    }

    fn set_exp(&mut self, _: Experience) {}

    fn exp(&self) -> Experience {
        0
    }
}
