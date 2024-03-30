use serenity::{
    model::channel::{Message, Reaction, ReactionType},
    prelude::Context,
};

use super::gulag::Gulag;

pub struct Derpies;

impl Derpies {
    pub async fn message_handler(ctx: &Context, msg: &Message) {
        let guild_id = msg.guild_id.unwrap().0;
        match msg.member(&ctx.http).await {
            Err(_) => return,
            Ok(member) => {
                if Gulag::member_has_role(&ctx.http, guild_id, &member, "derpies").await {
                    println!("It's derpies");
                }
            }
        }
    }

    pub async fn reaction_add_handler(ctx: &Context, add_reaction: &Reaction) {
        let guild_id = add_reaction.guild_id.unwrap().0;
        let user_id = add_reaction.user_id.unwrap().0;
        let hotdog_reaction = ReactionType::Unicode("🌭".to_string());

        let message = ctx
            .http
            .get_message(add_reaction.channel_id.0, add_reaction.message_id.0)
            .await
            .unwrap();

        let message_member = ctx
            .http
            .get_member(guild_id, message.author.id.0)
            .await
            .unwrap();
        let reaction_member = ctx.http.get_member(guild_id, user_id).await.unwrap();

        if Gulag::member_has_role(&ctx.http, guild_id, &reaction_member, "derpies").await {
            if Gulag::member_has_role(
                &ctx.http,
                guild_id,
                &message_member,
                "spreading-slurping-wriggling",
            )
            .await
            {
                if add_reaction.emoji == hotdog_reaction {
                    let _ = ctx
                        .http
                        .delete_reaction(
                            add_reaction.channel_id.0,
                            add_reaction.message_id.0,
                            Some(user_id),
                            &hotdog_reaction,
                        )
                        .await;
                }
            }
        }
    }
}
