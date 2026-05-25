use crate::db::message_vote::MessageVoteHandler;
use crate::features::Features;
use crate::handlers::get_pool;
use serenity::model::prelude::{Emoji, Reaction};

pub struct GulagReaction;

pub enum GulagReactionType {
    ADDED,
    REMOVED,
}

impl GulagReaction {
    pub async fn handler(
        ctx: &serenity::prelude::Context,
        add_reaction: &Reaction,
        _reaction_type: GulagReactionType,
    ) {
        //Match the emoji with the known gulag emoji
        let trigger_emoji = add_reaction.emoji.to_string();
        if trigger_emoji.contains("gulag") {
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

            // Find the :gulag: reaction and get all users who reacted
            let mut gulag_voters: Vec<i64> = Vec::new();
            let mut found_gulag_reaction = false;

            for reaction in &message.reactions {
                let emoji_str = reaction.reaction_type.to_string();
                eprintln!(
                    "[gulag_reaction]   Checking reaction: '{}' (count: {})",
                    emoji_str, reaction.count
                );
                if emoji_str.contains("gulag") {
                    found_gulag_reaction = true;
                    // Get users who reacted with this emoji
                    match ctx
                        .http
                        .get_reaction_users(
                            channel_id.into(),
                            message_id.into(),
                            &reaction.reaction_type,
                            100,  // limit - should be enough for gulag votes
                            None, // after
                        )
                        .await
                    {
                        Ok(users) => {
                            let all_count = users.len();
                            gulag_voters = users
                                .iter()
                                .filter(|u| !u.bot) // Exclude bots from voting
                                .map(|u| u.id.get() as i64)
                                .collect();
                            eprintln!(
                                "[gulag_reaction]   Found gulag reaction: {} users total, {} non-bot voters",
                                all_count,
                                gulag_voters.len()
                            );
                            break;
                        }
                        Err(e) => {
                            eprintln!("Failed to fetch reaction users: {}", e);
                            return;
                        }
                    }
                }
            }

            if !found_gulag_reaction {
                eprintln!(
                    "[gulag_reaction] NO MATCH - trigger='{}', message reactions: {:?}",
                    trigger_emoji,
                    message.reactions.iter().map(|r| r.reaction_type.to_string()).collect::<Vec<_>>()
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
                Ok(vote_data) => println!(
                    "Synced votes for message {}: {} votes",
                    message_id, vote_data.current_vote_tally
                ),
                Err(e) => eprintln!("Error syncing votes: {}", e),
            }
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
