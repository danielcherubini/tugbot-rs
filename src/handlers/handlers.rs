use super::{
    elon::Elon,
    gulag::gulag_handler::GulagHandler,
    gulag::gulag_list_handler::GulagListHandler,
    gulag::{gulag_remove_handler::GulagRemoveHandler, gulag_vote::GulagVoteHandler, Gulag},
    horny::Horny,
    phony::Phony,
    twitter::Twitter,
};
use crate::tugbot::servers::Servers;
use serenity::{
    async_trait,
    builder::CreateComponents,
    client::{Context, EventHandler},
    model::{
        application::interaction::InteractionResponseType,
        channel::Message,
        gateway::Ready,
        id::GuildId,
        prelude::{Interaction, Member},
    },
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
    async fn message(&self, ctx: Context, msg: Message) {
        Twitter::handler(&ctx, &msg).await;
        Elon::handler(&ctx, &msg).await;
    }

    // async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
    //     GulagReaction::handler(&ctx, &add_reaction).await;
    // }

    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        match Gulag::is_user_in_gulag(*member.user.id.as_u64()) {
            Some(user) => {
                Gulag::add_to_gulag(
                    &ctx.http,
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
                "gulag-release" => GulagRemoveHandler::setup_interaction(&ctx, &command).await,
                "gulag-list" => GulagListHandler::setup_interaction(&ctx, &command).await,
                "gulag-vote" => GulagVoteHandler::setup_interaction(&ctx, &command).await,
                "phony" => Horny::setup_interaction(&ctx, &command).await,
                "horny" => Phony::setup_interaction(&ctx, &command).await,
                _ => HandlerResponse {
                    content: "Not Implimented".to_string(),
                    components: None,
                    ephemeral: true,
                },
            };

            match command
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
            {
                Ok(()) => {
                    let res = command.get_interaction_response(&ctx.http).await.unwrap();
                    match res.interaction.to_owned() {
                        Some(msg) => match msg.name.as_str() {
                            "gulag-vote" => {
                                GulagVoteHandler::do_followup(&ctx, &command, res).await
                            }
                            _ => {}
                        },
                        None => {}
                    }
                }
                Err(why) => println!("Cannot respond to slash command: {}", why),
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let servers = Servers::get_servers(&ctx).await;
        Gulag::run_gulag_check(&ctx.http);
        Gulag::run_gulag_vote_check(&ctx.http);

        for server in servers {
            let commands = GuildId::set_application_commands(&server.guild_id, &ctx.http, |c| {
                c.create_application_command(|command| GulagHandler::setup_command(command));
                c.create_application_command(|command| GulagRemoveHandler::setup_command(command));
                c.create_application_command(|command| GulagListHandler::setup_command(command));
                c.create_application_command(|command| GulagVoteHandler::setup_command(command));
                c.create_application_command(|command| Horny::setup_command(command));
                c.create_application_command(|command| Phony::setup_command(command))
            })
            .await;

            println!("I now have the following guild slash commands:");
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
