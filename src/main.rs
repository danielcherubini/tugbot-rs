use serenity::Client;
use tugbot::{db::establish_pool, handlers::{DbPoolKey, Handler}, tugbot::config::Config};

#[tokio::main]
async fn main() {
    let tugbot_config = Config::get_config();

    // Initialize the database connection pool
    println!("Initializing database connection pool...");
    let pool = establish_pool();
    println!("Database connection pool established");

    // Configure the client with your Discord bot token in the environment.
    // The Application Id is usually the Bot User Id.
    // Build our client.
    let mut client = Client::builder(tugbot_config.token, tugbot_config.intents)
        .event_handler(Handler)
        .application_id(tugbot_config.application_id.into())
        .await
        .expect("Error creating client");

    // Insert the database pool into the client's data
    {
        let mut data = client.data.write().await;
        data.insert::<DbPoolKey>(pool);
    }

    // Finally, start a single shard, and start listening to events.
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
