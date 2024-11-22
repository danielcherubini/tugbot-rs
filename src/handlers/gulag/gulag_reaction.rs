use crate::db::{db, message_vote::MessageVoteHandler};
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
        reaction_type: GulagReactionType,
    ) {
        //Match the emoji with the known gulag emoji
        if add_reaction.emoji.to_string().contains(":gulag") {
            let message_id = add_reaction.message_id.0;
            let voter_id = add_reaction.user_id.unwrap().0;
            let guild_id = add_reaction.guild_id.unwrap().0;
            let channel_id = add_reaction.channel_id.0;

            let user_id = ctx
                .http
                .get_message(channel_id, message_id)
                .await
                .unwrap()
                .author
                .id
                .0;
            let conn = &mut db::establish_connection();
            match reaction_type {
                GulagReactionType::ADDED => {
                    println!("Added");
                    match MessageVoteHandler::message_vote_create_or_update(
                        conn, message_id, guild_id, channel_id, user_id, voter_id,
                    ) {
                        Ok(m) => println!("{:?}", m.content),
                        Err(e) => eprintln!("{}", e),
                    }
                }
                GulagReactionType::REMOVED => {
                    println!("Removed");
                    match MessageVoteHandler::message_vote_remove(conn, message_id, voter_id) {
                        Ok(m) => println!("{:?}", m.content),
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
        }
    }

    pub async fn find_emoji(ctx: &serenity::prelude::Context, guild_id: u64) -> Option<Emoji> {
        let guild_emojis = ctx.http.get_emojis(guild_id).await.unwrap();

        for ge in guild_emojis {
            if ge.name == "gulag" {
                return Some(ge);
            }
        }

        None
    }
}
