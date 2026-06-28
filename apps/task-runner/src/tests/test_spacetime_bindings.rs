use crate::bindings::*;
use spacetimedb_sdk::*;

#[test]
fn main() {
    let ctx = connect_to_db();

    register_callbacks(&ctx);

    subscribe_to_tables(&ctx);

    ctx.run_threaded();

    user_input_loop(&ctx);
}

const HOST: &str = "http://localhost:3000";
const DB_NAME: &str = "yral-database-spacetime-4lbo7";

fn connect_to_db() -> DbConnection {
    DbConnection::builder()
        .on_connect(on_connected)
        .on_connect_error(on_connect_error)
        .on_disconnect(on_disconnected)
        .with_token(creds_store().load().expect("Error loading credentials"))
        .with_database_name(DB_NAME)
        .with_uri(HOST)
        .build()
        .expect("Failed to connect")
}

fn creds_store() -> credentials::File {
    credentials::File::new("yral-database-spacetime-4lbo7")
}

fn on_connected(_ctx: &DbConnection, _identity: Identity, token: &str) {
    if let Err(e) = creds_store().save(token) {
        eprintln!("Failed to save credentials: {:?}", e);
    }
}

fn on_connect_error(_ctx: &ErrorContext, err: Error) {
    eprintln!("Connection error: {:?}", err);
    std::process::exit(1);
}

fn on_disconnected(_ctx: &ErrorContext, err: Option<Error>) {
    if let Some(err) = err {
        eprintln!("Disconnected: {}", err);
        std::process::exit(1);
    } else {
        println!("Disconnected.");
        std::process::exit(0);
    }
}

fn register_callbacks(ctx: &DbConnection) {
    ctx.db.user().on_insert(on_user_inserted);

    ctx.db.user().on_update(on_user_updated);

    ctx.db.message().on_insert(on_message_inserted);
}

fn on_user_inserted(_ctx: &EventContext, user: &User) {
    if user.online {
        println!("User {} connected.", user_name_or_identity(user));
    }
}

fn user_name_or_identity(user: &User) -> String {
    user.name
        .clone()
        .unwrap_or_else(|| user.identity.to_hex().to_string())
}

fn on_user_updated(_ctx: &EventContext, old: &User, new: &User) {
    if old.name != new.name {
        println!(
            "User {} renamed to {}.",
            user_name_or_identity(old),
            user_name_or_identity(new)
        );
    }

    if old.online && !new.online {
        println!("User {} disconnected.", user_name_or_identity(new));
    }

    if !old.online && new.online {
        println!("User {} connected.", user_name_or_identity(new));
    }
}

fn on_message_inserted(ctx: &EventContext, message: &Message) {
    if matches!(ctx.event, Event::Reducer(_) | Event::Transaction) {
        print_message(ctx, message)
    }
}

fn print_message(ctx: &impl RemoteDbContext, message: &Message) {
    let sender = ctx
        .db()
        .user()
        .identity()
        .find(&message.sender.clone())
        .map(|u| user_name_or_identity(&u))
        .unwrap_or_else(|| "unknown".to_string());

    println!("{}: {}", sender, message.text);
}

fn subscribe_to_tables(ctx: &DbConnection) {
    ctx.subscription_builder()
        .on_applied(on_sub_applied)
        .on_error(on_sub_error)
        .add_query(|q| q.from.user())
        .add_query(|q| q.from.message())
        .subscribe();
}

fn on_sub_applied(ctx: &SubscriptionEventContext) {
    let mut messages: Vec<Message> = ctx.db.message().iter().collect();
    messages.sort_by_key(|m| m.sent);
    for message in messages {
        print_message(ctx, &message);
    }
    println!("Fully connected and all subscriptions applied.");
    println!("Use /name to set your name, or type a message!");
}

fn on_sub_error(_ctx: &ErrorContext, err: Error) {
    eprintln!("Subscription failed: {}", err);
    std::process::exit(1);
}

fn user_input_loop(ctx: &DbConnection) {
    for line in std::io::stdin().lines() {
        let Ok(line) = line else {
            panic!("Failed to read from stdin.");
        };
        if let Some(name) = line.strip_prefix("/name ") {
            ctx.reducers
                .set_name_then(name.to_string(), {
                    let name = name.to_string();
                    move |_ctx, result| match result {
                        Err(e) => panic!("Internal error when setting name: {e}"),
                        Ok(Err(e)) => eprintln!("Failed to set name to {name}: {e}"),
                        Ok(Ok(())) => (),
                    }
                })
                .unwrap();
        } else {
            ctx.reducers
                .send_message_then(line.clone(), {
                    move |_ctx, result| match result {
                        Err(e) => panic!("Internal error when sending message: {e}"),
                        Ok(Err(e)) => eprintln!("Failed to send message {line:?}: {e}"),
                        Ok(Ok(())) => (),
                    }
                })
                .unwrap();
        }
    }
}
