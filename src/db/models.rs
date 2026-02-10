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
