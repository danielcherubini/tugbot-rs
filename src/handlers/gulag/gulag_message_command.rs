use crate::{
    db::{
        establish_connection,
        message_vote::{MessageVoteHanderResponseType, MessageVoteHandler},
    },
    handlers::handlers::HandlerResponse,
};
use serenity::{
    builder::CreateApplicationCommand,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        prelude::command::CommandType,
    },
};

pub struct GulagMessageCommandHandler;

impl GulagMessageCommandHandler {
    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command.name("Add Gulag Vote").kind(CommandType::Message);
    }

    pub async fn setup_interaction(
        _ctx: &serenity::client::Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        let conn = &mut establish_connection();
        let command_data = &command.data;
        let target_id = command_data.target_id.unwrap();
        let message = command_data
            .resolved
            .messages
            .get(&serenity::model::prelude::MessageId(target_id.0))
            .unwrap();

        match MessageVoteHandler::message_vote_create_or_update(
            conn,
            message.id.0,
            command.guild_id.unwrap().0,
            command.channel_id.0,
            message.author.id.0,
            command.user.id.0,
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
