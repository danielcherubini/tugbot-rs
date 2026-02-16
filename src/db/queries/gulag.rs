use crate::db::{
    models::{GulagUser, GulagVote, JobStatus, MessageVotes, NewGulagUser, NewGulagVote},
    pool_error_to_diesel,
    schema::{gulag_users, gulag_votes},
    DbPool,
};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::{
    ops::Add,
    time::{Duration, SystemTime},
};

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
            return Err(diesel::result::Error::QueryBuilderError(
                "gulag_length must be non-negative".into(),
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
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to acquire database connection in find_by_user_id for user {}: {}", user_id, e);
                return None;
            }
        };
        use crate::db::schema::gulag_users::dsl;

        match dsl::gulag_users
            .filter(dsl::user_id.eq(user_id))
            .first::<GulagUser>(&mut conn)
        {
            Ok(user) => Some(user),
            Err(diesel::result::Error::NotFound) => None,
            Err(e) => {
                eprintln!("Database error in find_by_user_id for user {}: {}", user_id, e);
                None
            }
        }
    }

    /// Find all users currently in the gulag whose release time has passed.
    ///
    /// Takes a connection reference so the caller can wrap this + mark_released
    /// in a single transaction, keeping FOR UPDATE locks held until commit.
    pub fn find_expired(conn: &mut PgConnection) -> Result<Vec<GulagUser>, diesel::result::Error> {
        use crate::db::schema::gulag_users::dsl::*;

        gulag_users
            .filter(in_gulag.eq(true))
            .filter(release_at.le(SystemTime::now()))
            .for_update()
            .skip_locked()
            .load::<GulagUser>(conn)
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

    /// Add time to an existing gulag sentence atomically.
    ///
    /// This function reads the current gulag_length and release_at, then atomically
    /// updates them by adding the specified duration. This avoids TOCTOU issues.
    pub fn add_time(
        pool: &DbPool,
        gulag_user_id: i32,
        additional_duration_secs: i32,
    ) -> Result<GulagUser, diesel::result::Error> {
        // Validate duration is non-negative
        if additional_duration_secs < 0 {
            return Err(diesel::result::Error::QueryBuilderError(
                "additional_duration_secs must be non-negative".into(),
            ));
        }

        let mut conn = pool.get().map_err(pool_error_to_diesel)?;

        // Start a transaction to ensure atomicity
        conn.transaction(|conn| {
            use crate::db::schema::gulag_users::dsl::*;

            // Read current values with FOR UPDATE lock
            let current_user: GulagUser =
                gulag_users.find(gulag_user_id).for_update().first(conn)?;

            // Compute new values
            let duration_to_add = Duration::from_secs(additional_duration_secs as u64);
            let new_length = current_user.gulag_length + additional_duration_secs;
            let new_release_at = current_user.release_at.add(duration_to_add);

            // Update atomically
            diesel::update(gulag_users.find(gulag_user_id))
                .set((gulag_length.eq(new_length), release_at.eq(new_release_at)))
                .get_result(conn)
        })
    }

    /// Mark a user as no longer in gulag.
    ///
    /// Takes a connection reference so it can be called within the same
    /// transaction as find_expired.
    pub fn mark_released(
        conn: &mut PgConnection,
        gulag_user_id: i32,
    ) -> Result<usize, diesel::result::Error> {
        use crate::db::schema::gulag_users::dsl::*;

        diesel::update(gulag_users.filter(id.eq(gulag_user_id)))
            .set(in_gulag.eq(false))
            .execute(conn)
    }

    /// Delete a gulag entry
    pub fn delete(pool: &DbPool, gulag_user_id: i32) -> Result<usize, diesel::result::Error> {
        let mut conn = pool.get().map_err(pool_error_to_diesel)?;
        use crate::db::schema::gulag_users::dsl::*;

        diesel::delete(gulag_users.filter(id.eq(gulag_user_id))).execute(&mut conn)
    }

    /// Create a new gulag vote.
    ///
    /// Returns `DatabaseError(UniqueViolation, ...)` if an unprocessed vote
    /// already exists for this sender targeting this requester in the same guild.
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

        // Check for existing unprocessed vote from this sender for this target in this guild
        let existing: Option<GulagVote> = gulag_votes::table
            .filter(gulag_votes::sender_id.eq(sender_id))
            .filter(gulag_votes::requester_id.eq(requester_id))
            .filter(gulag_votes::guild_id.eq(guild_id))
            .filter(gulag_votes::processed.eq(false))
            .first::<GulagVote>(&mut conn)
            .optional()?;

        if existing.is_some() {
            return Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                Box::new("An unprocessed vote already exists for this sender/target/guild".to_string()),
            ));
        }

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

    /// Find message votes that have reached the threshold and need processing.
    ///
    /// Takes a connection reference so the caller can wrap this + update_vote_status
    /// in a single transaction, keeping FOR UPDATE locks held until commit.
    ///
    /// Includes both `Created` and `Done` statuses to support idempotent
    /// re-processing.
    pub fn find_votes_ready_for_processing(
        conn: &mut PgConnection,
        threshold: i32,
    ) -> Result<Vec<MessageVotes>, diesel::result::Error> {
        use crate::db::schema::message_votes::dsl::*;

        let job_status_predicate = job_status
            .eq(JobStatus::Created)
            .or(job_status.eq(JobStatus::Done));

        message_votes
            .filter(current_vote_tally.ge(threshold))
            .filter(job_status_predicate)
            .for_update()
            .skip_locked()
            .load::<MessageVotes>(conn)
    }

    /// Update a message vote's job status.
    ///
    /// Takes a connection reference so it can be called within the same
    /// transaction as find_votes_ready_for_processing.
    pub fn update_vote_status(
        conn: &mut PgConnection,
        target_message_id: i64,
        new_status: JobStatus,
    ) -> Result<MessageVotes, diesel::result::Error> {
        use crate::db::schema::message_votes::dsl::*;

        diesel::update(message_votes.find(target_message_id))
            .set(job_status.eq(new_status))
            .get_result(conn)
    }

    /// Mark a vote as done and reset counters.
    ///
    /// Takes a connection reference so it can be called within the same
    /// transaction as find_votes_ready_for_processing.
    pub fn mark_vote_done(
        conn: &mut PgConnection,
        target_message_id: i64,
        total_votes: i32,
    ) -> Result<MessageVotes, diesel::result::Error> {
        use crate::db::schema::message_votes::dsl::*;

        let empty_vec: Vec<i64> = vec![];
        diesel::update(message_votes.find(target_message_id))
            .set((
                job_status.eq(JobStatus::Done),
                total_vote_tally.eq(total_votes),
                current_vote_tally.eq(0),
                voters.eq(empty_vec),
            ))
            .get_result(conn)
    }
}
