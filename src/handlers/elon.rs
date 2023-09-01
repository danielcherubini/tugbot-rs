use serenity::{
    model::{channel::Message, prelude::PartialMember},
    prelude::Context,
};

use crate::handlers::gulag_handler::GulagHandler;

pub struct Elon;

impl Elon {
    pub async fn handler(ctx: &Context, msg: &Message) {
        let guildid = msg.guild_id.unwrap().0;
        let member = msg.member.to_owned().unwrap();
        let gulag_length = 300;
        let channelid = msg.channel_id.0;

        match GulagHandler::find_gulag_role(ctx, guildid).await {
            None => println!("couldn't find gulag id"),
            Some(gulag_roleid) => {
                if Elon::has_elon_role(&ctx, guildid, &member).await {
                    println!("Send to gulag");
                    GulagHandler::add_to_gulag(
                        ctx,
                        guildid,
                        member.user.unwrap().id.0,
                        gulag_roleid.id.0,
                        gulag_length,
                        channelid,
                    )
                    .await;
                }
            }
        }
    }

    pub async fn has_elon_role(ctx: &Context, guildid: u64, member: &PartialMember) -> bool {
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
