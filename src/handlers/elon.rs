use serenity::{
    model::{channel::Message, prelude::Member},
    prelude::Context,
};

use crate::handlers::gulag_handler::GulagHandler;

pub struct Elon;

impl Elon {
    pub async fn handler(ctx: &Context, msg: &Message) {
        match msg.content.to_lowercase().as_str() {
            "concerning" | "looking into it" => {
                let guildid = msg.guild_id.unwrap().0;
                let partial_member = msg.member.to_owned().unwrap();
                let member = ctx
                    .http
                    .get_member(guildid, partial_member.user.unwrap().id.0)
                    .await
                    .unwrap();

                if Elon::has_elon_role(&ctx, guildid, &member).await {
                    let gulag_length = 300;
                    let channelid = msg.channel_id.0;

                    match GulagHandler::find_gulag_role(ctx, guildid).await {
                        None => println!("couldn't find gulag id"),
                        Some(gulag_roleid) => {
                            println!("Send to gulag");
                            GulagHandler::add_to_gulag(
                                ctx,
                                guildid,
                                member.user.id.0,
                                gulag_roleid.id.0,
                                gulag_length,
                                channelid,
                            )
                            .await;
                        }
                    }
                }
            }
            _ => return,
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
}
