use super::{get_pool, HandlerResponse};
use crate::db::{atomic_increment_ai_slop, get_server_by_guild_id};
use crate::features::Features;
use crate::handlers::gulag::Gulag;
use serenity::{
    all::{CommandInteraction, CommandType, Mentionable},
    builder::CreateCommand,
    prelude::Context,
};

pub struct AiSlopHandler;

impl AiSlopHandler {
    pub fn setup_command() -> CreateCommand {
        CreateCommand::new("AI Slop")
            .kind(CommandType::Message)
            .description("")
    }

    pub async fn setup_interaction(ctx: &Context, command: &CommandInteraction) -> HandlerResponse {
        let pool = get_pool(ctx).await;

        // Check feature flag
        if !Features::is_enabled(&pool, "ai_slop") {
            return HandlerResponse {
                content: "This feature is currently disabled.".to_string(),
                components: None,
                ephemeral: true,
            };
        }

        let guild_id = match command.guild_id {
            Some(id) => id.get(),
            None => {
                return HandlerResponse {
                    content: "Error: This command can only be used in a guild".to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        // Check permissions: require Highly Regarded or admin role
        let member = match ctx.http.get_member(guild_id.into(), command.user.id).await {
            Ok(m) => m,
            Err(_) => {
                return HandlerResponse {
                    content: "Error: Could not verify your permissions".to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        let allowed_roles = ["Highly Regarded", "admin"];
        if !Gulag::member_has_any_role(&ctx.http, guild_id, &member, &allowed_roles).await {
            return HandlerResponse {
                content: "Error: You need Highly Regarded or admin role to use this command"
                    .to_string(),
                components: None,
                ephemeral: true,
            };
        }

        // Extract target message from command data
        let target_message = match command.data.resolved.messages.values().next() {
            Some(msg) => msg,
            None => {
                return HandlerResponse {
                    content: "Error: Could not find target message".to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        let target_user = &target_message.author;

        // Prevent self-slop
        if target_user.id.get() == command.user.id.get() {
            return HandlerResponse {
                content: "Error: You cannot AI Slop yourself!".to_string(),
                components: None,
                ephemeral: true,
            };
        }

        // Prevent targeting the bot
        match Gulag::is_tugbot(&ctx.http, target_user).await {
            Some(true) => {
                return HandlerResponse {
                    content: "Error: You cannot AI Slop the bot!".to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
            Some(false) => {} // Continue
            None => {
                return HandlerResponse {
                    content: "Error: Could not verify bot status".to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
        }

        // Get server info from database
        let server = match get_server_by_guild_id(&pool, guild_id as i64) {
            Some(s) => s,
            None => {
                return HandlerResponse {
                    content:
                        "Error: This server is not configured. Please ensure a gulag role exists."
                            .to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        // Increment usage count first, then calculate duration for next offense
        let new_count =
            match atomic_increment_ai_slop(&pool, target_user.id.get() as i64, guild_id as i64) {
                Ok(count) => count,
                Err(_) => {
                    return HandlerResponse {
                        content: "Error: Failed to record AI slop usage".to_string(),
                        components: None,
                        ephemeral: true,
                    };
                }
            };

        // Calculate duration for the offense that just occurred (new_count - 1)
        let duration_seconds = match new_count.saturating_sub(1).try_into() {
            Ok(u32_count) => Gulag::get_gulag_duration_for_offense(u32_count),
            Err(_) => {
                return HandlerResponse {
                    content: "Error: Usage count too high for gulag calculation".to_string(),
                    components: None,
                    ephemeral: true,
                }
            }
        };

        // Send to gulag with calculated duration
        if let Err(e) = Gulag::add_to_gulag(
            &ctx.http,
            &pool,
            crate::handlers::gulag::GulagParams {
                guildid: guild_id,
                userid: target_user.id.get(),
                gulag_roleid: server.gulag_id as u64,
                gulaglength: duration_seconds.try_into().unwrap_or(u32::MAX),
                channelid: command.channel_id.get(),
                messageid: target_message.id.get(),
            },
        )
        .await
        {
            eprintln!("Failed to send user to gulag: {}", e);
            return HandlerResponse {
                content: format!("Error: Failed to send to gulag: {}", e),
                components: None,
                ephemeral: true,
            };
        }

        // Post notification to #the-gulag channel
        if let Some(gulag_channel) =
            Gulag::find_channel(&ctx.http, guild_id, "the-gulag".to_string()).await
        {
            let channel_message = format!(
                "{} has been sent to the gulag for {} for posting AI slop: {}\nThis is offense #{}",
                target_user.mention(),
                Gulag::format_duration(duration_seconds),
                target_message.link(),
                new_count
            );

            let _ = gulag_channel.say(&ctx.http, channel_message).await;
        }

        HandlerResponse {
            content: format!(
                "Sent {} to the gulag for {} for posting AI slop!\nThis is their offense #{} (next offense will be {})",
                target_user.name,
                Gulag::format_duration(duration_seconds),
                new_count,
                match new_count.try_into() {
                    Ok(u32_count) => Gulag::format_duration(Gulag::get_gulag_duration_for_offense(u32_count)),
                    Err(_) => Gulag::format_duration(2_592_000), // Max ~30 days for overflow protection
                }
            ),
            components: None,
            ephemeral: true,
        }
    }
}
