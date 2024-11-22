use super::{
    bsky, feat,
    gulag::{
        gulag, gulag_handler, gulag_list_handler, gulag_message_command,
        gulag_reaction::{self, GulagReactionType},
        gulag_remove_handler,
    },
    horny, instagram, phony, teh, twitter,
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
        prelude::{Interaction, Member, Reaction},
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
        teh::handler(&ctx, &msg).await;
        twitter::handler(&ctx, &msg).await;
        //tikTok::handler(&ctx, &msg).await;
        bsky::handler(&ctx, &msg).await;
        instagram::handler(&ctx, &msg).await;
    }

    async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
        gulag_reaction::handler(&ctx, &add_reaction, GulagReactionType::ADDED).await;
    }

    async fn reaction_remove(&self, ctx: Context, add_reaction: Reaction) {
        gulag_reaction::handler(&ctx, &add_reaction, GulagReactionType::REMOVED).await;
    }

    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        if let Some(user) = gulag::is_user_in_gulag(*member.user.id.as_u64()) {
            gulag::add_to_gulag(
                &ctx.http,
                user.guild_id as u64,
                user.user_id as u64,
                user.gulag_role_id as u64,
                user.gulag_length as u32,
                user.channel_id as u64,
                0,
            )
            .await;

            let message = format!("You can't escape so easly {}", member);
            let channel = ctx.http.get_channel(user.channel_id as u64).await.unwrap();
            channel
                .id()
                .send_message(ctx.http, |m| m.content(message))
                .await
                .unwrap();
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let handler_response = match command.data.name.as_str() {
                "gulag" => gulag_handler::setup_interaction(&ctx, &command).await,
                "gulag-release" => gulag_remove_handler::setup_interaction(&ctx, &command).await,
                "gulag-list" => gulag_list_handler::setup_interaction(&ctx, &command).await,
                "Add Gulag Vote" => gulag_message_command::setup_interaction(&ctx, &command).await,
                "phony" => horny::setup_interaction(&ctx, &command).await,
                "horny" => phony::setup_interaction(&ctx, &command).await,
                "feature" => feat::setup_interaction(&command).await,
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
                    if let Some(msg) = res.interaction.to_owned() {
                        if msg.name.as_str() == "gulag-vote" {
                            // GulagVoteHandler::do_followup(&ctx, &command, res).await
                        }
                    }
                }
                Err(why) => println!("Cannot respond to slash command: {}", why),
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let servers = Servers::get_servers(&ctx).await;
        gulag::run_gulag_check(&ctx.http);
        gulag::run_gulag_vote_check(&ctx.http);

        for server in servers {
            let commands = GuildId::set_application_commands(&server.guild_id, &ctx.http, |c| {
                c.create_application_command(|command| gulag_handler::setup_command(command));
                c.create_application_command(|command| {
                    gulag_remove_handler::setup_command(command)
                });
                c.create_application_command(|command| gulag_list_handler::setup_command(command));
                c.create_application_command(|command| {
                    gulag_message_command::setup_command(command)
                });
                c.create_application_command(|command| horny::setup_command(command));
                c.create_application_command(|command| phony::setup_command(command));
                c.create_application_command(|command| feat::setup_command(command))
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
