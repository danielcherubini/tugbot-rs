// Reaction event handlers

use crate::{Data, Error};
use poise::serenity_prelude as serenity;

/// Handle reaction add events (gulag voting, etc.)
pub async fn handle_reaction_add(
    _ctx: &serenity::Context,
    _reaction: &serenity::Reaction,
    _data: &Data,
) -> Result<(), Error> {
    // Gulag reaction voting will be implemented here
    Ok(())
}

/// Handle reaction remove events
pub async fn handle_reaction_remove(
    _ctx: &serenity::Context,
    _reaction: &serenity::Reaction,
    _data: &Data,
) -> Result<(), Error> {
    // Gulag reaction voting will be implemented here
    Ok(())
}
