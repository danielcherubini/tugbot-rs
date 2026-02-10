use super::HandlerResponse;
use crate::db::{
    add_time_to_gulag,
    models::{GulagUser, JobStatus, MessageVotes},
    schema::{
        gulag_users::{self, dsl::*},
        message_votes::{self, dsl::*},
    },
    send_to_gulag, DbPool,
};
use anyhow::{Context, Result};
use diesel::*;
use serenity::{
    all::CreateMessage,
    http::Http,
    model::{
        guild::{Member, Role},
        id::RoleId,
        prelude::GuildChannel,
        user::User,
    },
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

pub struct GulagParams {
    pub guildid: u64,
    pub userid: u64,
    pub gulag_roleid: u64,
    pub gulaglength: u32,
    pub channelid: u64,
    pub messageid: u64,
}

impl Gulag {
    fn send_error(err: &str) -> HandlerResponse {
        HandlerResponse {
            content: format!("Error: {}", err),
            components: None,
            ephemeral: true,
        }
    }

    pub async fn member_has_role(
        http: &Arc<Http>,
        guildid: u64,
        member: &Member,
        role_name: &str,
    ) -> bool {
        match Self::find_role(http, guildid, role_name).await {
            Some(derpies_role) => {
                for member_role in member.roles.iter().copied() {
                    if member_role.get() == derpies_role.id.get() {
                        return true;
                    };
                }
                false
            }
            None => false,
        }
    }

    pub async fn find_gulag_role(http: &Arc<Http>, guildid: u64) -> Option<Role> {
        Self::find_role(http, guildid, "gulag").await
    }

    pub async fn find_role(http: &Arc<Http>, guildid: u64, role_name: &str) -> Option<Role> {
        match http.get_guild_roles(guildid.into()).await {
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
        match http.get_channels(guildid.into()).await {
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
            Ok(current_user) => Some(current_user.id.get() == user.id.get()),
        }
    }

    pub async fn add_to_gulag(
        http: &Arc<Http>,
        pool: &DbPool,
        params: GulagParams,
    ) -> Result<GulagUser> {
        let mem = http
            .get_member(params.guildid.into(), params.userid.into())
            .await
            .with_context(|| "Failed to get guild member")?;
        mem.add_role(http, RoleId::new(params.gulag_roleid))
            .await
            .with_context(|| "Failed to add gulag role")?;

        match Gulag::is_user_in_gulag(pool, params.userid) {
            Some(gulag_db_user) => add_time_to_gulag(
                pool,
                gulag_db_user.id,
                gulag_db_user.gulag_length + params.gulaglength as i32,
                params.gulaglength as i32,
                gulag_db_user.release_at,
            )
            .with_context(|| "Failed to add time to gulag"),
            None => send_to_gulag(
                pool,
                params.userid as i64,
                params.guildid as i64,
                params.gulag_roleid as i64,
                params.gulaglength as i32,
                params.channelid as i64,
                params.messageid as i64,
            )
            .with_context(|| "Failed to send user to gulag"),
        }
    }

    pub async fn send_to_gulag_and_message(
        http: &Arc<Http>,
        pool: &DbPool,
        guildid: u64,
        userid: u64,
        channelid: u64,
        messageid: u64,
        users: Option<Vec<User>>,
    ) -> Result<()> {
        let gulag_role = Gulag::find_gulag_role(http, guildid)
            .await
            .with_context(|| "Couldn't find gulag role".to_string())?;
        let gulaglength = 300;

        let gulag_channel = Gulag::find_channel(http, guildid, "the-gulag".to_string())
            .await
            .with_context(|| "Cant find gulag channel".to_string())?;
        let gulag_user = Gulag::add_to_gulag(
            http,
            pool,
            GulagParams {
                guildid,
                userid,
                gulag_roleid: gulag_role.id.get(),
                gulaglength,
                channelid: gulag_channel.id.get(),
                messageid,
            },
        )
        .await
        .with_context(|| "Failed to add user to gulag")?;

        let msg = http.get_message(channelid.into(), messageid.into()).await?;
        let member = http.get_member(guildid.into(), userid.into()).await?;

        let mut user_string = "".to_string();
        if let Some(user_list) = users {
            user_string = "\nThese people voted them in".to_string();
            for user in user_list {
                user_string.push_str(format!(", {}", user).as_str());
            }
        }

        let content = format!(
            "Sending {} to the Gulag for {} minutes because of {}, they have {} minutes remaining{}",
            member.user,
            gulaglength / 60,
            msg.link(),
            gulag_user.gulag_length / 60,
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
        let mem = http.get_member(guildid.into(), userid.into()).await?;
        mem.remove_role(&http, gulag_roleid).await?;
        let channel = Gulag::find_channel(&http, guildid, "the-gulag".to_string())
            .await
            .with_context(|| "Couldn't find gulag channel".to_string())?;
        let message = format!("Freeing {} from the gulag", mem);
        channel
            .send_message(&http, CreateMessage::new().content(message))
            .await?;
        println!("Removed from gulag");
        Ok(())
    }

    pub fn run_gulag_check(http: &Arc<Http>, pool: DbPool) {
        let http = Arc::clone(http);
        spawn(async move {
            loop {
                sleep(Duration::from_secs(1)).await;

                // Get a fresh connection from the pool for each iteration
                let mut conn = match pool.get() {
                    Ok(conn) => conn,
                    Err(e) => {
                        eprintln!("Failed to get database connection in run_gulag_check: {}", e);
                        continue; // Skip this iteration and try again
                    }
                };

                let results = gulag_users
                    .filter(gulag_users::in_gulag.eq(true))
                    .filter(gulag_users::release_at.le(SystemTime::now()))
                    .for_update()
                    .skip_locked()
                    .load::<GulagUser>(&mut conn)
                    .expect("Error loading Servers");
                //println!("{:?}", results.len());
                if !results.is_empty() {
                    for result in results {
                        println!(
                            "It's been {} minutes, releasing {} from the gulag",
                            result.gulag_length / 60,
                            result.id
                        );

                        diesel::update(gulag_users.filter(gulag_users::id.eq(result.id)))
                            .set(in_gulag.eq(false))
                            .execute(&mut conn)
                            .unwrap();

                        match Gulag::remove_from_gulag(
                            http.to_owned(),
                            result.user_id as u64,
                            result.guild_id as u64,
                            RoleId::new(result.gulag_role_id as u64),
                        )
                        .await
                        {
                            Ok(_) => {
                                diesel::delete(gulag_users.filter(gulag_users::id.eq(result.id)))
                                    .execute(&mut conn)
                                    .expect("delete user");
                                println!("Removed from database");

                                if result.message_id != 0 {
                                    // Done the vote from the database
                                    let done_result = diesel::update(
                                        message_votes.filter(
                                            message_votes::message_id.eq(result.message_id),
                                        ),
                                    )
                                    .set(message_votes::job_status.eq(JobStatus::Done))
                                    .get_result::<MessageVotes>(&mut conn)
                                    .with_context(|| {
                                        format!(
                                            "failed to done message_vote_id {}",
                                            result.message_id
                                        )
                                    });

                                    match done_result {
                                        Ok(done_result) => {
                                            if done_result.job_status == JobStatus::Done {
                                                println!("Updated Gulag Vote Check Item to Done");
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("Error updating vote status: {:?}", e);
                                        }
                                    }
                                }
                            }
                            Err(why) => match why.to_string().as_str() {
                                "Unknown Guild" | "Unknown Message" => {
                                    diesel::delete(
                                        gulag_users.filter(gulag_users::id.eq(result.id)),
                                    )
                                    .execute(&mut conn)
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

    pub fn run_gulag_vote_check(http: &Arc<Http>, pool: DbPool) {
        let http = Arc::clone(http);
        spawn(async move {
            loop {
                sleep(Duration::from_secs(1)).await;

                // Get a fresh connection from the pool for each iteration
                let mut conn = match pool.get() {
                    Ok(conn) => conn,
                    Err(e) => {
                        eprintln!("Failed to get database connection in run_gulag_vote_check: {}", e);
                        continue; // Skip this iteration and try again
                    }
                };

                let job_status_predicate = message_votes::job_status
                    .eq(JobStatus::Created)
                    .or(message_votes::job_status.eq(JobStatus::Done));
                let results = message_votes
                    .filter(message_votes::current_vote_tally.ge(5))
                    .filter(job_status_predicate)
                    .for_update()
                    .skip_locked()
                    .load::<MessageVotes>(&mut conn)
                    .expect("Error loading Servers");
                if !results.is_empty() {
                    for result in results {
                        if let Err(err) =
                            Self::gulag_check_handler(http.to_owned(), &pool, &mut conn, &result).await
                        {
                            println!("Error running gulag vote {:?}", err);
                            let _result = diesel::update(message_votes.find(result.message_id))
                                .set(message_votes::job_status.eq(JobStatus::Failure))
                                .execute(&mut conn)
                                .unwrap();
                        }
                    }
                }
            }
        });
    }

    async fn gulag_check_handler(
        http: Arc<Http>,
        pool: &DbPool,
        conn: &mut PgConnection,
        result: &MessageVotes,
    ) -> Result<(), anyhow::Error> {
        // Set the vote to running in the database
        let updated_result: MessageVotes = diesel::update(message_votes.find(result.message_id))
            .set(message_votes::job_status.eq(JobStatus::Running))
            .get_result(conn)
            .with_context(|| format!("Failed to update message_vote_id {}", result.message_id))?;
        if updated_result.job_status == JobStatus::Running {
            println!("Updated Gulag Vote Check Item to Running");
            // Remove all gulag emoji's from gulag_reaction
            let message = http
                .get_message(
                    (result.channel_id as u64).into(),
                    (result.message_id as u64).into(),
                )
                .await
                .with_context(|| "Failed to get Message")?;

            // Iterate throught the message reactions and find the gulag type and remove it
            for reaction in message.reactions.iter().cloned() {
                if reaction.reaction_type.to_string().contains(":gulag") {
                    message
                        .delete_reaction_emoji(http.to_owned(), reaction.reaction_type)
                        .await
                        .with_context(|| "Failed to delete reaction emoji")?;
                }
            }

            // send to gulag and message
            return match Gulag::send_to_gulag_and_message(
                &http,
                pool,
                updated_result.guild_id as u64,
                updated_result.user_id as u64,
                updated_result.channel_id as u64,
                updated_result.message_id as u64,
                None,
            )
            .await
            {
                Ok(()) => {
                    println!("OK done with sending to gulag, now setting it to done");
                    let empty_vec: Vec<i64> = vec![];
                    let _updated_result: MessageVotes =
                        diesel::update(message_votes.find(result.message_id))
                            .set((
                                message_votes::job_status.eq(JobStatus::Done),
                                message_votes::total_vote_tally
                                    .eq(result.current_vote_tally + result.total_vote_tally),
                                message_votes::current_vote_tally.eq(0),
                                message_votes::voters.eq(empty_vec),
                            ))
                            .get_result(conn)
                            .with_context(|| {
                                format!("Failed to update message_vote_id {}", result.message_id)
                            })?;
                    Ok(())
                }
                Err(e) => Err(e),
            };
        }

        Ok(())
    }

    pub fn is_user_in_gulag(pool: &DbPool, userid: u64) -> Option<GulagUser> {
        let mut conn = pool.get().expect("Failed to get database connection from pool");
        let results = gulag_users
            .filter(gulag_users::user_id.eq(userid as i64))
            .load::<GulagUser>(&mut conn)
            .expect("Error loading Servers");
        if !results.is_empty() {
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
                remod: user.remod,
                message_id: user.message_id,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_error_formats_correctly() {
        let error_msg = "Something went wrong";
        let response = Gulag::send_error(error_msg);

        assert_eq!(response.content, "Error: Something went wrong");
        assert!(response.ephemeral);
        assert!(response.components.is_none());
    }

    #[test]
    fn test_send_error_with_empty_string() {
        let response = Gulag::send_error("");
        assert_eq!(response.content, "Error: ");
    }

    #[test]
    fn test_send_error_with_special_characters() {
        let response = Gulag::send_error("Test & <special> \"chars\"");
        assert_eq!(response.content, "Error: Test & <special> \"chars\"");
    }
}
