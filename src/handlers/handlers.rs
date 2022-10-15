use crate::tugbot::servers::Servers;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        gateway::Ready,
        id::GuildId,
        interactions::InteractionResponseType,
        prelude::{Interaction, Member},
    },
};

use super::{
    eggmen::Eggmen, elkmen::ElkMen, gulag_handler::GulagHandler, horny::Horny, phony::Phony,
};

pub struct HandlerResponse {
    pub content: String,
    pub ephemeral: bool,
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        match GulagHandler::is_user_in_gulag(*member.user.id.as_u64()) {
            Some(user) => {
                GulagHandler::add_to_gulag(
                    &ctx,
                    user.guild_id as u64,
                    user.user_id as u64,
                    user.gulag_role_id as u64,
                    user.gulag_length as u32,
                    user.channel_id as u64,
                )
                .await;

                let message = format!("You can't escape so easly {}", member.to_string());
                let channel = ctx.http.get_channel(user.channel_id as u64).await.unwrap();
                channel
                    .id()
                    .send_message(ctx.http, |m| m.content(message))
                    .await
                    .unwrap();
            }
            None => {}
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let handler_response = match command.data.name.as_str() {
                "gulag" => GulagHandler::setup_interaction(&ctx, &command).await,
                "phony" => Horny::setup_interaction(&ctx, &command).await,
                "horny" => Phony::setup_interaction(&ctx, &command).await,
                "elk-invite" => ElkMen::setup_interaction(&ctx, &command).await,
                "egg-invite" => Eggmen::setup_interaction(&ctx, &command).await,
                _ => HandlerResponse {
                    content: "not implemented :(".to_string(),
                    ephemeral: true,
                },
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .content(handler_response.content)
                                .ephemeral(handler_response.ephemeral)
                        })
                })
                .await
            {
                println!("Cannot respond to slash command: {}", why);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let servers = Servers::get_servers(&ctx).await;
        GulagHandler::run_gulag_check(&ctx);

        for server in servers {
            let commands =
                GuildId::set_application_commands(&server.guild_id, &ctx.http, |commands| {
                    commands
                        .create_application_command(|command| GulagHandler::setup_command(command));
                    commands.create_application_command(|command| Horny::setup_command(command));
                    commands.create_application_command(|command| Phony::setup_command(command));
                    commands.create_application_command(|command| ElkMen::setup_command(command));
                    commands.create_application_command(|command| Eggmen::setup_command(command))
                })
                .await;

            println!("I now have the following guild slash commands: ",);
            match commands {
                Ok(commandvec) => {
                    for command in commandvec {
                        println!("{}", command.name)
                    }
                }
                Err(e) => println!("{}", e),
            }
        }
    }
}
