// Event handlers for non-command events (messages, reactions, etc.)

pub mod message;
pub mod reaction;

use crate::Data;
use poise::serenity_prelude as serenity;
use serenity::async_trait;

/// Event handler struct that implements Serenity's EventHandler trait
pub struct Handler {
    /// Reference to the Poise framework for accessing bot data
    pub data: Data,
}

impl Handler {
    pub fn new(data: Data) -> Self {
        Self { data }
    }
}

#[async_trait]
impl serenity::EventHandler for Handler {
    async fn message(&self, ctx: serenity::Context, msg: serenity::Message) {
        if let Err(e) = message::handle_message(&ctx, &msg, &self.data).await {
            eprintln!("Error handling message: {}", e);
        }
    }

    async fn reaction_add(&self, ctx: serenity::Context, reaction: serenity::Reaction) {
        if let Err(e) = reaction::handle_reaction_add(&ctx, &reaction, &self.data).await {
            eprintln!("Error handling reaction add: {}", e);
        }
    }

    async fn reaction_remove(&self, ctx: serenity::Context, reaction: serenity::Reaction) {
        if let Err(e) = reaction::handle_reaction_remove(&ctx, &reaction, &self.data).await {
            eprintln!("Error handling reaction remove: {}", e);
        }
    }

    async fn guild_member_addition(&self, ctx: serenity::Context, member: serenity::Member) {
        // Check if user was in gulag and re-apply role
        if let Err(e) = crate::services::gulag::handle_member_rejoin(&ctx, &member, &self.data).await
        {
            eprintln!("Error handling member rejoin: {}", e);
        }
    }
}
