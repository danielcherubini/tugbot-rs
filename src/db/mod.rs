pub mod message_vote;
pub mod models;
pub mod schema;

diesel::define_sql_function! {
    /// PostgreSQL GREATEST function — returns the larger of two timestamps.
    /// Used by bulk_upsert_activity to prevent timestamp regression on startup scan.
    fn greatest(a: diesel::sql_types::Timestamp, b: diesel::sql_types::Timestamp) -> diesel::sql_types::Timestamp;
}

use self::{
    models::{
        AiSlopUsage, GokuPollUsage, GulagUser, GulagVote, IsThisRealUsage, NewAiSlopUsage,
        NewGokuPollUsage, NewGulagUser, NewGulagVote, NewIsThisRealUsage, NewServer,
        NewUserActivity, Server, UserActivity,
    },
    schema::{
        ai_slop_usage::{self},
        goku_poll_usage::{self},
        gulag_users::{self},
        gulag_votes::{self},
        is_this_real_usage::{self},
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

pub fn get_or_create_goku_poll_usage(
    pool: &DbPool,
    target_user_id: i64,
    target_guild_id: i64,
) -> Result<GokuPollUsage, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use self::goku_poll_usage::dsl::*;

    match goku_poll_usage
        .filter(user_id.eq(target_user_id))
        .filter(guild_id.eq(target_guild_id))
        .first::<GokuPollUsage>(&mut conn)
    {
        Ok(usage) => Ok(usage),
        Err(diesel::result::Error::NotFound) => {
            let new_usage = NewGokuPollUsage {
                user_id: target_user_id,
                guild_id: target_guild_id,
                usage_count: 0,
                last_goku_at: SystemTime::now(),
                created_at: SystemTime::now(),
            };

            diesel::insert_into(goku_poll_usage)
                .values(&new_usage)
                .get_result(&mut conn)
        }
        Err(e) => Err(e),
    }
}

pub fn atomic_increment_goku_poll(
    pool: &DbPool,
    target_user_id: i64,
    target_guild_id: i64,
) -> Result<i32, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use self::goku_poll_usage::dsl::*;

    let time_now = SystemTime::now();
    let new_record = NewGokuPollUsage {
        user_id: target_user_id,
        guild_id: target_guild_id,
        usage_count: 1,
        last_goku_at: time_now,
        created_at: time_now,
    };

    let result: GokuPollUsage = diesel::insert_into(goku_poll_usage)
        .values(&new_record)
        .on_conflict((user_id, guild_id))
        .do_update()
        .set((usage_count.eq(usage_count + 1), last_goku_at.eq(time_now)))
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

pub fn get_is_this_real_usage(
    pool: &DbPool,
    target_user_id: i64,
    target_guild_id: i64,
) -> Option<IsThisRealUsage> {
    let mut conn = pool.get().ok()?;
    use self::is_this_real_usage::dsl::*;

    is_this_real_usage
        .filter(user_id.eq(target_user_id))
        .filter(guild_id.eq(target_guild_id))
        .first::<IsThisRealUsage>(&mut conn)
        .ok()
}

pub fn get_or_create_is_this_real_usage(
    pool: &DbPool,
    target_user_id: i64,
    target_guild_id: i64,
) -> Result<IsThisRealUsage, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use self::is_this_real_usage::dsl::*;

    match is_this_real_usage
        .filter(user_id.eq(target_user_id))
        .filter(guild_id.eq(target_guild_id))
        .first::<IsThisRealUsage>(&mut conn)
    {
        Ok(usage) => Ok(usage),
        Err(diesel::result::Error::NotFound) => {
            let new_usage = NewIsThisRealUsage {
                user_id: target_user_id,
                guild_id: target_guild_id,
                last_used_at: SystemTime::now(),
                created_at: SystemTime::now(),
            };

            diesel::insert_into(is_this_real_usage)
                .values(&new_usage)
                .get_result(&mut conn)
        }
        Err(e) => Err(e),
    }
}

pub fn update_is_this_real_usage(
    pool: &DbPool,
    usage_id: i32,
) -> Result<IsThisRealUsage, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;

    diesel::update(is_this_real_usage::dsl::is_this_real_usage.find(usage_id))
        .set(is_this_real_usage::dsl::last_used_at.eq(SystemTime::now()))
        .get_result(&mut conn)
}

pub fn bulk_upsert_activity(
    pool: &DbPool,
    records: Vec<(i64, i64)>,
) -> Result<usize, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use crate::db::schema::user_activity;
    use diesel::prelude::*;

    let time_now = SystemTime::now();
    let new_records: Vec<NewUserActivity> = records
        .into_iter()
        .map(|(uid, gid)| NewUserActivity {
            user_id: uid,
            guild_id: gid,
            last_message_at: time_now,
            created_at: time_now,
        })
        .collect();

    let rows = diesel::insert_into(user_activity::table)
        .values(&new_records)
        .on_conflict((user_activity::user_id, user_activity::guild_id))
        .do_update()
        .set(user_activity::last_message_at.eq(greatest(
            user_activity::last_message_at,
            diesel::upsert::excluded(user_activity::last_message_at),
        )))
        .execute(&mut conn)?;

    Ok(rows)
}

pub fn query_inactive_users(
    pool: &DbPool,
    guild_id: i64,
    days: i32,
) -> Result<Vec<i64>, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use crate::db::schema::user_activity;
    use diesel::prelude::*;

    let cutoff = SystemTime::now() - Duration::from_secs((days as u64) * 86400);
    let inactive_ids: Vec<i64> = user_activity::table
        .filter(user_activity::guild_id.eq(guild_id))
        .filter(user_activity::last_message_at.lt(cutoff))
        .select(user_activity::user_id)
        .load(&mut conn)?;

    Ok(inactive_ids)
}

pub fn query_all_tracked_user_ids_for_guild(
    pool: &DbPool,
    guild_id: i64,
) -> Result<Vec<i64>, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use crate::db::schema::user_activity;
    use diesel::prelude::*;

    let tracked_ids: Vec<i64> = user_activity::table
        .filter(user_activity::guild_id.eq(guild_id))
        .select(user_activity::user_id)
        .load(&mut conn)?;

    Ok(tracked_ids)
}

pub fn query_user_activity_for_ids(
    pool: &DbPool,
    guild_id: i64,
    user_ids: Vec<i64>,
) -> Result<Vec<UserActivity>, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use crate::db::schema::user_activity;
    use diesel::prelude::*;

    let results: Vec<UserActivity> = user_activity::table
        .filter(user_activity::guild_id.eq(guild_id))
        .filter(user_activity::user_id.eq_any(user_ids))
        .load(&mut conn)?;

    Ok(results)
}
