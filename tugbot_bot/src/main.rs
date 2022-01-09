extern crate tugbot_lib;

use serenity::{
    async_trait,
    model::{
        gateway::Ready,
        id::GuildId,
        interactions::{
            application_command::{
                ApplicationCommandInteractionDataOptionValue, 
                ApplicationCommandOptionType,
            },
            Interaction, 
            InteractionResponseType,
        },
    },
    prelude::*,
};
use tugbot_lib::tugbot::{
    config::Config, 
    server::Server
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "gulag" => {
                    let options = command
                        .data
                        .options
                        .get(0)
                        .expect("Expected user option")
                        .resolved
                        .as_ref()
                        .expect("Expected user object");

                    if let ApplicationCommandInteractionDataOptionValue::User(user, _member) =
                        options
                    {
                        format!("Sending {} to the Gulag", user.tag())
                    } else {
                        "Please provide a valid user".to_string()
                    }
                }
                _ => "not implemented :(".to_string(),
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                })
                .await
            {
                println!("Cannot respond to slash command: {}", why);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let servers = Server::get_servers(&ctx).await;
        
        for server in servers {
            let _commands =
                GuildId::set_application_commands(&server.guild_id, &ctx.http, |commands| {
                    commands.create_application_command(|command| {
                        command
                            .name("gulag")
                            .description("Send a user to the Gulag")
                            .create_option(|option| {
                                option
                                    .name("user")
                                    .description("The user to lookup")
                                    .kind(ApplicationCommandOptionType::User)
                                    .required(true)
                            })
                    })
                })
                .await;
        }
    }
}

#[tokio::main]
async fn main() {

    let tugbot_config = Config::get_config(); 

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
