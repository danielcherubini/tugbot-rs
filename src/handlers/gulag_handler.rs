use std::{sync::Arc, time::Duration};

use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    http::Http,
    model::{
        guild::{Member, Role},
        id::RoleId,
        interactions::application_command::{
            ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
            ApplicationCommandOptionType,
        },
    },
};
use tokio::{task::spawn, time::sleep};

use crate::db::schema::gulag_users::dsl::*;
use crate::db::{establish_connection, models::GulagUser, send_to_gulag};
use diesel::*;

use super::handlers::HandlerResponse;

pub struct GulagHandler;

impl GulagHandler {
    pub async fn find_gulag_role(ctx: &Context, guildid: u64) -> Option<Role> {
        match ctx.http.get_guild_roles(guildid).await {
            Err(_why) => None,
            Ok(roles) => {
                for role in roles {
                    if role.name == "gulag" {
                        return Some(role);
                    }
                }
                None
            }
        }
    }

    pub async fn add_to_gulag(
        ctx: &Context,
        guildid: u64,
        userid: u64,
        gulag_roleid: RoleId,
        channelid: u64,
    ) -> Member {
        let mut mem = ctx.http.get_member(guildid, userid).await.unwrap();
        mem.add_role(&ctx.http, gulag_roleid).await.unwrap();
        let conn = &mut establish_connection();

        send_to_gulag(
            conn,
            userid as i64,
            guildid as i64,
            *gulag_roleid.as_u64() as i64,
            channelid as i64,
        );

        return mem;
    }

    async fn remove_from_gulag(
        http: Arc<Http>,
        userid: u64,
        guildid: u64,
        gulag_roleid: RoleId,
        channelid: u64,
    ) {
        let mut mem = http.get_member(guildid, userid).await.unwrap();
        mem.remove_role(&http, gulag_roleid).await.unwrap();
        let message = format!("Freeing {} from the gulag", mem.to_string());
        let channel = http.get_channel(channelid).await.unwrap();
        channel
            .id()
            .send_message(http, |m| m.content(message))
            .await
            .unwrap();
    }

    pub fn run_gulag_check(ctx: &Context) {
        let http = Arc::clone(&ctx.http);
        spawn(async move {
            loop {
                let conn = &mut establish_connection();
                sleep(Duration::from_secs(1)).await;
                let results = gulag_users
                    .filter(in_gulag.eq(true))
                    .load::<GulagUser>(conn)
                    .expect("Error loading Servers");
                println!("Gulag Users {}", results.len());
                if results.len() > 0 {
                    for result in results {
                        let greater_than_5_minutes =
                            result.created_at.elapsed().unwrap() > Duration::from_secs(5);
                        if greater_than_5_minutes {
                            println!("{}", result.user_id);
                            diesel::delete(gulag_users.filter(id.eq(result.id)))
                                .execute(conn)
                                .expect("delete user");

                            GulagHandler::remove_from_gulag(
                                http.to_owned(),
                                result.user_id as u64,
                                result.guild_id as u64,
                                RoleId(result.gulag_role_id as u64),
                                result.channel_id as u64,
                            )
                            .await;
                        }
                    }
                }
            }
        });
    }

    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command
            .name("gulag")
            .description("Send a user to the Gulag")
            .create_option(|option| {
                option
                    .name("user")
                    .description("The user to lookup")
                    .kind(ApplicationCommandOptionType::User)
                    .required(true)
            });
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        let options = command
            .data
            .options
            .get(0)
            .expect("Expected user option")
            .resolved
            .as_ref()
            .expect("Expected user object");
        let channelid = command.channel_id.0;
        if let ApplicationCommandInteractionDataOptionValue::User(user, _member) = options {
            match command.guild_id {
                None => {
                    return HandlerResponse {
                        content: "no member".to_string(),
                        ephemeral: false,
                    }
                }
                Some(guildid) => match Self::find_gulag_role(&ctx, *guildid.as_u64()).await {
                    None => {
                        return HandlerResponse {
                            content: "couldn't find gulag role".to_string(),
                            ephemeral: false,
                        }
                    }
                    Some(gulag_role) => {
                        let _mem = GulagHandler::add_to_gulag(
                            ctx,
                            *guildid.as_u64(),
                            *user.id.as_u64(),
                            gulag_role.id,
                            channelid,
                        )
                        .await;
                        // let http = Arc::clone(&ctx.http);
                        // spawn(async move {
                        //     sleep(Duration::from_secs(300)).await;
                        //     Gulag::remove_from_gulag(http, mem, gulag_role.id, channel_id).await;
                        // });

                        return HandlerResponse {
                            content: format!("Sending {} to the Gulag", user.to_string()),
                            ephemeral: false,
                        };
                    }
                },
            }
        } else {
            return HandlerResponse {
                content: "Please provide a valid user".to_string(),
                ephemeral: false,
            };
        };
    }
}
