use serenity::{
    all::CommandInteraction,
    builder::{CreateCommand, EditMember},
    client::Context,
};

use super::{nickname::fix_nickname, HandlerResponse};
use crate::features::Features;

pub struct Horny;

impl Horny {
    pub fn setup_command() -> CreateCommand {
        CreateCommand::new("horny").description("Mark yourself as horny/lfg")
    }

    pub async fn setup_interaction(ctx: &Context, command: &CommandInteraction) -> HandlerResponse {
        if !Features::is_enabled("horny") {
            return HandlerResponse {
                content: String::from("This feature is currently disabled"),
                components: None,
                ephemeral: true,
            };
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

        let mut mem = match ctx.http.get_member(guild_id, user.id).await {
            Ok(m) => m,
            Err(_) => {
                return HandlerResponse {
                    content: String::from("Error: Could not fetch member"),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        match member.nick.as_ref() {
            Some(nick) => {
                let new_nick = fix_nickname(nick, prefix);
                if let Err(why) = mem
                    .edit(&ctx.http, EditMember::new().nickname(new_nick))
                    .await
                {
                    return HandlerResponse {
                        content: format!("Error: Could not update nickname: {}", why),
                        components: None,
                        ephemeral: true,
                    };
                }
                HandlerResponse {
                    content: String::from("Done"),
                    components: None,
                    ephemeral: true,
                }
            }
            None => {
                let name = member.display_name().to_string();
                let new_nick = fix_nickname(&name, prefix);

                if let Err(why) = mem
                    .edit(&ctx.http, EditMember::new().nickname(new_nick))
                    .await
                {
                    return HandlerResponse {
                        content: format!("Error: Could not update nickname: {}", why),
                        components: None,
                        ephemeral: true,
                    };
                }
                HandlerResponse {
                    content: String::from("Done"),
                    components: None,
                    ephemeral: true,
                }
            }
        }
    }
}
