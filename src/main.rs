extern crate tugbot;

use diesel::r2d2::ManageConnection;
use serenity::Client;
use tugbot::{
    handlers::handlers::Handler,
    tugbot::{config::Config, servers::PostgresClient},
};

#[tokio::main]
async fn main() {
    let tugbot_config = Config::get_config();
    let connection_manager = diesel::r2d2::ConnectionManager::new(tugbot_config.db_url);
    let _db_client = connection_manager
        .connect()
        .expect("Could not connect to Postgres");

    // Configure the client with your Discord bot token in the environment.
    // The Application Id is usually the Bot User Id.
    // Build our client.
    let mut client = Client::builder(tugbot_config.token, tugbot_config.intents)
        .event_handler(Handler)
        .application_id(tugbot_config.application_id)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<PostgresClient>(connection_manager);
    };

    // Finally, start a single shard, and start listening to events.
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
