use crate::{handlers::gulag::Gulag, tugbot::server::Server};
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        gateway::Ready, id::GuildId, interactions::InteractionResponseType, prelude::Interaction,
    },
};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "gulag" => Gulag::setup_interaction(&ctx, &command).await,
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
                    commands.create_application_command(|command| Gulag::setup_command(command))
                })
                .await;
        }
    }
}
