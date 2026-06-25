//! Generic `/<prefix>` handler that toggles a `<prefix> | <nick>` style prefix
//! on the calling member's nickname. Shared by the "horny" and "phony" commands.

use serenity::{
    all::CommandInteraction,
    builder::{CreateCommand, EditMember},
    client::Context,
};

use super::{get_pool, HandlerResponse};
use crate::features::Features;

fn clean_username(nick: &str) -> String {
    nick.replace("phony | ", "").replace("horny | ", "")
}

/// Build the new nickname by adding (or removing) `<prefix> | ` to the start.
/// When the prefix is already present, it's stripped (toggle behavior).
fn fix_nickname(nick: &str, prefix: &str) -> String {
    let nick_to_find = format!("{} | ", prefix);
    if nick.contains(&nick_to_find) {
        clean_username(nick)
    } else if nick.contains(" | ") {
        format!("{} | {}", prefix, clean_username(nick))
    } else {
        format!("{} | {}", prefix, nick)
    }
}

pub struct PrefixHandler;

impl PrefixHandler {
    pub fn setup_command(name: &'static str, description: &'static str) -> CreateCommand {
        CreateCommand::new(name).description(description)
    }

    pub async fn setup_interaction(ctx: &Context, command: &CommandInteraction) -> HandlerResponse {
        let pool = get_pool(ctx).await;
        let prefix = command.data.name.clone();

        // Check feature flag
        match Features::check_enabled(&pool, &prefix) {
            Ok(true) => {}
            Ok(false) => {
                return HandlerResponse {
                    content: String::from("This feature is currently disabled"),
                    components: None,
                    ephemeral: true,
                    defer_response: None,
                };
            }
            Err(e) => {
                eprintln!("[{}] Failed to check feature status: {}", prefix, e);
                return HandlerResponse {
                    content: String::from(
                        "Error: Could not connect to the database. Please try again later.",
                    ),
                    components: None,
                    ephemeral: true,
                    defer_response: None,
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
                    defer_response: None,
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
                    defer_response: None,
                };
            }
        };
        let user = &command.user;

        let current_nick = member.nick.as_deref().unwrap_or(member.display_name());
        let new_nick = fix_nickname(current_nick, &prefix);
        let was_already_prefixed = current_nick.contains(&format!("{} | ", prefix));
        let action_word = if was_already_prefixed {
            "Removed"
        } else {
            "Added"
        };

        let mut mem = match ctx.http.get_member(guild_id, user.id).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[{}] Failed to fetch member: {}", prefix, e);
                return HandlerResponse {
                    content: String::from(
                        "Error: Could not fetch your member info. Please try again later.",
                    ),
                    components: None,
                    ephemeral: true,
                    defer_response: None,
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
                defer_response: None,
            },
            Err(e) => {
                eprintln!("[{}] Failed to update nickname: {}", prefix, e);
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
                    defer_response: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fix_nickname;

    #[test]
    fn horny() {
        let nick = String::from("foo");
        let prefix = String::from("horny");
        let positive_test = fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("horny | foo"));
    }
    #[test]
    fn phony() {
        let nick = String::from("foo");
        let prefix = String::from("phony");
        let positive_test = fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("phony | foo"));
    }
    #[test]
    fn swap() {
        let nick = String::from("horny | foo");
        let prefix = String::from("phony");
        let positive_test = fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("phony | foo"));
    }

    #[test]
    fn nickname_clean_one() {
        let nick = String::from("horny | foo");
        let prefix = String::from("horny");
        let positive_test = fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("foo"));
    }
    #[test]
    fn nickname_clean_all() {
        let nick = String::from("phony | horny | foo");
        let prefix = String::from("phony");
        let positive_test = fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("foo"));
    }

    #[test]
    fn empty_nickname() {
        let nick = String::from("");
        let prefix = String::from("horny");
        let result = fix_nickname(&nick, &prefix);
        assert_eq!(result, String::from("horny | "));
    }

    #[test]
    fn nickname_with_multiple_pipes() {
        let nick = String::from("other | prefix | username");
        let prefix = String::from("horny");
        let result = fix_nickname(&nick, &prefix);
        assert_eq!(result, String::from("horny | other | prefix | username"));
    }

    #[test]
    fn nickname_already_has_correct_prefix() {
        let nick = String::from("phony | username");
        let prefix = String::from("phony");
        let result = fix_nickname(&nick, &prefix);
        assert_eq!(result, String::from("username"));
    }
}
