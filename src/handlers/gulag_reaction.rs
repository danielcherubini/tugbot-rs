use serenity::{
    model::prelude::{Emoji, MessageReaction, Reaction, ReactionType},
    prelude::Context,
};

use crate::handlers::gulag_handler::GulagHandler;

pub struct GulagReaction;

impl GulagReaction {
    pub async fn handler(ctx: &Context, add_reaction: &Reaction) {
        let guildid = add_reaction.guild_id.unwrap().0;

        match GulagReaction::find_emoji(ctx, guildid).await {
            None => println!("Couldn't find the gulag emoji"),
            Some(gulag_emoji) => {
                //Match the emoji with the known gulag emoji
                if add_reaction.emoji == ReactionType::from(gulag_emoji.to_owned()) {
                    let channelid = add_reaction.channel_id.0;
                    let messageid = add_reaction.message_id.0;
                    match ctx.http.get_message(channelid, messageid).await {
                        Ok(msg) => {
                            if GulagReaction::can_gulag(msg.reactions, &gulag_emoji) {
                                GulagHandler::send_to_gulag_and_message(
                                    &ctx,
                                    guildid,
                                    msg.author.id.0,
                                    msg.channel_id.0,
                                    msg.id.0,
                                )
                                .await;
                            }
                        }
                        Err(why) => println!("{}", why),
                    }
                }
            }
        }
    }

    pub async fn find_emoji(ctx: &Context, guild_id: u64) -> Option<Emoji> {
        let guild_emojis = ctx.http.get_emojis(guild_id).await.unwrap();

        for ge in guild_emojis {
            if ge.name == "gulag" {
                return Some(ge);
            }
        }

        None
    }

    fn can_gulag(reactions: Vec<MessageReaction>, gulag_emoji: &Emoji) -> bool {
        for react in reactions {
            if react.reaction_type == ReactionType::from(gulag_emoji.to_owned()) {
                //gulag vote is over 0 and also divisible by 5
                if react.count > 0 && react.count % 5 == 0 {
                    return true;
                }
            }
        }
        false
    }
}
