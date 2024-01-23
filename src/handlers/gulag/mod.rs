use super::handlers::HandlerResponse;
use crate::db::{
    add_time_to_gulag, establish_connection,
    models::{GulagUser, MessageVotes},
    schema::{
        gulag_users::{self, dsl::*},
        message_votes::{self, dsl::*},
    },
    send_to_gulag,
};
use anyhow::{Context, Result};
use diesel::*;
use serenity::{
    http::Http,
    model::{guild::Role, id::RoleId, prelude::GuildChannel, user::User},
};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{task::spawn, time::sleep};

pub mod gulag_handler;
pub mod gulag_list_handler;
pub mod gulag_message_command;
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
        Self::find_role(http, guildid, "gulag").await
    }

    pub async fn find_role(http: &Arc<Http>, guildid: u64, role_name: &str) -> Option<Role> {
        match http.get_guild_roles(guildid).await {
            Err(_why) => None,
            Ok(roles) => {
                for role in roles {
                    if role.name == role_name {
                        return Some(role);
                    }
                }
                None
            }
        }
    }

    pub async fn find_channel(
        http: &Arc<Http>,
        guildid: u64,
        channel_name: String,
    ) -> Option<GuildChannel> {
        match http.get_channels(guildid).await {
            Err(_why) => None,
            Ok(channels) => {
                for channel in channels {
                    if channel.name == channel_name {
                        return Some(channel);
                    }
                }
                None
            }
        }
    }

    pub async fn is_tugbot(http: &Arc<Http>, user: &User) -> Option<bool> {
        match http.get_current_user().await {
            Err(why) => {
                eprintln!("{:#?}", why);
                None
            }
            Ok(current_user) => Some(current_user.id.0 == user.id.0),
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
                gulaglength as i32,
                gulag_db_user.release_at,
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
        let mut gulaglength = 300;
        let user = http.get_user(userid).await?;
        match Self::find_role(http, guildid, "derpies").await {
            Some(derpies_role) => {
                if user.has_role(http, guildid, derpies_role).await? {
                    gulaglength = 600;
                }
            }
            None => {}
        };

        let gulag_channel = Gulag::find_channel(http, guildid, "the-gulag".to_string())
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
        let channel = Gulag::find_channel(&http, guildid, "the-gulag".to_string())
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
                    .filter(gulag_users::release_at.le(SystemTime::now()))
                    .for_update()
                    .skip_locked()
                    .load::<GulagUser>(conn)
                    .expect("Error loading Servers");
                // println!("{}", results.len());
                if results.len() > 0 {
                    for result in results {
                        println!(
                            "It's been {} minutes, releasing {} from the gulag",
                            result.gulag_length / 60,
                            result.id
                        );

                        diesel::update(gulag_users.filter(gulag_users::id.eq(result.id)))
                            .set(in_gulag.eq(false))
                            .execute(conn)
                            .unwrap();

                        match Gulag::remove_from_gulag(
                            http.to_owned(),
                            result.user_id as u64,
                            result.guild_id as u64,
                            RoleId(result.gulag_role_id as u64),
                        )
                        .await
                        {
                            Ok(_) => {
                                diesel::delete(gulag_users.filter(gulag_users::id.eq(result.id)))
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
        });
    }

    pub fn run_gulag_vote_check(http: &Arc<Http>) {
        let http = Arc::clone(&http);
        spawn(async move {
            let conn = &mut establish_connection();
            loop {
                sleep(Duration::from_secs(1)).await;
                let results = message_votes
                    .filter(message_votes::vote_tally.ge(5))
                    .for_update()
                    .skip_locked()
                    .load::<MessageVotes>(conn)
                    .expect("Error loading Servers");
                if results.len() > 0 {
                    for result in results {
                        // Remove all gulag emoji's from gulag_reaction
                        let message = http
                            .get_message(result.channel_id as u64, result.message_id as u64)
                            .await
                            .unwrap();
                        for reaction in message.reactions.to_owned() {
                            if reaction.reaction_type.to_string().contains(":gulag") {
                                message
                                    .delete_reaction_emoji(http.to_owned(), reaction.reaction_type)
                                    .await
                                    .unwrap();
                            }
                        }
                        // send to gulag and message
                        if let Err(why) = Gulag::send_to_gulag_and_message(
                            &http,
                            result.guild_id as u64,
                            result.user_id as u64,
                            result.channel_id as u64,
                            result.message_id as u64,
                            None,
                        )
                        .await
                        {
                            println!("Error running gulag vote {:?}", why);
                        }

                        // Delete the vote from the database
                        let _result = diesel::delete(message_votes.find(result.message_id))
                            .execute(conn)
                            .unwrap();
                        println!("Removed from database");
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
                release_at: user.release_at,
            })
        } else {
            None
        }
    }
}