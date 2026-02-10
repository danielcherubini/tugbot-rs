use super::HandlerResponse;
use crate::db::{
    establish_connection, get_or_create_ai_slop_usage, get_server_by_guild_id,
    increment_ai_slop_usage,
};
use crate::features::Features;
use crate::handlers::gulag::Gulag;
use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::{
        command::CommandType,
        interaction::application_command::ApplicationCommandInteraction, Mentionable,
    },
    prelude::Context,
};

pub struct AiSlopHandler;

impl AiSlopHandler {
    // Maximum duration: ~30 days (to prevent overflow)
    const MAX_DURATION_SECS: u32 = 2_592_000; // 30 days in seconds

    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        command
            .name("AI Slop")
            .kind(CommandType::Message)
            .description("")
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        // Check feature flag
        if !Features::is_enabled("ai_slop") {
            return HandlerResponse {
                content: "This feature is currently disabled.".to_string(),
                components: None,
                ephemeral: true,
            };
        }

        let guild_id = match command.guild_id {
            Some(id) => id.0,
            None => {
                return HandlerResponse {
                    content: "Error: This command can only be used in a guild".to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        // Check permissions: require moderator, admin, or derpies role
        let member = match ctx.http.get_member(guild_id, command.user.id.0).await {
            Ok(m) => m,
            Err(_) => {
                return HandlerResponse {
                    content: "Error: Could not verify your permissions".to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        let is_moderator = Gulag::member_has_role(&ctx.http, guild_id, &member, "moderator").await;
        let is_admin = Gulag::member_has_role(&ctx.http, guild_id, &member, "admin").await;
        let is_derpies = Gulag::member_has_role(&ctx.http, guild_id, &member, "derpies").await;

        if !is_moderator && !is_admin && !is_derpies {
            return HandlerResponse {
                content: "Error: You need moderator, admin, or derpies role to use this command"
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
        if target_user.id.0 == command.user.id.0 {
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
        let conn = &mut establish_connection();
        let server = match get_server_by_guild_id(conn, guild_id as i64) {
            Some(s) => s,
            None => {
                return HandlerResponse {
                    content: "Error: This server is not configured. Please ensure a gulag role exists."
                        .to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        // Get or create usage record
        let usage = match get_or_create_ai_slop_usage(conn, target_user.id.0 as i64, guild_id as i64)
        {
            Ok(u) => u,
            Err(_) => {
                return HandlerResponse {
                    content: "Error: Database error occurred".to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        // Calculate duration based on CURRENT usage count (before increment)
        let duration_seconds = Self::calculate_duration(usage.usage_count);

        // Increment usage count for next time
        let new_count = usage.usage_count + 1;
        if let Err(_) = increment_ai_slop_usage(conn, usage.id, new_count) {
            return HandlerResponse {
                content: "Error: Could not update usage count".to_string(),
                components: None,
                ephemeral: true,
            };
        }

        // Send to gulag using the gulag role ID from database
        let _gulag_user = Gulag::add_to_gulag(
            &ctx.http,
            guild_id,
            target_user.id.0,
            server.gulag_id as u64,
            duration_seconds,
            command.channel_id.0,
            target_message.id.0,
        )
        .await;

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
