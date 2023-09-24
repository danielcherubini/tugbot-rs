use regex::Regex;
use serenity::{
    model::{channel::Message, prelude::Member},
    prelude::Context,
};

use crate::handlers::gulag::Gulag;

pub struct Elon;

impl Elon {
    pub async fn handler(ctx: &Context, msg: &Message) {
        if Elon::has_elon_words(msg.content.as_str()) {
            let guildid = msg.guild_id.unwrap().0;
            match msg.member(&ctx.http).await {
                Err(_) => println!("no partial member"),
                Ok(member) => {
                    println!("{:?}", member);

                    if Elon::has_elon_role(&ctx, guildid, &member).await {
                        let channelid = msg.channel_id.0;

                        Gulag::send_to_gulag_and_message(
                            &ctx.http,
                            guildid,
                            member.user.id.0,
                            channelid,
                            msg.id.0,
                            None,
                        )
                        .await
                        .unwrap();
                    }
                }
            }
        }
    }

    pub async fn has_elon_role(ctx: &Context, guildid: u64, member: &Member) -> bool {
        match ctx.http.get_guild_roles(guildid).await {
            Err(_why) => false,
            Ok(roles) => {
                for role in roles {
                    if role.name == "#1ElonMuskFan" {
                        for member_role in member.roles.to_owned() {
                            if member_role.0 == role.id.0 {
                                return true;
                            }
                        }
                    }
                }
                false
            }
        }
    }

    fn has_elon_words(msg: &str) -> bool {
        let english = unidecode::unidecode(msg);

        let re = Regex::new(r"(concerning|looking.{0,}into.{0,}it)").unwrap();

        // Clean out all special characters
        let clean =
            Regex::replace_all(&Regex::new(r"[^a-zA-Z0-9 ]").unwrap(), english.as_str(), "");

        match re.captures(&clean.to_lowercase()) {
            None => false,
            Some(_) => true,
        }
    }
}
