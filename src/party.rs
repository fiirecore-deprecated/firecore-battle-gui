use pokedex::{
    context::PokedexClientContext,
    gui::{party::PartyGui, pokemon::PokemonDisplay},
    pokemon::PokemonParty,
};

pub fn battle_party_gui(
    ctx: &PokedexClientContext,
    gui: &PartyGui,
    party: &PokemonParty,
    exitable: bool,
) {
    gui.spawn(
        party
            .iter()
            .cloned()
            .map(|instance| PokemonDisplay::new(ctx, std::borrow::Cow::Owned(instance)))
            .collect(),
        Some(false),
        exitable,
    );
}
