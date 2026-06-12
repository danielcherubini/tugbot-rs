use serenity::Client;
use std::sync::Arc;
use tugbot::{
    db::establish_pool,
    handlers::{ConfigKey, DbPoolKey, Handler},
    tugbot::config::Config,
};

#[tokio::main]
async fn main() {
    let tugbot_config = Arc::new(Config::get_config());

    // Initialize the database connection pool
    eprintln!("Initializing database connection pool...");
    let pool = establish_pool();
    eprintln!("Database connection pool established");

    // Configure the client with your Discord bot token in the environment.
    // The Application Id is usually the Bot User Id.
    // Build our client.
    let mut client = Client::builder(tugbot_config.token.clone(), tugbot_config.intents)
        .event_handler(Handler)
        .application_id(tugbot_config.application_id.into())
        .await
        .expect("Error creating client");

    // Insert the database pool and config into the client's data
    {
        let mut data = client.data.write().await;
        data.insert::<DbPoolKey>(pool);
        data.insert::<ConfigKey>(tugbot_config);
    }

    // Finally, start a single shard, and start listening to events.
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        eprintln!("Client error: {:?}", why);
    }
}
