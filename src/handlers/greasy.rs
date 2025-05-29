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
    /// Handles a reaction added to a message, triggering a custom response if criteria are met.
    ///
    /// If the "greasy" feature is enabled and a message authored by a user whose name contains "greasy" receives enough "denthead" reactions, this function reacts to the message with a "kekw" emoji and sends a follow-up message in the same channel. The function ensures the bot does not react multiple times to the same message and gracefully handles errors in message retrieval or reaction creation.
    ///
    /// # Examples
    ///
    /// ```
    /// // This function is intended to be called by the Discord event handler when a reaction is added.
    /// // Example usage within an event context:
    /// Greasy::reaction_add_handler(&ctx, &reaction).await;
    /// ```
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

    /// Sends the message "i hate u" to the channel where the reaction was added.
    ///
    /// This function ignores any errors that occur while sending the message.
    ///
    /// # Examples
    ///
    /// ```
    /// // Inside an async context with a valid Context and Reaction:
    /// Greasy::message(&ctx, &reaction).await;
    /// ```
    async fn message(ctx: &Context, incoming_reaction: &Reaction) {
        let message_string = MessageBuilder::new().push("i hate u").build();
        incoming_reaction
            .channel_id
            .say(&ctx.http, message_string)
            .await
            .ok();
    }

    /// Returns true if the provided author name contains the substring "greasy", case-insensitive.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(Greasy::is_greasy("GreasyUser"));
    /// assert!(!Greasy::is_greasy("CleanUser"));
    /// ```
    fn is_greasy(author_name: &str) -> bool {
        author_name.to_lowercase().contains("greasy")
    }

    /// Determines if a message has more than the threshold number of "denthead" or bot reactions.
    ///
    /// Counts the total number of reactions that are either the "denthead" emoji or were added by the bot itself, and returns true if this count exceeds the configured threshold.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::handlers::greasy::{Greasy, has_enough_dents};
    /// use serenity::model::channel::MessageReaction;
    ///
    /// let reactions: Vec<MessageReaction> = vec![/* populate with test reactions */];
    /// let result = Greasy::has_enough_dents(&reactions);
    /// assert!(result == false || result == true);
    /// ```
    pub fn has_enough_dents(reactions: &[MessageReaction]) -> bool {
        let count: u64 = reactions
            .iter()
            .filter(|r| r.reaction_type.to_string() == DENTHEAD || r.me)
            .map(|r| r.count)
            .sum();
        count > MAX_HEADDENTS
    }

    /// Returns true if the bot has already reacted to the message.
    ///
    /// Checks if any reaction in the provided list was made by the bot itself.
    ///
    /// # Examples
    ///
    /// ```
    /// use serenity::model::channel::MessageReaction;
    /// # fn get_reactions() -> Vec<MessageReaction> { vec![] }
    /// let reactions = get_reactions();
    /// let has_bot_reaction = Greasy::already_reacted(&reactions);
    /// ```
    fn already_reacted(reactions: &[MessageReaction]) -> bool {
        reactions.iter().any(|x| x.me)
    }

    /// Attempts to parse the predefined reaction emoji string into a `ReactionType`.
    ///
    /// Returns `Some(ReactionType)` if parsing succeeds, or `None` if the string is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// let emoji = Greasy::get_reaction_emoji();
    /// assert!(emoji.is_some());
    /// ```
    fn get_reaction_emoji() -> Option<ReactionType> {
        ReactionType::try_from(REACTION_STRING).ok()
    }
}
