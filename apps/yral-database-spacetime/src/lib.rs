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
    ctx.db.character().insert(Character {
        player_id: ctx.sender(),
        nickname,
        level: 1,
        class,
    });
}

fn find_character_for_player(ctx: &ReducerContext) -> Character {
    ctx.db
        .character()
        .player_id()
        .find(ctx.sender())
        .expect("Player has not created a character")
}

fn update_character(ctx: &ReducerContext, character: Character) {
    ctx.db.character().player_id().update(character);
}
