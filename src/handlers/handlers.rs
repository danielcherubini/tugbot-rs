use crate::tugbot::servers::Servers;
use serenity::{
    async_trait,
    builder::CreateComponents,
    client::{Context, EventHandler},
    model::{
        channel::Message,
        gateway::Ready,
        id::GuildId,
        interactions::InteractionResponseType,
        prelude::{Interaction, Member},
    },
};

use super::{
    color_handler::ColorHandler, eggmen::Eggmen, elkmen::ElkMen, elon::Elon,
    game_handler::GameHandler, gulag_handler::GulagHandler, horny::Horny, phony::Phony,
    twitter::Twitter,
};

#[derive(Default)]
pub struct HandlerResponse {
    pub content: String,
    pub components: Option<CreateComponents>,
    pub ephemeral: bool,
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // Twitter Changer
    async fn message(&self, ctx: Context, msg: Message) {
        Twitter::handler(&ctx, &msg).await;
        Elon::handler(&ctx, &msg).await;
    }

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
                "color" => ColorHandler::setup_interaction(&ctx, &command).await,
                "game" => GameHandler::setup_interaction(&ctx, &command).await,
                _ => HandlerResponse {
                    content: "Not Implimented".to_string(),
                    components: None,
                    ephemeral: true,
                },
            };

            command
                .create_interaction_response(&ctx.http, |r| {
                    r.kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|i| match handler_response.components {
                            Some(components) => i
                                .content(handler_response.content)
                                .ephemeral(handler_response.ephemeral)
                                .set_components(components),
                            None => i
                                .content(handler_response.content)
                                .ephemeral(handler_response.ephemeral),
                        })
                })
                .await
                .unwrap();

            let response = command.get_interaction_response(&ctx.http).await;

            match response {
                Ok(r) => {
                    let res = r.await_component_interaction(&ctx).await.unwrap();
                    match res.data.custom_id.as_str() {
                        "color_select" => {
                            println!("Do Color Select");
                            ColorHandler::swap_color_role(
                                &ctx,
                                *command.guild_id.unwrap().as_u64(),
                                *command.user.id.as_u64(),
                                res.data.values[0].parse::<u64>().unwrap(),
                            )
                            .await;

                            res.create_interaction_response(&ctx.http, |r| {
                                r.kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|data| {
                                        data.content(format!("OK, done")).ephemeral(true)
                                    })
                            })
                            .await
                            .unwrap()
                        }
                        "game_select" => {
                            println!("Do Game Select");
                            GameHandler::add_or_remove_game_role(
                                &ctx,
                                *command.guild_id.unwrap().as_u64(),
                                *command.user.id.as_u64(),
                                res.data.values[0].parse::<u64>().unwrap(),
                            )
                            .await;

                            res.create_interaction_response(&ctx.http, |r| {
                                r.kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|data| {
                                        data.content(format!("OK, done... if you want to add another game, please ignore the above message and do /game again")).ephemeral(true)
                                    })
                            })
                            .await
                            .unwrap()
                        }
                        _ => {
                            println!("Select custom_id match was missing")
                        }
                    }
                }
                Err(e) => {
                    println!("Cannot respond to slash command: {}", e);
                }
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
                    commands.create_application_command(|command| Eggmen::setup_command(command));
                    commands
                        .create_application_command(|command| ColorHandler::setup_command(command));
                    commands
                        .create_application_command(|command| GameHandler::setup_command(command))
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
