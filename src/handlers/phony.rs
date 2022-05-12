use serenity::{
    builder::CreateApplicationCommand, client::Context,
    model::interactions::application_command::ApplicationCommandInteraction,
};

use super::{handlers::HandlerResponse, nickname::fix_nickname};

pub struct Phony;

impl Phony {
    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command
            .name("phony")
            .description("Mark yourself as phony/watching");
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        let member = command.member.as_ref().unwrap();
        let guild_id = command.guild_id.unwrap();
        let user = &command.user;
        let prefix = String::from("phony");
        let mem = ctx
            .http
            .get_member(*guild_id.as_u64(), *user.id.as_u64())
            .await
            .unwrap();

        match member.nick.as_ref() {
            Some(nick) => {
                let new_nick = fix_nickname(nick, &prefix);
                mem.edit(&ctx.http, |m| m.nickname(new_nick)).await.unwrap();
                return HandlerResponse {
                    content: String::from("Done"),
                    ephemeral: true,
                };
            }
            None => {
                let name = member.display_name().to_string();
                let new_nick = fix_nickname(&name, &prefix);

                mem.edit(&ctx.http, |m| m.nickname(new_nick)).await.unwrap();
                return HandlerResponse {
                    content: String::from("Done"),
                    ephemeral: true,
                };
            }
        }
    }
}
