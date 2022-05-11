use serenity::{
    builder::CreateApplicationCommand, client::Context,
    model::interactions::application_command::ApplicationCommandInteraction,
};

use super::nickname::fix_nickname;

pub struct Horny;

impl Horny {
    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command
            .name("horny")
            .description("Mark yourself as horny/lfg");
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> String {
        let member = command.member.as_ref().unwrap();
        let guild_id = command.guild_id.unwrap();
        let user = &command.user;
        let prefix = String::from("horny");
        let mem = ctx
            .http
            .get_member(*guild_id.as_u64(), *user.id.as_u64())
            .await
            .unwrap();

        match member.nick.as_ref() {
            Some(nick) => {
                let new_nick = fix_nickname(nick, &prefix);
                mem.edit(&ctx.http, |m| m.nickname(new_nick)).await.unwrap();
                return String::from("Done");
            }
            None => {
                let name = member.display_name().to_string();
                let new_nick = fix_nickname(&name, &prefix);

                mem.edit(&ctx.http, |m| m.nickname(new_nick)).await.unwrap();
                return String::from("Done");
            }
        }
    }
}
