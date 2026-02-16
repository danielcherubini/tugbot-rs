pub mod message_vote;
pub mod models;
pub mod queries;
pub mod schema;

use self::{
    models::{
        AiSlopUsage, GulagUser, GulagVote, NewAiSlopUsage, NewGulagUser, NewGulagVote, NewServer,
        Server,
    },
    schema::{
        ai_slop_usage::{self},
        gulag_users::{self},
        gulag_votes::{self},
        servers,
    },
};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenv::dotenv;
use std::{
    env,
    ops::Add,
    time::{Duration, SystemTime},
};

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

/// Helper to convert pool errors to Diesel errors
fn pool_error_to_diesel(e: diesel::r2d2::PoolError) -> diesel::result::Error {
    diesel::result::Error::QueryBuilderError(Box::new(e))
}

/// Establishes a connection pool for database operations
/// This should be called once at application startup
pub fn establish_pool() -> DbPool {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    Pool::builder()
        .max_size(15) // Maximum number of connections in the pool
        .connection_timeout(Duration::from_secs(30))
        .build(manager)
        .expect("Failed to create database connection pool")
}

/// Legacy function for backwards compatibility during migration
/// Prefer using the pool directly via establish_pool()
#[deprecated(note = "Use establish_pool() and pass DbPool instead")]
pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn create_server(
    pool: &DbPool,
    guild_id: i64,
    gulag_id: i64,
) -> Result<Server, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    let new_server = NewServer { guild_id, gulag_id };

    diesel::insert_into(servers::table)
        .values(&new_server)
        .get_result(&mut conn)
}

pub fn send_to_gulag(
    pool: &DbPool,
    user_id: i64,
    guild_id: i64,
    gulag_role_id: i64,
    gulag_length: i32,
    channel_id: i64,
    message_id: i64,
) -> Result<GulagUser, diesel::result::Error> {
    // Validate gulag_length is non-negative
    if gulag_length < 0 {
        return Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::CheckViolation,
            Box::new("gulag_length must be non-negative".to_string()),
        ));
    }

    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
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
        .get_result(&mut conn)
}

pub fn add_time_to_gulag(
    pool: &DbPool,
    gulag_user_id: i32,
    gulag_length: i32,
    gulag_duration: i32,
    release_at: SystemTime,
) -> Result<GulagUser, diesel::result::Error> {
    // Validate gulag_duration is non-negative
    if gulag_duration < 0 {
        return Err(diesel::result::Error::QueryBuilderError(
            "gulag_duration must be non-negative".into(),
        ));
    }

    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    let gulag_duration = Duration::from_secs(gulag_duration as u64);
    let new_release_time = release_at.add(gulag_duration);
    diesel::update(gulag_users::dsl::gulag_users.find(gulag_user_id))
        .set((
            gulag_users::gulag_length.eq(gulag_length),
            gulag_users::release_at.eq(new_release_time),
        ))
        .get_result(&mut conn)
}

pub fn new_gulag_vote(
    pool: &DbPool,
    requester_id: i64,
    sender_id: i64,
    guild_id: i64,
    gulag_role_id: i64,
    message_id: i64,
    channel_id: i64,
) -> Result<GulagVote, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
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
    diesel::insert_into(gulag_votes::table)
        .values(&new_gulag_vote)
        .get_result(&mut conn)
}

pub fn get_or_create_ai_slop_usage(
    pool: &DbPool,
    target_user_id: i64,
    target_guild_id: i64,
) -> Result<AiSlopUsage, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use self::ai_slop_usage::dsl::*;

    // Try to get existing record
    match ai_slop_usage
        .filter(user_id.eq(target_user_id))
        .filter(guild_id.eq(target_guild_id))
        .first::<AiSlopUsage>(&mut conn)
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
                .get_result(&mut conn)
        }
        Err(e) => Err(e),
    }
}

pub fn increment_ai_slop_usage(
    pool: &DbPool,
    usage_id: i32,
    new_count: i32,
) -> Result<AiSlopUsage, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use self::ai_slop_usage::dsl::*;

    diesel::update(ai_slop_usage.find(usage_id))
        .set((
            usage_count.eq(new_count),
            last_slop_at.eq(SystemTime::now()),
        ))
        .get_result(&mut conn)
}

pub fn atomic_increment_ai_slop(
    pool: &DbPool,
    target_user_id: i64,
    target_guild_id: i64,
) -> Result<i32, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use self::ai_slop_usage::dsl::*;

    // Upsert: insert with count=1 or increment existing
    // This is fully atomic and returns the NEW count
    let time_now = SystemTime::now();
    let new_record = NewAiSlopUsage {
        user_id: target_user_id,
        guild_id: target_guild_id,
        usage_count: 1,
        last_slop_at: time_now,
        created_at: time_now,
    };

    // Use Diesel's on_conflict API for proper upsert
    let result: AiSlopUsage = diesel::insert_into(ai_slop_usage)
        .values(&new_record)
        .on_conflict((user_id, guild_id))
        .do_update()
        .set((usage_count.eq(usage_count + 1), last_slop_at.eq(time_now)))
        .get_result(&mut conn)?;

    Ok(result.usage_count)
}

pub fn get_server_by_guild_id(pool: &DbPool, target_guild_id: i64) -> Option<Server> {
    let mut conn = pool.get().ok()?;
    use self::servers::dsl::*;

    servers
        .filter(guild_id.eq(target_guild_id))
        .first::<Server>(&mut conn)
        .ok()
}
