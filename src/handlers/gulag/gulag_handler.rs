use crate::features::Features;
use crate::handlers::{get_pool, HandlerResponse};
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
        let pool = get_pool(ctx).await;
        if !Features::is_enabled(&pool, "gulag") {
            return HandlerResponse {
                content: String::from("Gulag feature is currently disabled"),
                components: None,
                ephemeral: true,
            };
        }

        // Check permissions: require Highly Regarded or admin role
        let guild_id = match command.guild_id {
            Some(id) => id.get(),
            None => return HandlerResponse {
                content: "Error: This command can only be used in a guild".to_string(),
                components: None,
                ephemeral: true,
            },
        };

        let member = match ctx.http.get_member(guild_id.into(), command.user.id).await {
            Ok(m) => m,
            Err(_) => return HandlerResponse {
                content: "Error: Could not verify your permissions".to_string(),
                components: None,
                ephemeral: true,
            },
        };

        let allowed_roles = ["Highly Regarded", "admin"];
        if !Gulag::member_has_any_role(&ctx.http, guild_id, &member, &allowed_roles).await {
            return HandlerResponse {
                content: "Error: You need Highly Regarded or admin role to use this command".to_string(),
                components: None,
                ephemeral: true,
            };
        }

        let user_options = match command.data.options.first() {
            Some(opt) => &opt.value,
            None => return HandlerResponse {
                content: "Error: Missing required user option".to_string(),
                components: None,
                ephemeral: true,
            },
        };

        let reason_options = match command.data.options.get(1) {
            Some(opt) => &opt.value,
            None => return HandlerResponse {
                content: "Error: Missing required reason option".to_string(),
                components: None,
                ephemeral: true,
            },
        };

        let length_options = match command.data.options.get(2) {
            Some(opt) => &opt.value,
            None => return HandlerResponse {
                content: "Error: Missing required length option".to_string(),
                components: None,
                ephemeral: true,
            },
        };

        let channelid = command.channel_id.get();

        let mut gulaglength = 300;
        if let CommandDataOptionValue::Integer(length) = length_options {
            if *length > 0 && *length <= 10080 {
                // Max 1 week
                gulaglength = length * 60;
            } else if *length <= 0 {
                return HandlerResponse {
                    content: String::from("Gulag length must be positive"),
                    components: None,
                    ephemeral: true,
                };
            } else {
                return HandlerResponse {
                    content: String::from("Gulag length cannot exceed 10080 minutes (1 week)"),
                    components: None,
                    ephemeral: true,
                };
            }
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
                        let gulag_user = match Gulag::add_to_gulag(
                            &ctx.http,
                            &pool,
                            super::GulagParams {
                                guildid: guildid.get(),
                                userid: user.get(),
                                gulag_roleid: gulag_role.id.get(),
                                gulaglength: gulaglength as u32,
                                channelid,
                                messageid: 0,
                            },
                        )
                        .await
                        {
                            Ok(u) => u,
                            Err(e) => {
                                return HandlerResponse {
                                    content: format!("Failed to send to gulag: {}", e),
                                    components: None,
                                    ephemeral: true,
                                };
                            }
                        };

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
