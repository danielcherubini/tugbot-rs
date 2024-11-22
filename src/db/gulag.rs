use super::{
    models::{GulagUser, GulagVote, NewGulagUser, NewGulagVote},
    schema::{
        gulag_users::{self},
        gulag_votes::{self},
    },
};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::{
    ops::Add,
    time::{Duration, SystemTime},
};

pub fn send_to_gulag(
    conn: &mut PgConnection,
    user_id: i64,
    guild_id: i64,
    gulag_role_id: i64,
    gulag_length: i32,
    channel_id: i64,
    message_id: i64,
) -> GulagUser {
    let time_now = SystemTime::now();
    let gulag_duration = Duration::from_secs(gulag_length as u64);
    let release_time = time_now.add(gulag_duration);

    let new_user = NewGulagUser {
        user_id,
        guild_id,
        gulag_role_id,
        channel_id,
        in_gulag: true,
        gulag_length,
        created_at: time_now,
        release_at: release_time,
        message_id,
    };

    diesel::insert_into(gulag_users::table)
        .values(&new_user)
        .get_result(conn)
        .expect("Error saving new User")
}

pub fn add_time_to_gulag(
    conn: &mut PgConnection,
    gulag_user_id: i32,
    gulag_length: i32,
    gulag_duration: i32,
    release_at: SystemTime,
) -> GulagUser {
    let gulag_duration = Duration::from_secs(gulag_duration as u64);
    let new_release_time = release_at.add(gulag_duration);
    diesel::update(gulag_users::dsl::gulag_users.find(gulag_user_id))
        .set((
            gulag_users::gulag_length.eq(gulag_length),
            gulag_users::release_at.eq(new_release_time),
        ))
        .get_result(conn)
        .expect("Error saving new User")
}

pub fn new_gulag_vote(
    conn: &mut PgConnection,
    requester_id: i64,
    sender_id: i64,
    guild_id: i64,
    gulag_role_id: i64,
    message_id: i64,
    channel_id: i64,
) -> GulagVote {
    let new_gulag_vote = NewGulagVote {
        requester_id,
        sender_id,
        guild_id,
        channel_id,
        gulag_role_id,
        processed: false,
        message_id,
        created_at: SystemTime::now(),
    };
    println!("inserting");
    diesel::insert_into(gulag_votes::table)
        .values(&new_gulag_vote)
        .get_result(conn)
        .expect("Error saving new gulag vote")
}
