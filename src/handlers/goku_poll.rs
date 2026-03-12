use super::get_pool;
use crate::db::{
    atomic_increment_goku_poll, get_or_create_goku_poll_usage, get_server_by_guild_id,
};
use crate::features::Features;
use crate::handlers::gulag::Gulag;
use serenity::{
    all::{CreateMessage, Mentionable},
    client::Context,
    model::channel::Message,
};

pub struct GokuPoll;

impl GokuPoll {
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
        let current_count = match get_or_create_goku_poll_usage(
            &pool,
            poll_creator.id.get() as i64,
            guild_id as i64,
        ) {
            Ok(u) => u.usage_count,
            Err(e) => {
                eprintln!("Goku poll: failed to get usage count: {}", e);
                return;
            }
        };

        let duration_seconds = match current_count.try_into() {
            Ok(u32_count) => Gulag::get_gulag_duration_for_offense(u32_count),
            Err(_) => {
                eprintln!("Goku poll: usage count too high for gulag calculation");
                return;
            }
        };

        // Find a channel to post in - use the-gulag channel
        let gulag_channel =
            match Gulag::find_channel(&ctx.http, guild_id, "the-gulag".to_string()).await {
                Some(c) => c,
                None => {
                    eprintln!("Goku poll: could not find the-gulag channel");
                    return;
                }
            };

        // Send to gulag with calculated duration
        if let Err(e) = Gulag::add_to_gulag(
            &ctx.http,
            &pool,
            crate::handlers::gulag::GulagParams {
                guildid: guild_id,
                userid: poll_creator.id.get(),
                gulag_roleid: server.gulag_id as u64,
                gulaglength: duration_seconds.try_into().unwrap_or(u32::MAX),
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
        let new_count = match atomic_increment_goku_poll(
            &pool,
            poll_creator.id.get() as i64,
            guild_id as i64,
        ) {
            Ok(count) => count,
            Err(e) => {
                eprintln!("Goku poll: failed to increment usage count: {}", e);
                current_count + 1
            }
        };

        let next_duration_seconds = match new_count.saturating_add(1).try_into() {
            Ok(u32_count) => Gulag::get_gulag_duration_for_offense(u32_count),
            Err(_) => 2_592_000, // Max ~30 days for overflow protection
        };

        let content = format!(
            "{} created a poll and Goku won. Sent to the gulag for {}!\nThis is offense #{} (next offense will be {})",
            poll_creator.mention(),
            Gulag::format_duration(duration_seconds),
            new_count,
            Gulag::format_duration(next_duration_seconds),
        );

        let _ = gulag_channel
            .send_message(&ctx.http, CreateMessage::new().content(content))
            .await;
    }
}
