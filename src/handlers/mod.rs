// pub mod elkmen;
pub mod ai_slop;
pub mod bsky;
pub mod derpies;
pub mod elon;
pub mod feat;
pub mod gulag;
pub mod horny;
pub mod instagram;
pub mod nickname;
pub mod phony;
pub mod teh;
pub mod tiktok;
pub mod twitter;

use crate::handlers::{
    ai_slop::AiSlopHandler,
    bsky::Bsky,
    feat::Feat,
    gulag::{
        gulag_handler::GulagHandler,
        gulag_list_handler::GulagListHandler,
        gulag_message_command::GulagMessageCommandHandler,
        gulag_reaction::{GulagReaction, GulagReactionType},
        gulag_remove_handler::GulagRemoveHandler,
        Gulag,
    },
    horny::Horny,
    phony::Phony,
    teh::Teh,
    twitter::Twitter,
};
use crate::tugbot::servers::Servers;
use instagram::Instagram;
use serenity::{
    all::{Interaction, Member, Message, Reaction, Ready},
    async_trait,
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage},
    client::{Context, EventHandler},
};

#[derive(Default)]
pub struct HandlerResponse {
    pub content: String,
    pub components: Option<Vec<serenity::all::CreateActionRow>>,
    pub ephemeral: bool,
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        Teh::handler(&ctx, &msg).await;
        Twitter::handler(&ctx, &msg).await;
        //TikTok::handler(&ctx, &msg).await;
        Bsky::handler(&ctx, &msg).await;
        Instagram::handler(&ctx, &msg).await;
    }

    async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
        GulagReaction::handler(&ctx, &add_reaction, GulagReactionType::ADDED).await;
    }

    async fn reaction_remove(&self, ctx: Context, add_reaction: Reaction) {
        GulagReaction::handler(&ctx, &add_reaction, GulagReactionType::REMOVED).await;
    }

    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        if let Some(user) = Gulag::is_user_in_gulag(member.user.id.get()) {
            Gulag::add_to_gulag(
                &ctx.http,
                user.guild_id as u64,
                user.user_id as u64,
                user.gulag_role_id as u64,
                user.gulag_length as u32,
                user.channel_id as u64,
                0,
            )
            .await;

            let message = format!("You can't escape so easily {}", member);
            if let Ok(channel) = ctx.http.get_channel((user.channel_id as u64).into()).await {
                if let Err(why) = channel
                    .id()
                    .send_message(&ctx.http, CreateMessage::new().content(message))
                    .await
                {
                    println!("Failed to send gulag escape message: {}", why);
                }
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let handler_response = match command.data.name.as_str() {
                "gulag" => GulagHandler::setup_interaction(&ctx, &command).await,
                "gulag-release" => GulagRemoveHandler::setup_interaction(&ctx, &command).await,
                "gulag-list" => GulagListHandler::setup_interaction(&ctx, &command).await,
                "Add Gulag Vote" => {
                    GulagMessageCommandHandler::setup_interaction(&ctx, &command).await
                }
                "AI Slop" => AiSlopHandler::setup_interaction(&ctx, &command).await,
                "phony" => Horny::setup_interaction(&ctx, &command).await,
                "horny" => Phony::setup_interaction(&ctx, &command).await,
                "feature" => Feat::setup_interaction(&command).await,
                _ => HandlerResponse {
                    content: "Not Implimented".to_string(),
                    components: None,
                    ephemeral: true,
                },
            };

            let mut message = CreateInteractionResponseMessage::new()
                .content(handler_response.content)
                .ephemeral(handler_response.ephemeral);
            if let Some(components) = handler_response.components {
                message = message.components(components);
            }

            match command
                .create_response(&ctx.http, CreateInteractionResponse::Message(message))
                .await
            {
                Ok(()) => {}
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
            let commands = server
                .guild_id
                .set_commands(
                    &ctx.http,
                    vec![
                        GulagHandler::setup_command(),
                        GulagRemoveHandler::setup_command(),
                        GulagListHandler::setup_command(),
                        GulagMessageCommandHandler::setup_command(),
                        AiSlopHandler::setup_command(),
                        Horny::setup_command(),
                        Phony::setup_command(),
                        Feat::setup_command(),
                    ],
                )
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
