use crate::db::schema::*;
use diesel::{
    pg::{Pg, PgValue},
    prelude::*,
    AsExpression, FromSqlRow,
};
use serde::{Deserialize, Serialize};
use std::{io::Write, time::SystemTime};

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = ai_slop_usage)]
pub struct AiSlopUsage {
    pub id: i32,
    pub user_id: i64,
    pub guild_id: i64,
    pub usage_count: i32,
    pub last_slop_at: SystemTime,
    pub created_at: SystemTime,
}

#[derive(Insertable)]
#[diesel(table_name = ai_slop_usage)]
pub struct NewAiSlopUsage {
    pub user_id: i64,
    pub guild_id: i64,
    pub usage_count: i32,
    pub last_slop_at: SystemTime,
    pub created_at: SystemTime,
}

#[derive(Queryable)]
pub struct Server {
    pub id: i32,
    pub guild_id: i64,
    pub gulag_id: i64,
}

#[derive(Insertable)]
#[diesel(table_name = servers)]
pub struct NewServer {
    pub guild_id: i64,
    pub gulag_id: i64,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = gulag_users)]
pub struct GulagUser {
    pub id: i32,
    pub user_id: i64,
    pub guild_id: i64,
    pub gulag_role_id: i64,
    pub channel_id: i64,
    pub in_gulag: bool,
    pub gulag_length: i32,
    pub created_at: SystemTime,
    pub release_at: SystemTime,
    pub remod: bool,
    pub message_id: i64,
}

#[derive(Insertable)]
#[diesel(table_name = gulag_users)]
pub struct NewGulagUser {
    pub user_id: i64,
    pub guild_id: i64,
    pub gulag_role_id: i64,
    pub channel_id: i64,
    pub in_gulag: bool,
    pub gulag_length: i32,
    pub created_at: SystemTime,
    pub release_at: SystemTime,
    pub remod: bool,
    pub message_id: i64,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = gulag_votes)]
pub struct GulagVote {
    pub id: i32,
    pub requester_id: i64,
    pub sender_id: i64,
    pub guild_id: i64,
    pub channel_id: i64,
    pub gulag_role_id: i64,
    pub processed: bool,
    pub message_id: i64,
    pub created_at: SystemTime,
}

#[derive(Insertable)]
#[diesel(table_name = gulag_votes)]
pub struct NewGulagVote {
    pub requester_id: i64,
    pub sender_id: i64,
    pub guild_id: i64,
    pub channel_id: i64,
    pub gulag_role_id: i64,
    pub processed: bool,
    pub message_id: i64,
    pub created_at: SystemTime,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = reversal_of_fortunes)]
pub struct ReversalOfFortune {
    pub user_id: i64,
    pub current_percentage: i32,
}

#[derive(Insertable)]
#[diesel(table_name = reversal_of_fortunes)]
pub struct NewReversalOfFortune {
    pub user_id: i64,
    pub current_percentage: i64,
}

#[derive(Debug, PartialEq, FromSqlRow, AsExpression, Eq)]
#[diesel(sql_type = sql_types::JobStatus)]
pub enum JobStatus {
    Created,
    Running,
    Done,
    Failure,
}

impl diesel::serialize::ToSql<sql_types::JobStatus, Pg> for JobStatus {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, Pg>,
    ) -> diesel::serialize::Result {
        match *self {
            JobStatus::Created => out.write_all(b"created")?,
            JobStatus::Running => out.write_all(b"running")?,
            JobStatus::Done => out.write_all(b"done")?,
            JobStatus::Failure => out.write_all(b"failure")?,
        }
        Ok(diesel::serialize::IsNull::No)
    }
}

impl diesel::deserialize::FromSql<sql_types::JobStatus, Pg> for JobStatus {
    fn from_sql(bytes: PgValue<'_>) -> diesel::deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"created" => Ok(JobStatus::Created),
            b"running" => Ok(JobStatus::Running),
            b"done" => Ok(JobStatus::Done),
            b"failure" => Ok(JobStatus::Failure),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = message_votes)]
pub struct MessageVotes {
    pub message_id: i64,
    pub channel_id: i64,
    pub guild_id: i64,
    pub user_id: i64,
    pub current_vote_tally: i32,
    pub voters: Vec<Option<i64>>,
    pub job_status: JobStatus,
    pub total_vote_tally: i32,
}

#[derive(Insertable)]
#[diesel(table_name = message_votes)]
pub struct NewMessageVotes {
    pub message_id: i64,
    pub channel_id: i64,
    pub guild_id: i64,
    pub user_id: i64,
    pub current_vote_tally: i32,
    pub voters: Vec<Option<i64>>,
    pub job_status: JobStatus,
    pub total_vote_tally: i32,
}

#[derive(Queryable, Insertable, Debug, Serialize, Deserialize)]
#[diesel(table_name = features)]
pub struct Features {
    pub id: i32,
    pub name: String,
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_job_status_equality() {
        assert_eq!(JobStatus::Created, JobStatus::Created);
        assert_eq!(JobStatus::Running, JobStatus::Running);
        assert_eq!(JobStatus::Done, JobStatus::Done);
        assert_eq!(JobStatus::Failure, JobStatus::Failure);
        assert_ne!(JobStatus::Created, JobStatus::Running);
    }

    #[test]
    fn test_job_status_debug() {
        // Ensure Debug trait works
        let status = JobStatus::Created;
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("Created"));
    }

    #[test]
    fn test_new_gulag_user_creation() {
        let now = SystemTime::now();
        let user = NewGulagUser {
            user_id: 123456789,
            guild_id: 987654321,
            gulag_role_id: 111222333,
            channel_id: 444555666,
            in_gulag: true,
            gulag_length: 300,
            created_at: now,
            release_at: now + Duration::from_secs(300),
            remod: false,
            message_id: 777888999,
        };
        assert_eq!(user.user_id, 123456789);
        assert_eq!(user.gulag_length, 300);
        assert!(user.in_gulag);
    }

    #[test]
    fn test_gulag_user_time_calculation() {
        let now = SystemTime::now();
        let future = now + Duration::from_secs(600);

        // Simulate a gulag user with release in future
        let duration = future.duration_since(now).unwrap();
        assert!(duration.as_secs() >= 599 && duration.as_secs() <= 601);
    }

    #[test]
    fn test_gulag_user_expired_time() {
        let now = SystemTime::now();
        let past = now - Duration::from_secs(3600);

        // This should error when trying to get duration from past to now
        assert!(past.duration_since(now).is_err());

        // But should work the other way
        let overdue = now.duration_since(past).unwrap();
        assert!(overdue.as_secs() >= 3599 && overdue.as_secs() <= 3601);
    }
}
