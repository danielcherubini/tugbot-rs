use crate::tugbot::server::Server;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        gateway::Ready, id::GuildId, interactions::InteractionResponseType, prelude::Interaction,
    },
};

use super::{gulag::Gulag, horny::Horny, phony::Phony};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "gulag" => Gulag::setup_interaction(&ctx, &command).await,
                "phony" => Horny::setup_interaction(&ctx, &command).await,
                "horny" => Phony::setup_interaction(&ctx, &command).await,
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
            let commands =
                GuildId::set_application_commands(&server.guild_id, &ctx.http, |commands| {
                    commands.create_application_command(|command| Gulag::setup_command(command));
                    commands.create_application_command(|command| Horny::setup_command(command));
                    commands.create_application_command(|command| Phony::setup_command(command))
                })
                .await;

            println!(
                "I now have the following guild slash commands: {:#?}",
                commands
            );
        }
    }
}
