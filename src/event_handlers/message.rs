// Message event handler

use crate::{Data, Error};
use poise::serenity_prelude as serenity;

/// Handle incoming messages (link rewrites, etc.)
pub async fn handle_message(
    _ctx: &serenity::Context,
    _msg: &serenity::Message,
    _data: &Data,
) -> Result<(), Error> {
    // Link rewriters will be implemented here
    Ok(())
}
