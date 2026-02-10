use super::{get_pool, HandlerResponse};
use crate::db::{atomic_increment_ai_slop, get_or_create_ai_slop_usage, get_server_by_guild_id};
use crate::features::Features;
use crate::handlers::gulag::Gulag;
use serenity::{
    all::{CommandInteraction, CommandType, Mentionable},
    builder::CreateCommand,
    prelude::Context,
};

pub struct AiSlopHandler;

impl AiSlopHandler {
    // Maximum duration: ~30 days (to prevent overflow)
    const MAX_DURATION_SECS: u32 = 2_592_000; // 30 days in seconds

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
        // Fetch roles once instead of three separate API calls
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
        let has_permission = match ctx.http.get_guild_roles(guild_id.into()).await {
            Ok(guild_roles) => {
                let allowed_role_ids: Vec<_> = guild_roles
                    .iter()
                    .filter(|r| allowed_roles.contains(&r.name.as_str()))
                    .map(|r| r.id)
                    .collect();
                member.roles.iter().any(|r| allowed_role_ids.contains(r))
            }
            Err(_) => false,
        };

        if !has_permission {
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

        // Get current usage count (don't increment yet)
        let current_count =
            match get_or_create_ai_slop_usage(&pool, target_user.id.get() as i64, guild_id as i64)
            {
                Ok(u) => u.usage_count,
                Err(_) => {
                    return HandlerResponse {
                        content: "Error: Database error occurred".to_string(),
                        components: None,
                        ephemeral: true,
                    };
                }
            };

        // Calculate duration based on CURRENT usage count (before increment)
        let duration_seconds = Self::calculate_duration(current_count);

        // Send to gulag
        if let Err(e) = Gulag::add_to_gulag(
            &ctx.http,
            &pool,
            crate::handlers::gulag::GulagParams {
                guildid: guild_id,
                userid: target_user.id.get(),
                gulag_roleid: server.gulag_id as u64,
                gulaglength: duration_seconds,
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

        // Only increment if gulag succeeded
        // Use atomic increment to prevent race conditions
        let new_count =
            match atomic_increment_ai_slop(&pool, target_user.id.get() as i64, guild_id as i64) {
                Ok(count) => count,
                Err(_) => {
                    // Gulag succeeded but increment failed - log but don't fail the command
                    eprintln!(
                    "Warning: Successfully added to gulag but failed to increment AI slop count"
                );
                    current_count + 1 // Estimate for display
                }
            };

        // Post notification to #the-gulag channel
        if let Some(gulag_channel) =
            Gulag::find_channel(&ctx.http, guild_id, "the-gulag".to_string()).await
        {
            let channel_message = format!(
                "{} has been sent to the gulag for {} for posting AI slop: {}\nThis is offense #{}",
                target_user.mention(),
                Self::format_duration(duration_seconds),
                target_message.link(),
                new_count
            );

            let _ = gulag_channel.say(&ctx.http, channel_message).await;
        }

        // Calculate next duration for display
        let next_duration_seconds = Self::calculate_duration(new_count);

        HandlerResponse {
            content: format!(
                "Sent {} to the gulag for {} for posting AI slop!\nThis is their offense #{} (next offense will be {})",
                target_user.name,
                Self::format_duration(duration_seconds),
                new_count,
                Self::format_duration(next_duration_seconds)
            ),
            components: None,
            ephemeral: true,
        }
    }

    fn calculate_duration(usage_count: i32) -> u32 {
        // Formula: 1800 * 2^usage_count seconds (30 * 2^usage_count minutes)
        // First offense (count=0): 30 minutes
        // Second offense (count=1): 60 minutes
        // Third offense (count=2): 120 minutes
        // Capped at MAX_DURATION_SECS to prevent overflow

        let base_seconds: u64 = 1800; // 30 minutes

        // Use checked operations to prevent overflow
        let multiplier = match usage_count.try_into() {
            Ok(count) if count < 32 => 2u64.checked_pow(count).unwrap_or(u64::MAX),
            _ => u64::MAX,
        };

        let duration = base_seconds.saturating_mul(multiplier);

        // Clamp to MAX_DURATION_SECS
        duration.min(Self::MAX_DURATION_SECS as u64) as u32
    }

    fn format_duration(seconds: u32) -> String {
        let minutes = seconds / 60;
        let hours = minutes / 60;
        let days = hours / 24;
        let remaining_hours = hours % 24;
        let remaining_minutes = minutes % 60;

        if days > 0 {
            if remaining_hours > 0 {
                format!("{} days {} hours", days, remaining_hours)
            } else {
                format!("{} days", days)
            }
        } else if hours > 0 {
            if remaining_minutes > 0 {
                format!("{} hours {} minutes", hours, remaining_minutes)
            } else {
                format!("{} hours", hours)
            }
        } else {
            format!("{} minutes", minutes)
        }
    }
}
