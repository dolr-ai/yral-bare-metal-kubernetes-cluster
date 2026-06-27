use spacetimedb::*;

#[spacetimedb::table(public, accessor = person)]
pub struct Person {
    name: String,
}

#[spacetimedb::reducer]
pub fn add(context: &ReducerContext, name: String) {
    context.db.person().insert(Person { name });
}

#[spacetimedb::reducer]
pub fn say_hello(context: &ReducerContext) {
    for person in context.db.person().iter() {
        log::info!("Hello, {}!", person.name);
    }

    log::info!("Hello, World!");
}
