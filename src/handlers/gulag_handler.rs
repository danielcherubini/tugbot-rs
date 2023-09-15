use std::{sync::Arc, time::Duration};

use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    http::Http,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        guild::Role,
        id::RoleId,
        prelude::{
            application_command::CommandDataOptionValue, command::CommandOptionType, GuildChannel,
        },
        user::User,
    },
};
use tokio::{task::spawn, time::sleep};

use crate::db::{add_time_to_gulag, schema::gulag_users::dsl::*};
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

    pub async fn find_gulag_channel(http: &Arc<Http>, guildid: u64) -> Option<GuildChannel> {
        match http.get_channels(guildid).await {
            Err(_why) => None,
            Ok(channels) => {
                for channel in channels {
                    if channel.name == "the-gulag" {
                        return Some(channel);
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
        gulag_roleid: u64,
        gulaglength: u32,
        channelid: u64,
    ) -> GulagUser {
        let mut mem = ctx.http.get_member(guildid, userid).await.unwrap();
        mem.add_role(&ctx.http, RoleId(gulag_roleid)).await.unwrap();
        let conn = &mut establish_connection();

        match GulagHandler::is_user_in_gulag(userid) {
            Some(gulag_db_user) => add_time_to_gulag(
                conn,
                gulag_db_user.id,
                gulag_db_user.gulag_length + 300 as i32,
            ),
            None => send_to_gulag(
                conn,
                userid as i64,
                guildid as i64,
                gulag_roleid as i64,
                gulaglength as i32,
                channelid as i64,
            ),
        }
    }

    pub async fn send_to_gulag_and_message(
        ctx: &Context,
        guildid: u64,
        userid: u64,
        channelid: u64,
        messageid: u64,
        users: Option<Vec<User>>,
    ) {
        match GulagHandler::find_gulag_role(&ctx, guildid).await {
            None => println!("Couldn't find gulag role"),
            Some(role) => {
                let gulaglength = 300;

                let gulag_channel = GulagHandler::find_gulag_channel(&ctx.http, guildid)
                    .await
                    .unwrap();

                let gulag_user = GulagHandler::add_to_gulag(
                    &ctx,
                    guildid,
                    userid,
                    role.id.0,
                    gulaglength,
                    gulag_channel.id.0,
                )
                .await;

                let msg = ctx.http.get_message(channelid, messageid).await.unwrap();
                let member = ctx.http.get_member(guildid, userid).await.unwrap();

                let mut user_string = "".to_string();
                if users.is_some() {
                    user_string = "\nThese people voted them in".to_string();
                    for user in users.unwrap() {
                        user_string.push_str(format!(", {}", user).as_str());
                    }
                }

                let content = format!(
                    "Sending {} to the Gulag for {} minutes because of this {}{}",
                    member.user.to_string(),
                    gulag_user.gulag_length / 60,
                    msg.link(),
                    user_string,
                );

                if let Err(why) = gulag_channel.say(ctx, content).await {
                    println!("Error Editing Message to Tweet {:?}", why);
                }
            }
        };
    }

    async fn remove_from_gulag(http: Arc<Http>, userid: u64, guildid: u64, gulag_roleid: RoleId) {
        let mut mem = http.get_member(guildid, userid).await.unwrap();
        mem.remove_role(&http, gulag_roleid).await.unwrap();
        let message = format!("Freeing {} from the gulag", mem.to_string());
        let channel = GulagHandler::find_gulag_channel(&http, guildid)
            .await
            .unwrap();
        match channel.send_message(&http, |m| m.content(message)).await {
            Ok(_) => {
                println!("Removed from gulag");
                return;
            }
            Err(e) => {
                println!("Error releasing from gulag {}", e.to_string());
                return;
            }
        }
    }

    pub fn run_gulag_check(ctx: &Context) {
        let http = Arc::clone(&ctx.http);
        spawn(async move {
            let conn = &mut establish_connection();
            loop {
                sleep(Duration::from_secs(1)).await;
                let results = gulag_users
                    .filter(in_gulag.eq(true))
                    .load::<GulagUser>(conn)
                    .expect("Error loading Servers");
                if results.len() > 0 {
                    println!("There are {} users in the gulag", results.len());
                    for result in results {
                        let greater_than_5_minutes = result.created_at.elapsed().unwrap()
                            > Duration::from_secs(result.gulag_length as u64);
                        if greater_than_5_minutes {
                            println!(
                                "It's been 5 minutes, releasing {} from the gulag",
                                result.id
                            );
                            diesel::delete(gulag_users.filter(id.eq(result.id)))
                                .execute(conn)
                                .expect("delete user");
                            println!("Removed from database");
                            GulagHandler::remove_from_gulag(
                                http.to_owned(),
                                result.user_id as u64,
                                result.guild_id as u64,
                                RoleId(result.gulag_role_id as u64),
                            )
                            .await;
                        }
                    }
                }
            }
        });
    }

    pub fn is_user_in_gulag(userid: u64) -> Option<GulagUser> {
        let conn = &mut establish_connection();
        let results = gulag_users
            .filter(user_id.eq(userid as i64))
            .load::<GulagUser>(conn)
            .expect("Error loading Servers");
        if results.len() > 0 {
            let user = results.first().unwrap();
            Some(GulagUser {
                id: user.id,
                user_id: user.user_id,
                channel_id: user.channel_id,
                guild_id: user.guild_id,
                gulag_role_id: user.gulag_role_id,
                gulag_length: user.gulag_length,
                created_at: user.created_at,
                in_gulag: user.in_gulag,
            })
        } else {
            None
        }
    }

    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command
            .name("gulag")
            .description("Send a user to the Gulag")
            .create_option(|option| {
                option
                    .name("user")
                    .description("The user to lookup")
                    .kind(CommandOptionType::User)
                    .required(true)
            })
            .create_option(|option| {
                option
                    .name("reason")
                    .description("Why Are you sending them")
                    .kind(CommandOptionType::String)
                    .required(true)
            });
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        let user_options = command
            .data
            .options
            .get(0)
            .expect("Expected user option")
            .resolved
            .as_ref()
            .expect("Expected user object");
        let reason_options = command
            .data
            .options
            .get(1)
            .expect("Expected reason option")
            .resolved
            .as_ref()
            .expect("Expected reason object");

        let channelid = command.channel_id.0;
        let gulaglength = 300;
        if let CommandDataOptionValue::User(user, _member) = user_options {
            match command.guild_id {
                None => {
                    return HandlerResponse {
                        content: "no member".to_string(),
                        components: None,
                        ephemeral: false,
                    }
                }
                Some(guildid) => match Self::find_gulag_role(&ctx, *guildid.as_u64()).await {
                    None => {
                        return HandlerResponse {
                            content: "couldn't find gulag role".to_string(),
                            components: None,
                            ephemeral: false,
                        }
                    }
                    Some(gulag_role) => {
                        let _mem = GulagHandler::add_to_gulag(
                            ctx,
                            *guildid.as_u64(),
                            *user.id.as_u64(),
                            *gulag_role.id.as_u64(),
                            gulaglength,
                            channelid,
                        )
                        .await;

                        if let CommandDataOptionValue::String(reason) = reason_options {
                            let content = format!(
                                "Sending {} to the Gulag for {} minutes, because {}",
                                user.to_string(),
                                gulaglength / 60,
                                reason,
                            );
                            return HandlerResponse {
                                content,
                                components: None,
                                ephemeral: false,
                            };
                        } else {
                            let content = format!(
                                "Sending {} to the Gulag for {} minutes",
                                user.to_string(),
                                gulaglength / 60,
                            );
                            return HandlerResponse {
                                content,
                                components: None,
                                ephemeral: false,
                            };
                        }
                    }
                },
            }
        } else {
            return HandlerResponse {
                content: "Please provide a valid user".to_string(),
                components: None,
                ephemeral: false,
            };
        };
    }
}
