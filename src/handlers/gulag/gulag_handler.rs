use crate::handlers::HandlerResponse;
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption},
    client::Context,
};

use super::Gulag;

pub struct GulagHandler;

impl GulagHandler {
    pub fn setup_command() -> CreateCommand {
        CreateCommand::new("gulag")
            .description("Send a user to the Gulag")
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "The user to lookup")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "reason",
                    "Why Are you sending them",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "length", "How Long minutes")
                    .required(true),
            )
    }

    pub async fn setup_interaction(ctx: &Context, command: &CommandInteraction) -> HandlerResponse {
        let user_options = &command
            .data
            .options
            .first()
            .expect("Expected user option")
            .value;
        let reason_options = &command
            .data
            .options
            .get(1)
            .expect("Expected reason option")
            .value;
        let length_options = &command
            .data
            .options
            .get(2)
            .expect("Expected length option")
            .value;

        let channelid = command.channel_id.get();

        let mut gulaglength = 300;
        if let CommandDataOptionValue::Integer(length) = length_options {
            gulaglength = length * 60;
        }

        if let CommandDataOptionValue::User(user) = user_options {
            match command.guild_id {
                None => HandlerResponse {
                    content: "no member".to_string(),
                    components: None,
                    ephemeral: false,
                },
                Some(guildid) => match Gulag::find_gulag_role(&ctx.http, guildid.get()).await {
                    None => HandlerResponse {
                        content: "couldn't find gulag role".to_string(),
                        components: None,
                        ephemeral: false,
                    },
                    Some(gulag_role) => {
                        let gulag_user = Gulag::add_to_gulag(
                            &ctx.http,
                            guildid.get(),
                            user.get(),
                            gulag_role.id.get(),
                            gulaglength as u32,
                            channelid,
                            0,
                        )
                        .await;

                        if let CommandDataOptionValue::String(reason) = reason_options {
                            let content = format!(
                                "Sending {} to the Gulag for {} minutes, because {}",
                                user,
                                gulag_user.gulag_length / 60,
                                reason,
                            );
                            HandlerResponse {
                                content,
                                components: None,
                                ephemeral: false,
                            }
                        } else {
                            let content = format!(
                                "Sending {} to the Gulag for {} minutes",
                                user,
                                gulag_user.gulag_length / 60,
                            );
                            HandlerResponse {
                                content,
                                components: None,
                                ephemeral: false,
                            }
                        }
                    }
                },
            }
        } else {
            HandlerResponse {
                content: "Please provide a valid user".to_string(),
                components: None,
                ephemeral: false,
            }
        }
    }
}
