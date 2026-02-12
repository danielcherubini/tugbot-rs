use crate::{
    db::message_vote::{MessageVoteHanderResponseType, MessageVoteHandler},
    handlers::{get_pool, HandlerResponse},
};
use serenity::{
    all::{CommandInteraction, CommandType},
    builder::CreateCommand,
};

pub struct GulagMessageCommandHandler;

impl GulagMessageCommandHandler {
    pub fn setup_command() -> CreateCommand {
        CreateCommand::new("Add Gulag Vote").kind(CommandType::Message)
    }

    pub async fn setup_interaction(
        ctx: &serenity::client::Context,
        command: &CommandInteraction,
    ) -> HandlerResponse {
        let pool = get_pool(ctx).await;

        if !crate::features::Features::is_enabled(&pool, "gulag") {
            return HandlerResponse {
                content: "Gulag feature is currently disabled.".to_string(),
                components: None,
                ephemeral: true,
            };
        }
        let command_data = &command.data;

        let Some(target_id) = command_data.target_id else {
            return HandlerResponse {
                content: "No target message found.".to_string(),
                components: None,
                ephemeral: true,
            };
        };

        let Some(message) = command_data
            .resolved
            .messages
            .get(&serenity::model::prelude::MessageId::new(target_id.get()))
        else {
            return HandlerResponse {
                content: "Could not resolve target message.".to_string(),
                components: None,
                ephemeral: true,
            };
        };

        let Some(guild_id) = command.guild_id else {
            return HandlerResponse {
                content: "This command can only be used in a server.".to_string(),
                components: None,
                ephemeral: true,
            };
        };

        match MessageVoteHandler::message_vote_create_or_update(
            &pool,
            message.id.get(),
            guild_id.get(),
            command.channel_id.get(),
            message.author.id.get(),
            command.user.id.get(),
        ) {
            Ok(message_vote) => {
                let content = match message_vote.response_type {
                    MessageVoteHanderResponseType::ADDED => {
                        format!(
                            "A gulag vote has been added to {}\nThere are now {} unique votes total",
                            message.link(),
                            message_vote.content.current_vote_tally
                        )
                    }
                    MessageVoteHanderResponseType::REMOVED => {
                        format!(
                            "A gulag vote has been removed from {}\nThere are now {} unique votes total",
                            message.link(),
                            message_vote.content.current_vote_tally
                        )
                    }
                };
                HandlerResponse {
                    content,
                    components: None,
                    ephemeral: true,
                }
            }
            Err(err) => HandlerResponse {
                content: err.to_string(),
                components: None,
                ephemeral: true,
            },
        }
    }
}
