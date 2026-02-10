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

        let member = command.member.as_ref().unwrap();
        let guild_id = command.guild_id.unwrap();
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
                mem.edit(&ctx.http, EditMember::new().nickname(new_nick))
                    .await
                    .unwrap();
                HandlerResponse {
                    content: String::from("Done"),
                    components: None,
                    ephemeral: true,
                }
            }
            None => {
                let name = member.display_name().to_string();
                let new_nick = fix_nickname(&name, prefix);

                mem.edit(&ctx.http, EditMember::new().nickname(new_nick))
                    .await
                    .unwrap();
                HandlerResponse {
                    content: String::from("Done"),
                    components: None,
                    ephemeral: true,
                }
            }
        }
    }
}
