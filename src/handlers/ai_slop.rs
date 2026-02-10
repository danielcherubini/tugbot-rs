use super::HandlerResponse;
use crate::db::{
    establish_connection, get_or_create_ai_slop_usage, get_server_by_guild_id,
    increment_ai_slop_usage,
};
use crate::handlers::gulag::Gulag;
use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::{
        command::CommandType,
        interaction::application_command::ApplicationCommandInteraction,
        Mentionable,
    },
    prelude::Context,
};

pub struct AiSlopHandler;

impl AiSlopHandler {
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
        let usage = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            get_or_create_ai_slop_usage(conn, target_user.id.0 as i64, guild_id as i64)
        })) {
            Ok(usage) => usage,
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
        if let Err(_) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            increment_ai_slop_usage(conn, usage.id, new_count)
        })) {
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
                "{} has been sent to the gulag for {} for posting AI slop: {}\nThis is offense #{} (usage count: {})",
                target_user.mention(),
                Self::format_duration(duration_seconds),
                target_message.link(),
                new_count,
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
        let base_seconds: u32 = 1800; // 30 minutes
        let multiplier = 2u32.pow(usage_count as u32);
        base_seconds * multiplier
    }

    fn format_duration(seconds: u32) -> String {
        let minutes = seconds / 60;
        let hours = minutes / 60;
        let remaining_minutes = minutes % 60;

        if hours > 0 {
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
