pub mod utility;

use crate::{Data, Error};

/// Returns all bot commands
pub fn commands() -> Vec<poise::Command<Data, Error>> {
    let mut cmds = Vec::new();
    cmds.extend(utility::commands());
    cmds
}
