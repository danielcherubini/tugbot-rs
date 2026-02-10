pub mod message_vote;
pub mod models;
pub mod schema;

use self::{
    models::{AiSlopUsage, GulagUser, GulagVote, NewAiSlopUsage, NewGulagUser, NewGulagVote, NewServer, Server},
    schema::{
        ai_slop_usage::{self},
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
        remod: false,
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

pub fn get_or_create_ai_slop_usage(
    conn: &mut PgConnection,
    target_user_id: i64,
    target_guild_id: i64,
) -> Result<AiSlopUsage, diesel::result::Error> {
    use self::ai_slop_usage::dsl::*;

    // Try to get existing record
    match ai_slop_usage
        .filter(user_id.eq(target_user_id))
        .filter(guild_id.eq(target_guild_id))
        .first::<AiSlopUsage>(conn)
    {
        Ok(usage) => Ok(usage),
        Err(diesel::result::Error::NotFound) => {
            // Create new record with count 0
            let new_usage = NewAiSlopUsage {
                user_id: target_user_id,
                guild_id: target_guild_id,
                usage_count: 0,
                last_slop_at: SystemTime::now(),
                created_at: SystemTime::now(),
            };

            diesel::insert_into(ai_slop_usage)
                .values(&new_usage)
                .get_result(conn)
        }
        Err(e) => Err(e),
    }
}

pub fn increment_ai_slop_usage(
    conn: &mut PgConnection,
    usage_id: i32,
    new_count: i32,
) -> Result<AiSlopUsage, diesel::result::Error> {
    use self::ai_slop_usage::dsl::*;

    diesel::update(ai_slop_usage.find(usage_id))
        .set((
            usage_count.eq(new_count),
            last_slop_at.eq(SystemTime::now()),
        ))
        .get_result(conn)
}

pub fn atomic_increment_ai_slop(
    conn: &mut PgConnection,
    target_user_id: i64,
    target_guild_id: i64,
) -> Result<i32, diesel::result::Error> {
    use diesel::sql_types::Integer;

    // Upsert: insert with count=1 or increment existing
    // This is fully atomic and returns the NEW count
    #[derive(QueryableByName)]
    struct CountResult {
        #[diesel(sql_type = Integer)]
        usage_count: i32,
    }

    let result: CountResult = diesel::sql_query(
        "INSERT INTO ai_slop_usage (user_id, guild_id, usage_count, last_slop_at, created_at)
         VALUES ($1, $2, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
         ON CONFLICT (user_id, guild_id)
         DO UPDATE SET
           usage_count = ai_slop_usage.usage_count + 1,
           last_slop_at = CURRENT_TIMESTAMP
         RETURNING usage_count"
    )
    .bind::<diesel::sql_types::BigInt, _>(target_user_id)
    .bind::<diesel::sql_types::BigInt, _>(target_guild_id)
    .get_result(conn)?;

    Ok(result.usage_count)
}

pub fn get_server_by_guild_id(conn: &mut PgConnection, target_guild_id: i64) -> Option<Server> {
    use self::servers::dsl::*;

    servers
        .filter(guild_id.eq(target_guild_id))
        .first::<Server>(conn)
        .ok()
}
