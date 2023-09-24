use crate::handlers::handlers::HandlerResponse;
use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        prelude::{application_command::CommandDataOptionValue, command::CommandOptionType},
    },
};

use super::Gulag;

pub struct GulagHandler;

impl GulagHandler {
    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command
            .name("gulag")
            .description("Send a user to the Gulag")
            .create_option(|option| {
                option
                    .name("user")
                    .description("The user to lookup")
                    .kind(CommandOptionType::User)
                    .required(true)
            })
            .create_option(|option| {
                option
                    .name("reason")
                    .description("Why Are you sending them")
                    .kind(CommandOptionType::String)
                    .required(true)
            })
            .create_option(|option| {
                option
                    .name("length")
                    .description("How Long minutes")
                    .kind(CommandOptionType::Integer)
                    .required(true)
            });
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        let user_options = command
            .data
            .options
            .get(0)
            .expect("Expected user option")
            .resolved
            .as_ref()
            .expect("Expected user object");
        let reason_options = command
            .data
            .options
            .get(1)
            .expect("Expected reason option")
            .resolved
            .as_ref()
            .expect("Expected reason object");
        let length_options = command
            .data
            .options
            .get(2)
            .expect("Expected length option")
            .resolved
            .as_ref()
            .expect("Expected length object");

        let channelid = command.channel_id.0;

        let mut gulaglength = 300;
        if let CommandDataOptionValue::Integer(length) = length_options {
            gulaglength = length * 60;
        }

        if let CommandDataOptionValue::User(user, _member) = user_options {
            match command.guild_id {
                None => {
                    return HandlerResponse {
                        content: "no member".to_string(),
                        components: None,
                        ephemeral: false,
                    }
                }
                Some(guildid) => match Gulag::find_gulag_role(&ctx.http, *guildid.as_u64()).await {
                    None => {
                        return HandlerResponse {
                            content: "couldn't find gulag role".to_string(),
                            components: None,
                            ephemeral: false,
                        }
                    }
                    Some(gulag_role) => {
                        let gulag_user = Gulag::add_to_gulag(
                            &ctx.http,
                            *guildid.as_u64(),
                            *user.id.as_u64(),
                            *gulag_role.id.as_u64(),
                            gulaglength as u32,
                            channelid,
                        )
                        .await;

                        if let CommandDataOptionValue::String(reason) = reason_options {
                            let content = format!(
                                "Sending {} to the Gulag for {} minutes, because {}",
                                user.to_string(),
                                gulag_user.gulag_length / 60,
                                reason,
                            );
                            return HandlerResponse {
                                content,
                                components: None,
                                ephemeral: false,
                            };
                        } else {
                            let content = format!(
                                "Sending {} to the Gulag for {} minutes",
                                user.to_string(),
                                gulag_user.gulag_length / 60,
                            );
                            return HandlerResponse {
                                content,
                                components: None,
                                ephemeral: false,
                            };
                        }
                    }
                },
            }
        } else {
            return HandlerResponse {
                content: "Please provide a valid user".to_string(),
                components: None,
                ephemeral: false,
            };
        };
    }
}
