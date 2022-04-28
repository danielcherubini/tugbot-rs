use serenity::{
    builder::CreateApplicationCommand, client::Context,
    model::interactions::application_command::ApplicationCommandInteraction,
};

pub struct Phony;

fn fix_nickname(nick: &String) -> String {
    if let Some(_result) = nick.find("phony") {
        let nicks: Vec<&str> = nick.split("| ").collect();
        return nicks[1].to_string();
    } else {
        return format!("phony | {}", nick);
    }
}

impl Phony {
    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command
            .name("phony")
            .description("Mark yourself as phony/watching");
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> String {
        let member = command.member.as_ref().unwrap();
        let guild_id = command.guild_id.unwrap();
        let user = &command.user;
        let mem = ctx
            .http
            .get_member(*guild_id.as_u64(), *user.id.as_u64())
            .await
            .unwrap();

        match member.nick.as_ref() {
            Some(nick) => {
                let new_nick = fix_nickname(nick);
                mem.edit(&ctx.http, |m| m.nickname(new_nick)).await.unwrap();
                return format!("Done now you are {}", nick);
            }
            None => {
                let name = member.display_name().to_string();
                let new_nick = fix_nickname(&name);
                println!("{}", new_nick);

                mem.edit(&ctx.http, |m| m.nickname(new_nick)).await.unwrap();
                return String::from("Missing user thing");
            }
        }
    }
}