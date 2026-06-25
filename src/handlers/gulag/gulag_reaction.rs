use crate::db::message_vote::MessageVoteHandler;
use crate::features::Features;
use crate::handlers::get_pool;
use serenity::all::User;
use serenity::model::prelude::{Emoji, Reaction, ReactionType};

pub struct GulagReaction;

impl GulagReaction {
    /// Fetch all non-bot users who reacted with the given emoji on a message.
    /// Pages through Discord's `get_reaction_users` API (100 per page max) to
    /// support any vote count, not just the first 100.
    async fn fetch_all_voters(
        ctx: &serenity::prelude::Context,
        channel_id: u64,
        message_id: u64,
        reaction_type: &ReactionType,
    ) -> Result<Vec<i64>, String> {
        const PAGE_SIZE: u8 = 100;
        // Safety cap. 50 pages × 100 = 5000 voters, which is more than any
        // real Discord message has ever accumulated. Protects against a
        // buggy or hostile Discord state returning full pages forever.
        const MAX_PAGES: u8 = 50;
        let mut all_users: Vec<User> = Vec::new();
        let mut after: Option<u64> = None;

        for page_num in 0..MAX_PAGES {
            let page = ctx
                .http
                .get_reaction_users(
                    channel_id.into(),
                    message_id.into(),
                    reaction_type,
                    PAGE_SIZE,
                    after,
                )
                .await
                .map_err(|e| e.to_string())?;
            let last_id = page.last().map(|u| u.id.get());
            let is_last = page.len() < PAGE_SIZE as usize;
            all_users.extend(page);
            if is_last {
                break;
            }
            // Pagination cursor: pass the last user ID to get the next page.
            let Some(id) = last_id else { break };
            after = Some(id);

            // Warn if we're about to hit the cap on the next iteration.
            if page_num + 1 == MAX_PAGES {
                eprintln!(
                    "[gulag_reaction] WARNING: hit MAX_PAGES={} for message {}, aborting pagination",
                    MAX_PAGES, message_id
                );
            }
        }

        Ok(all_users
            .iter()
            .filter(|u| !u.bot) // Exclude bots from voting
            .map(|u| u.id.get() as i64)
            .collect())
    }

    pub async fn handler(ctx: &serenity::prelude::Context, add_reaction: &Reaction) {
        // Match the emoji with the known gulag emoji
        let trigger_emoji = add_reaction.emoji.to_string();
        if !trigger_emoji.contains("gulag") {
            return;
        }

        let pool = get_pool(ctx).await;

        // Check if gulag feature is enabled
        if !Features::is_enabled(&pool, "gulag") {
            return;
        }

        let guild_id = match add_reaction.guild_id {
            Some(id) => id.get(),
            None => return, // Not in a guild context
        };

        let message_id = add_reaction.message_id.get();
        let channel_id = add_reaction.channel_id.get();

        // Fetch the message to get actual reaction data from Discord
        let message = match ctx
            .http
            .get_message(channel_id.into(), message_id.into())
            .await
        {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Failed to fetch message {}: {}", message_id, e);
                return;
            }
        };

        let user_id = message.author.id.get();

        eprintln!(
            "[gulag_reaction] Message {} has {} reaction(s)",
            message_id,
            message.reactions.len()
        );

        // Find the :gulag: reaction and get all users who reacted (paginated)
        let mut gulag_voters: Vec<i64> = Vec::new();
        let mut found_gulag_reaction = false;

        for reaction in &message.reactions {
            let emoji_str = reaction.reaction_type.to_string();
            eprintln!(
                "[gulag_reaction]   Checking reaction: '{}' (count: {})",
                emoji_str, reaction.count
            );
            if !emoji_str.contains("gulag") {
                continue;
            }
            found_gulag_reaction = true;
            match Self::fetch_all_voters(ctx, channel_id, message_id, &reaction.reaction_type).await
            {
                Ok(voters) => {
                    eprintln!(
                        "[gulag_reaction]   Found gulag reaction: {} non-bot voters",
                        voters.len()
                    );
                    gulag_voters = voters;
                    break;
                }
                Err(e) => {
                    eprintln!("Failed to fetch reaction users: {}", e);
                    return;
                }
            }
        }

        if !found_gulag_reaction {
            eprintln!(
                "[gulag_reaction] NO MATCH - trigger='{}', message reactions: {:?}",
                trigger_emoji,
                message
                    .reactions
                    .iter()
                    .map(|r| r.reaction_type.to_string())
                    .collect::<Vec<_>>()
            );
        }

        // Sync database with actual Discord reaction data
        match MessageVoteHandler::sync_from_discord(
            &pool,
            message_id,
            guild_id,
            channel_id,
            user_id,
            gulag_voters,
        ) {
            Ok(vote_data) => eprintln!(
                "[gulag_reaction] Synced votes for message {}: {} votes",
                message_id, vote_data.current_vote_tally
            ),
            Err(e) => eprintln!("Error syncing votes: {}", e),
        }
    }

    pub async fn find_emoji(ctx: &serenity::prelude::Context, guild_id: u64) -> Option<Emoji> {
        let guild_emojis = ctx.http.get_emojis(guild_id.into()).await.ok()?;

        for ge in guild_emojis {
            if ge.name == "gulag" {
                return Some(ge);
            }
        }

        None
    }
}
