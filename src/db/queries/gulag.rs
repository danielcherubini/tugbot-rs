use crate::db::{
    models::{GulagUser, GulagVote, JobStatus, MessageVotes, NewGulagUser, NewGulagVote},
    schema::{gulag_users, gulag_votes},
    DbPool,
};
use diesel::prelude::*;
use std::{
    ops::Add,
    time::{Duration, SystemTime},
};

/// Helper to convert pool errors to Diesel errors
fn pool_error_to_diesel(e: diesel::r2d2::PoolError) -> diesel::result::Error {
    diesel::result::Error::QueryBuilderError(Box::new(e))
}

pub struct GulagQueries;

impl GulagQueries {
    /// Create a new gulag entry for a user
    pub fn create(
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

    /// Find a user in the gulag by user ID
    pub fn find_by_user_id(pool: &DbPool, user_id: i64) -> Option<GulagUser> {
        let mut conn = pool.get().ok()?;
        use crate::db::schema::gulag_users::dsl;

        dsl::gulag_users
            .filter(dsl::user_id.eq(user_id))
            .first::<GulagUser>(&mut conn)
            .optional()
            .ok()?
    }

    /// Find all users currently in the gulag whose release time has passed
    pub fn find_expired(pool: &DbPool) -> Result<Vec<GulagUser>, diesel::result::Error> {
        let mut conn = pool.get().map_err(pool_error_to_diesel)?;
        use crate::db::schema::gulag_users::dsl::*;

        gulag_users
            .filter(in_gulag.eq(true))
            .filter(release_at.le(SystemTime::now()))
            .for_update()
            .skip_locked()
            .load::<GulagUser>(&mut conn)
    }

    /// Find all active gulag users for a guild
    pub fn find_active_by_guild(
        pool: &DbPool,
        target_guild_id: i64,
    ) -> Result<Vec<GulagUser>, diesel::result::Error> {
        let mut conn = pool.get().map_err(pool_error_to_diesel)?;
        use crate::db::schema::gulag_users::dsl::*;

        gulag_users
            .filter(guild_id.eq(target_guild_id))
            .filter(in_gulag.eq(true))
            .load::<GulagUser>(&mut conn)
    }

    /// Add time to an existing gulag sentence
    pub fn add_time(
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

    /// Mark a user as no longer in gulag
    pub fn mark_released(
        pool: &DbPool,
        gulag_user_id: i32,
    ) -> Result<usize, diesel::result::Error> {
        let mut conn = pool.get().map_err(pool_error_to_diesel)?;
        use crate::db::schema::gulag_users::dsl::*;

        diesel::update(gulag_users.filter(id.eq(gulag_user_id)))
            .set(in_gulag.eq(false))
            .execute(&mut conn)
    }

    /// Delete a gulag entry
    pub fn delete(pool: &DbPool, gulag_user_id: i32) -> Result<usize, diesel::result::Error> {
        let mut conn = pool.get().map_err(pool_error_to_diesel)?;
        use crate::db::schema::gulag_users::dsl::*;

        diesel::delete(gulag_users.filter(id.eq(gulag_user_id))).execute(&mut conn)
    }

    /// Create a new gulag vote
    pub fn create_vote(
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

    /// Find message votes that have reached the threshold and need processing
    pub fn find_votes_ready_for_processing(
        pool: &DbPool,
        threshold: i32,
    ) -> Result<Vec<MessageVotes>, diesel::result::Error> {
        let mut conn = pool.get().map_err(pool_error_to_diesel)?;
        use crate::db::schema::message_votes::dsl::*;

        let job_status_predicate = job_status
            .eq(JobStatus::Created)
            .or(job_status.eq(JobStatus::Done));

        message_votes
            .filter(current_vote_tally.ge(threshold))
            .filter(job_status_predicate)
            .for_update()
            .skip_locked()
            .load::<MessageVotes>(&mut conn)
    }

    /// Update a message vote's job status
    pub fn update_vote_status(
        pool: &DbPool,
        target_message_id: i64,
        new_status: JobStatus,
    ) -> Result<MessageVotes, diesel::result::Error> {
        let mut conn = pool.get().map_err(pool_error_to_diesel)?;
        use crate::db::schema::message_votes::dsl::*;

        diesel::update(message_votes.find(target_message_id))
            .set(job_status.eq(new_status))
            .get_result(&mut conn)
    }

    /// Mark a vote as done and reset counters
    pub fn mark_vote_done(
        pool: &DbPool,
        target_message_id: i64,
        total_votes: i32,
    ) -> Result<MessageVotes, diesel::result::Error> {
        let mut conn = pool.get().map_err(pool_error_to_diesel)?;
        use crate::db::schema::message_votes::dsl::*;

        let empty_vec: Vec<i64> = vec![];
        diesel::update(message_votes.find(target_message_id))
            .set((
                job_status.eq(JobStatus::Done),
                total_vote_tally.eq(total_votes),
                current_vote_tally.eq(0),
                voters.eq(empty_vec),
            ))
            .get_result(&mut conn)
    }
}
