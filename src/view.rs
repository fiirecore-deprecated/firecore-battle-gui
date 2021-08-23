use pokedex::{ailment::LiveAilment, pokemon::{Experience, Health, Level, OwnedRefPokemon, PokemonRef}};

use battle::{
    party::PlayerParty,
    player::PlayerKnowable,
    pokemon::{PokemonView, battle::InitUnknownPokemon},
};

type Active = usize;
type PartyIndex = usize;

#[deprecated]
pub trait PlayerView<'d, ID> {
    fn id(&self) -> &ID;

    fn name(&self) -> &str;

    fn active(&self, active: Active) -> Option<&dyn GuiPokemonView<'d>>;

    fn active_mut(&mut self, active: Active) -> Option<&mut dyn GuiPokemonView<'d>>;

    fn active_eq(&self, active: Active, index: &Option<PartyIndex>) -> bool;

    fn pokemon(&self, index: PartyIndex) -> Option<&dyn GuiPokemonView<'d>>;

    fn replace(&mut self, active: Active, new: Option<PartyIndex>);
}

impl<'d, ID, P: GuiPokemonView<'d>> PlayerView<'d, ID> for PlayerKnowable<ID, P> {
    fn id(&self) -> &ID {
        &self.id
    }

    fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("Unknown")
    }

    fn active(&self, active: usize) -> Option<&dyn GuiPokemonView<'d>> {
        PlayerParty::active(self, active).map(|p| p as _)
    }

    fn active_mut(&mut self, active: usize) -> Option<&mut dyn GuiPokemonView<'d>> {
        PlayerParty::active_mut(self, active).map(|p| p as _)
    }

    fn active_eq(&self, active: usize, index: &Option<usize>) -> bool {
        self.active
            .get(active)
            .map(|i| i == index)
            .unwrap_or_default()
    }

    fn pokemon(&self, index: usize) -> Option<&dyn GuiPokemonView<'d>> {
        self.pokemon.get(index).map(|p| p as _)
    }

    fn replace(&mut self, active: usize, new: Option<usize>) {
        PlayerParty::replace(self, active, new)
    }

}

pub trait GuiPokemonView<'d>: PokemonView {

    fn pokemon(&self) -> PokemonRef<'d>;

    fn name(&self) -> &str;

    fn set_level(&mut self, level: Level);
    fn level(&self) -> Level;

    fn set_hp(&mut self, hp: f32);
    fn hp(&self) -> f32;

    fn set_ailment(&mut self, effect: LiveAilment);
    fn ailment(&mut self) -> Option<&mut LiveAilment>;

    fn set_exp(&mut self, experience: Experience);

    fn instance(&mut self) -> Option<&mut OwnedRefPokemon<'d>>;

    fn exp(&self) -> Experience;
}

impl<'d> GuiPokemonView<'d> for OwnedRefPokemon<'d> {

    fn pokemon(&self) -> PokemonRef<'d> {
        self.pokemon
    }

    fn name(&self) -> &str {
        OwnedRefPokemon::name(self)
    }

    fn set_level(&mut self, level: Level) {
        self.level = level;
    }

    fn level(&self) -> Level {
        self.level
    }

    fn set_hp(&mut self, hp: f32) {
        self.hp = (hp.max(0.0) * self.max_hp() as f32) as Health
    }

    fn hp(&self) -> f32 {
        self.percent_hp()
    }

    fn set_ailment(&mut self, ailment: LiveAilment) {
        self.ailment = Some(ailment);
    }

    fn ailment(&mut self) -> Option<&mut LiveAilment> {
        self.ailment.as_mut()
    }

    fn set_exp(&mut self, experience: Experience) {
        self.experience = experience;
    }

    fn instance(&mut self) -> Option<&mut OwnedRefPokemon<'d>> {
        Some(self)
    }

    fn exp(&self) -> Experience {
        self.experience
    }
}

impl<'d> GuiPokemonView<'d> for Option<InitUnknownPokemon<'d>> {

    fn pokemon(&self) -> PokemonRef<'d> {
        match self {
            Some(u) => u.pokemon,
            None => todo!(),
        }
    }

    fn name(&self) -> &str {
        match self {
            Some(u) => u.name(),
            None => "Unknown",
        }
    }

    fn set_level(&mut self, level: Level) {
        if let Some(u) = self.as_mut() {
            u.level = level;
        }
    }

    fn level(&self) -> Level {
        self.as_ref().map(|u| u.level).unwrap_or_default()
    }

    fn set_hp(&mut self, hp: f32) {
        if let Some(u) = self.as_mut() {
            u.hp = hp;
        }
    }

    fn hp(&self) -> f32 {
        self.as_ref().map(|v| v.hp).unwrap_or_default()
    }

    fn set_ailment(&mut self, ailment: LiveAilment) {
        if let Some(u) = self {
            u.ailment = Some(ailment);
        }
    }

    fn ailment(&mut self) -> Option<&mut LiveAilment> {
        self.as_mut().map(|u| u.ailment.as_mut()).flatten()
    }

    fn instance(&mut self) -> Option<&mut OwnedRefPokemon<'d>> {
        None
    }

    fn set_exp(&mut self, _: Experience) {}

    fn exp(&self) -> Experience {
        0
    }
}
