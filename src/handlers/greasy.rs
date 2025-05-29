use serenity::{
    model::{
        channel::{MessageReaction, Reaction},
        prelude::ReactionType,
    },
    prelude::Context,
    utils::MessageBuilder,
};

use crate::features::Features;

const MAX_HEADDENTS: u64 = 3;
const REACTION_STRING: &str = "<:kekw:841395318674292767>";
const DENTHEAD: &str = "<:denthead:1377314774219489520>";

pub struct Greasy;
impl Greasy {
    pub async fn reaction_add_handler(ctx: &Context, incoming_reaction: &Reaction) {
        if !Features::is_enabled("greasy") {
            return;
        };
        let ch_id = incoming_reaction.channel_id.into();
        let m_id = incoming_reaction.message_id.into();

        // get the message
        let msg = match ctx.http.get_message(ch_id, m_id).await {
            Ok(msg) => msg,
            Err(err) => {
                println!("Failed to fetch message: {:?}", err);
                return;
            }
        };

        if !Greasy::is_greasy(&msg.author.name) {
            return;
        }

        // see if we already reacted
        if Greasy::already_reacted(&msg.reactions) {
            return;
        }

        // check if we have enough dents
        if !Greasy::has_enough_dents(&msg.reactions) {
            return;
        }

        // get the reaction emoji
        let emoji_reaction = match Greasy::get_reaction_emoji() {
            Some(reaction_as_emoji) => reaction_as_emoji,
            None => return,
        };
        // if we have enough dents, send kekw reaction and message
        match ctx.http.create_reaction(ch_id, m_id, &emoji_reaction).await {
            Ok(_) => Greasy::message(ctx, incoming_reaction).await,
            Err(err) => {
                println!("Failed to react: {:?}", err);
            }
        }
    }

    async fn message(ctx: &Context, incoming_reaction: &Reaction) {
        let message_string = MessageBuilder::new().push("i hate u").build();
        incoming_reaction
            .channel_id
            .say(&ctx.http, message_string)
            .await
            .ok();
    }

    fn is_greasy(author_name: &str) -> bool {
        author_name.to_lowercase().contains("greasy")
    }

    pub fn has_enough_dents(reactions: &[MessageReaction]) -> bool {
        let count: u64 = reactions
            .iter()
            .filter(|r| r.reaction_type.to_string() == DENTHEAD || r.me)
            .map(|r| r.count)
            .sum();
        count > MAX_HEADDENTS
    }

    fn already_reacted(reactions: &[MessageReaction]) -> bool {
        reactions.iter().any(|x| x.me)
    }

    fn get_reaction_emoji() -> Option<ReactionType> {
        ReactionType::try_from(REACTION_STRING).ok()
    }
}
