extern crate tugbot;

use celery::prelude::*;
use serenity::Client;
use tugbot::{handlers::handlers::Handler, tugbot::config::Config};

#[tokio::main]
async fn main() {
    let tugbot_config = Congig::get_config();

    let my_app = celery::app!(
        broker = RedisBroker { tugbot_config.redis },
        tasks = [remove_from_gulag],
        task_routes = [],
    );

    // Configure the client with your Discord bot token in the environment.
    // The Application Id is usually the Bot User Id.
    // Build our client.
    let mut client = Client::builder(tugbot_config.token)
        .event_handler(Handler)
        .application_id(tugbot_config.application_id)
        .await
        .expect("Error creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
