use serenity::{
    model::channel::{Message, Reaction},
    prelude::Context,
};

use super::gulag::Gulag;
use crate::features::Features;

pub struct Derpies;

impl Derpies {
    pub async fn message_handler(ctx: &Context, msg: &Message) {
        if !Features::is_enabled("derpies") {
            return;
        }

        let Some(guild_id_val) = msg.guild_id else {
            return;
        };
        let guild_id = guild_id_val.get();
        match msg.member(&ctx.http).await {
            Err(_) => (),
            Ok(member) => {
                if Gulag::member_has_role(&ctx.http, guild_id, &member, "derpies").await {
                    println!("It's derpies");
                }
            }
        }
    }

    pub async fn reaction_add_handler(ctx: &Context, add_reaction: &Reaction) {
        if !Features::is_enabled("derpies") {
            return;
        }

        let Some(guild_id_val) = add_reaction.guild_id else {
            return;
        };
        let guild_id = guild_id_val.get();

        let Some(user_id_val) = add_reaction.user_id else {
            return;
        };
        let user_id = user_id_val.get();

        let Ok(reaction_member) = ctx.http.get_member(guild_id.into(), user_id.into()).await else {
            return;
        };

        let has_derpies_role =
            Gulag::member_has_role(&ctx.http, guild_id, &reaction_member, "derpies").await;

        // let message = ctx
        //     .http
        //     .get_message(add_reaction.channel_id.get(), add_reaction.message_id.get())
        //     .await
        //     .unwrap();
        // let message_member = ctx
        //     .http
        //     .get_member(guild_id, message.author.id.0)
        //     .await
        //     .unwrap();
        // let has_kovbasa_role = Gulag::member_has_role(
        //     &ctx.http,
        //     guild_id,
        //     &message_member,
        //     "spreading-slurping-wriggling",
        // )
        // .await;

        if has_derpies_role {
            let _ = ctx
                .http
                .delete_reaction(
                    add_reaction.channel_id,
                    add_reaction.message_id,
                    user_id.into(),
                    &add_reaction.emoji,
                )
                .await;
        }
    }
}
