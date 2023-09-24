use super::handlers::HandlerResponse;
use crate::{
    db::{
        add_time_to_gulag, establish_connection,
        models::{GulagUser, GulagVote},
        schema::{
            gulag_users::{self, dsl::*},
            gulag_votes::{self, dsl::*},
        },
        send_to_gulag,
    },
    handlers::gulag::gulag_vote::GulagVoteHandler,
};
use anyhow::{Context, Result};
use diesel::*;
use serenity::{
    http::Http,
    model::{guild::Role, id::RoleId, prelude::GuildChannel, user::User},
};
use std::{sync::Arc, time::Duration};
use tokio::{task::spawn, time::sleep};

pub mod gulag_handler;
pub mod gulag_list_handler;
pub mod gulag_reaction;
pub mod gulag_remove_handler;
pub mod gulag_vote;

pub struct Gulag;

impl Gulag {
    fn send_error(err: &str) -> HandlerResponse {
        return HandlerResponse {
            content: format!("Error: {}", err),
            components: None,
            ephemeral: true,
        };
    }

    pub async fn find_gulag_role(http: &Arc<Http>, guildid: u64) -> Option<Role> {
        match http.get_guild_roles(guildid).await {
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
        http: &Arc<Http>,
        guildid: u64,
        userid: u64,
        gulag_roleid: u64,
        gulaglength: u32,
        channelid: u64,
    ) -> GulagUser {
        let mut mem = http.get_member(guildid, userid).await.unwrap();
        mem.add_role(http, RoleId(gulag_roleid)).await.unwrap();
        let conn = &mut establish_connection();

        match Gulag::is_user_in_gulag(userid) {
            Some(gulag_db_user) => add_time_to_gulag(
                conn,
                gulag_db_user.id,
                gulag_db_user.gulag_length + gulaglength as i32,
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
        http: &Arc<Http>,
        guildid: u64,
        userid: u64,
        channelid: u64,
        messageid: u64,
        users: Option<Vec<User>>,
    ) -> Result<()> {
        let gulag_role = Gulag::find_gulag_role(&http, guildid)
            .await
            .with_context(|| format!("Couldn't find gulag role"))?;
        let gulaglength = 300;
        let gulag_channel = Gulag::find_gulag_channel(http, guildid)
            .await
            .with_context(|| format!("Cant find gulag channel"))?;
        let gulag_user = Gulag::add_to_gulag(
            http,
            guildid,
            userid,
            gulag_role.id.0,
            gulaglength,
            gulag_channel.id.0,
        )
        .await;

        let msg = http.get_message(channelid, messageid).await?;
        let member = http.get_member(guildid, userid).await?;

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

        gulag_channel.say(http, content).await?;
        Ok(())
    }

    async fn remove_from_gulag(
        http: Arc<Http>,
        userid: u64,
        guildid: u64,
        gulag_roleid: RoleId,
    ) -> Result<()> {
        let mut mem = http.get_member(guildid, userid).await?;
        mem.remove_role(&http, gulag_roleid).await?;
        let channel = Gulag::find_gulag_channel(&http, guildid)
            .await
            .with_context(|| format!("Couldn't find gulag channel"))?;
        let message = format!("Freeing {} from the gulag", mem.to_string());
        channel.send_message(&http, |m| m.content(message)).await?;
        println!("Removed from gulag");
        return Ok(());
    }

    pub fn run_gulag_check(http: &Arc<Http>) {
        let http = Arc::clone(&http);
        spawn(async move {
            let conn = &mut establish_connection();
            loop {
                sleep(Duration::from_secs(1)).await;
                let results = gulag_users
                    .filter(gulag_users::in_gulag.eq(true))
                    .load::<GulagUser>(conn)
                    .expect("Error loading Servers");
                if results.len() > 0 {
                    for result in results {
                        let greater_than_5_minutes = result.created_at.elapsed().unwrap()
                            > Duration::from_secs(result.gulag_length as u64);
                        if greater_than_5_minutes {
                            println!(
                                "It's been 5 minutes, releasing {} from the gulag",
                                result.id
                            );

                            match Gulag::remove_from_gulag(
                                http.to_owned(),
                                result.user_id as u64,
                                result.guild_id as u64,
                                RoleId(result.gulag_role_id as u64),
                            )
                            .await
                            {
                                Ok(_) => {
                                    diesel::delete(
                                        gulag_users.filter(gulag_users::id.eq(result.id)),
                                    )
                                    .execute(conn)
                                    .expect("delete user");
                                    println!("Removed from database");
                                }
                                Err(why) => match why.to_string().as_str() {
                                    "Unknown Guild" | "Unknown Message" => {
                                        diesel::delete(
                                            gulag_users.filter(gulag_users::id.eq(result.id)),
                                        )
                                        .execute(conn)
                                        .expect("delete user");
                                        println!("Removed from database due to error {}", why);
                                    }
                                    _ => {
                                        println!("Error run_gulag_check: {:?}", why.to_string());
                                    }
                                },
                            };
                        }
                    }
                }
            }
        });
    }

    pub fn run_gulag_vote_check(http: &Arc<Http>) {
        let http = Arc::clone(&http);
        spawn(async move {
            let conn = &mut establish_connection();
            loop {
                sleep(Duration::from_secs(1)).await;
                let results = gulag_votes
                    .filter(gulag_votes::processed.eq(false))
                    .load::<GulagVote>(conn)
                    .expect("Error loading Servers");
                if results.len() > 0 {
                    for result in results {
                        let greater_than_10_minutes =
                            result.created_at.elapsed().unwrap() > Duration::from_secs(600);
                        if greater_than_10_minutes {
                            println!("It's been 10 minutes, processing gulag {}", result.id);

                            // Process the gulag votes here
                            match GulagVoteHandler::process_gulag_votes(
                                &http,
                                result.channel_id as u64,
                                result.message_id as u64,
                            )
                            .await
                            {
                                Ok(passed) => {
                                    let who_to_gulag: u64;
                                    if passed {
                                        who_to_gulag = result.sender_id as u64;
                                    } else {
                                        who_to_gulag = result.requester_id as u64;
                                    }

                                    if let Err(why) = Gulag::send_to_gulag_and_message(
                                        &http,
                                        result.guild_id as u64,
                                        who_to_gulag,
                                        result.channel_id as u64,
                                        result.message_id as u64,
                                        None,
                                    )
                                    .await
                                    {
                                        println!("Error running gulag vote {:?}", why);
                                    }
                                    let _result = diesel::update(gulag_votes.find(result.id))
                                        .set(gulag_votes::processed.eq(true))
                                        .returning(GulagVote::as_returning())
                                        .get_result(conn)
                                        .unwrap();
                                    println!("Removed from database");
                                }
                                Err(why) => match why.to_string().as_str() {
                                    "Unknown Guild" | "Unknown Message" => {
                                        diesel::delete(
                                            gulag_votes.filter(gulag_votes::id.eq(result.id)),
                                        )
                                        .execute(conn)
                                        .expect("delete user");
                                        println!("Removed from database Due to error {}", why);
                                    }
                                    _ => {
                                        println!("Error run_gulag_check: {:?}", why.to_string());
                                    }
                                },
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn is_user_in_gulag(userid: u64) -> Option<GulagUser> {
        let conn = &mut establish_connection();
        let results = gulag_users
            .filter(gulag_users::user_id.eq(userid as i64))
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
}
