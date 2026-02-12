use regex::Regex;
use serenity::{
    model::{channel::Message, prelude::Member},
    prelude::Context,
};

use crate::features::Features;
use crate::handlers::{get_pool, gulag::Gulag};

pub struct Elon;

impl Elon {
    pub async fn handler(ctx: &Context, msg: &Message) {
        let pool = get_pool(ctx).await;
        if !Features::is_enabled(&pool, "elon") {
            return;
        }

        if Elon::has_elon_words(msg.content.as_str()) {
            let Some(guild_id) = msg.guild_id else {
                return;
            };
            let guildid = guild_id.get();
            match msg.member(&ctx.http).await {
                Err(_) => println!("no partial member"),
                Ok(member) => {
                    println!("{:?}", member);

                    if Elon::has_elon_role(ctx, guildid, &member).await {
                        let channelid = msg.channel_id.get();

                        if let Err(e) = Gulag::send_to_gulag_and_message(
                            &ctx.http,
                            &pool,
                            guildid,
                            member.user.id.get(),
                            channelid,
                            msg.id.get(),
                            None,
                        )
                        .await
                        {
                            eprintln!("Failed to send to gulag: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    pub async fn has_elon_role(ctx: &Context, guildid: u64, member: &Member) -> bool {
        match ctx.http.get_guild_roles(guildid.into()).await {
            Err(_why) => false,
            Ok(roles) => {
                for role in roles {
                    if role.name == "#1ElonMuskFan" {
                        for member_role in member.roles.iter().copied() {
                            if member_role.get() == role.id.get() {
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

        re.captures(&clean.to_lowercase()).is_some()
    }
}
