use spacetimedb::*;

#[spacetimedb::table(accessor = character, public)]
pub struct Character {
    #[primary_key]
    player_id: Identity,
    #[unique]
    nickname: String,
    level: u32,
    class: Class,
}

#[derive(SpacetimeType, Debug, Copy, Clone)]
pub enum Class {
    Fighter,
    Caster,
    Medic,
}

#[spacetimedb::reducer]
fn create_character(ctx: &ReducerContext, class: Class, nickname: String) {
    log::info!("Creating new level 1 {class:?} named {nickname}");
}
