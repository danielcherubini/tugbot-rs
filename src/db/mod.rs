pub mod feats;
pub mod message_vote;
pub mod models;
pub mod schema;

use self::{
    models::{GulagUser, GulagVote, NewGulagUser, NewGulagVote, NewServer, Server},
    schema::{
        gulag_users::{self},
        gulag_votes::{self},
        servers,
    },
};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::{
    env,
    ops::Add,
    time::{Duration, SystemTime},
};

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn create_server(conn: &mut PgConnection, guild_id: i64, gulag_id: i64) -> Server {
    let new_server = NewServer { guild_id, gulag_id };

    diesel::insert_into(servers::table)
        .values(&new_server)
        .get_result(conn)
        .expect("Error saving new server")
}

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
