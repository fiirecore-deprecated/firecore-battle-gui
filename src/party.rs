use pokedex::{
    battle::party::BattleParty,
    context::PokedexClientContext,
    gui::{party::PartyGui, pokemon::PokemonDisplay},
    pokemon::instance::PokemonInstance,
};

pub fn battle_party_gui<ID: Sized + Copy + core::fmt::Debug + core::fmt::Display + Eq + Ord, A>(
    ctx: &PokedexClientContext,
    gui: &PartyGui,
    party: &BattleParty<ID, A, PokemonInstance>,
    exitable: bool,
) {
    gui.spawn(
        party
            .pokemon
            .iter()
            .cloned()
            .map(|instance| PokemonDisplay::new(ctx, std::borrow::Cow::Owned(instance)))
            .collect(),
        Some(false),
        exitable,
    );
}
