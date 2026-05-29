use serenity::{
    all::CommandInteraction,
    builder::{CreateCommand, EditMember},
    client::Context,
};

use super::{get_pool, nickname::fix_nickname, HandlerResponse};
use crate::features::Features;

pub struct Horny;

impl Horny {
    pub fn setup_command() -> CreateCommand {
        CreateCommand::new("horny").description("Mark yourself as horny/lfg")
    }

    pub async fn setup_interaction(ctx: &Context, command: &CommandInteraction) -> HandlerResponse {
        let pool = get_pool(ctx).await;

        // Check feature flag with proper error reporting
        match Features::check_enabled(&pool, "horny") {
            Ok(true) => {}
            Ok(false) => {
                return HandlerResponse {
                    content: String::from("This feature is currently disabled"),
                    components: None,
                    ephemeral: true,
                };
            }
            Err(e) => {
                eprintln!("Failed to check horny feature status: {}", e);
                return HandlerResponse {
                    content: String::from(
                        "Error: Could not connect to the database. Please try again later.",
                    ),
                    components: None,
                    ephemeral: true,
                };
            }
        }

        let member = match command.member.as_ref() {
            Some(m) => m,
            None => {
                return HandlerResponse {
                    content: String::from("Error: This command can only be used in a server"),
                    components: None,
                    ephemeral: true,
                };
            }
        };
        let guild_id = match command.guild_id {
            Some(id) => id,
            None => {
                return HandlerResponse {
                    content: String::from("Error: This command can only be used in a server"),
                    components: None,
                    ephemeral: true,
                };
            }
        };
        let user = &command.user;
        let prefix = &command.data.name;

        let current_nick = member.nick.as_deref().unwrap_or(member.display_name());
        let new_nick = fix_nickname(current_nick, prefix);

        // Determine if we're adding or removing the prefix
        let was_already_prefixed = current_nick.contains(&format!("{} | ", prefix));
        let action_word = if was_already_prefixed {
            "Removed"
        } else {
            "Added"
        };

        let mut mem = match ctx.http.get_member(guild_id, user.id).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to fetch member for horny command: {}", e);
                return HandlerResponse {
                    content: String::from(
                        "Error: Could not fetch your member info. Please try again later.",
                    ),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        match mem
            .edit(&ctx.http, EditMember::new().nickname(new_nick.clone()))
            .await
        {
            Ok(_) => HandlerResponse {
                content: format!("{} | {} your nickname", action_word, prefix),
                components: None,
                ephemeral: true,
            },
            Err(e) => {
                eprintln!("Failed to update nickname for horny command: {}", e);
                // Provide more specific error messages for common cases
                let error_msg = if e.to_string().contains("Missing Permissions") {
                    "Error: I don't have permission to change nicknames. Please check my role permissions.".to_string()
                } else if e.to_string().contains("Cannot exceed the limit") {
                    "Error: Nickname is too long. Please shorten your nickname first.".to_string()
                } else {
                    format!("Error: Could not update nickname: {}", e)
                };
                HandlerResponse {
                    content: error_msg,
                    components: None,
                    ephemeral: true,
                }
            }
        }
    }
}
