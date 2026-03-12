use super::get_pool;
use crate::db::{atomic_increment_goku_poll, get_or_create_goku_poll_usage, get_server_by_guild_id};
use crate::features::Features;
use crate::handlers::gulag::Gulag;
use serenity::{
    all::{CreateMessage, Mentionable},
    client::Context,
    model::channel::Message,
};

pub struct GokuPoll;

impl GokuPoll {
    const MAX_DURATION_SECS: u32 = 2_592_000; // 30 days in seconds

    pub async fn handle_message_update(ctx: &Context, message: &Message) {
        let poll = match &message.poll {
            Some(p) => p,
            None => return,
        };

        // Only process finalized polls
        let results = match &poll.results {
            Some(r) if r.is_finalized => r,
            _ => return,
        };

        let guild_id = match message.guild_id {
            Some(id) => id.get(),
            None => return,
        };

        let pool = get_pool(ctx).await;

        if !Features::is_enabled(&pool, "goku_poll") {
            return;
        }

        // Find the winning answer (highest vote count)
        let winning_answer = match results.answer_counts.iter().max_by_key(|a| a.count) {
            Some(a) if a.count > 0 => a,
            _ => return, // No votes cast
        };

        // Look up the answer text from the poll answers
        let winning_text = match poll
            .answers
            .iter()
            .find(|a| a.answer_id == winning_answer.id)
        {
            Some(answer) => match &answer.poll_media.text {
                Some(text) => text.clone(),
                None => return,
            },
            None => return,
        };

        // Check if the winning answer contains "goku" (case-insensitive)
        if !winning_text.to_lowercase().contains("goku") {
            return;
        }

        let poll_creator = &message.author;

        // Don't gulag the bot
        match Gulag::is_tugbot(&ctx.http, poll_creator).await {
            Some(true) | None => return,
            Some(false) => {}
        }

        let server = match get_server_by_guild_id(&pool, guild_id as i64) {
            Some(s) => s,
            None => {
                eprintln!("Goku poll: server not configured for guild {}", guild_id);
                return;
            }
        };

        // Get current usage count for exponential duration
        let current_count =
            match get_or_create_goku_poll_usage(&pool, poll_creator.id.get() as i64, guild_id as i64)
            {
                Ok(u) => u.usage_count,
                Err(e) => {
                    eprintln!("Goku poll: failed to get usage count: {}", e);
                    return;
                }
            };

        let duration_seconds = Self::calculate_duration(current_count);

        // Find a channel to post in - use the-gulag channel
        let gulag_channel =
            match Gulag::find_channel(&ctx.http, guild_id, "the-gulag".to_string()).await {
                Some(c) => c,
                None => {
                    eprintln!("Goku poll: could not find the-gulag channel");
                    return;
                }
            };

        // Send to gulag
        if let Err(e) = Gulag::add_to_gulag(
            &ctx.http,
            &pool,
            crate::handlers::gulag::GulagParams {
                guildid: guild_id,
                userid: poll_creator.id.get(),
                gulag_roleid: server.gulag_id as u64,
                gulaglength: duration_seconds,
                channelid: gulag_channel.id.get(),
                messageid: message.id.get(),
            },
        )
        .await
        {
            eprintln!("Goku poll: failed to send user to gulag: {}", e);
            return;
        }

        // Increment usage count after successful gulag
        let new_count =
            match atomic_increment_goku_poll(&pool, poll_creator.id.get() as i64, guild_id as i64) {
                Ok(count) => count,
                Err(e) => {
                    eprintln!("Goku poll: failed to increment usage count: {}", e);
                    current_count + 1
                }
            };

        let next_duration_seconds = Self::calculate_duration(new_count);

        let content = format!(
            "{} created a poll and Goku won. Sent to the gulag for {}!\nThis is offense #{} (next offense will be {})",
            poll_creator.mention(),
            Self::format_duration(duration_seconds),
            new_count,
            Self::format_duration(next_duration_seconds),
        );

        let _ = gulag_channel
            .send_message(&ctx.http, CreateMessage::new().content(content))
            .await;
    }

    fn calculate_duration(usage_count: i32) -> u32 {
        // Same formula as AI slop: 1800 * 2^usage_count seconds
        // First offense (count=0): 30 minutes
        // Second offense (count=1): 60 minutes
        // Third offense (count=2): 120 minutes
        // Capped at MAX_DURATION_SECS

        let base_seconds: u64 = 1800; // 30 minutes

        let multiplier = match usage_count.try_into() {
            Ok(count) if count < 32 => 2u64.checked_pow(count).unwrap_or(u64::MAX),
            _ => u64::MAX,
        };

        let duration = base_seconds.saturating_mul(multiplier);

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
