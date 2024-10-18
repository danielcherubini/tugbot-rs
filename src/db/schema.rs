// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "job_status"))]
    pub struct JobStatus;
}

diesel::table! {
    gulag_users (id) {
        id -> Int4,
        user_id -> Int8,
        guild_id -> Int8,
        gulag_role_id -> Int8,
        channel_id -> Int8,
        in_gulag -> Bool,
        gulag_length -> Int4,
        created_at -> Timestamp,
        release_at -> Timestamp,
        remod -> Bool,
    }
}

diesel::table! {
    gulag_votes (id) {
        id -> Int4,
        requester_id -> Int8,
        sender_id -> Int8,
        guild_id -> Int8,
        channel_id -> Int8,
        gulag_role_id -> Int8,
        processed -> Bool,
        message_id -> Int8,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::JobStatus;

    message_votes (message_id) {
        message_id -> Int8,
        channel_id -> Int8,
        guild_id -> Int8,
        user_id -> Int8,
        total_vote_tally -> Int4,
        voters -> Array<Nullable<Int8>>,
        job_status -> JobStatus,
        current_vote_tally -> Int4,
    }
}

diesel::table! {
    reversal_of_fortunes (user_id) {
        user_id -> Int8,
        current_percentage -> Int8,
    }
}

diesel::table! {
    servers (id) {
        id -> Int4,
        guild_id -> Int8,
        gulag_id -> Int8,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    gulag_users,
    gulag_votes,
    message_votes,
    reversal_of_fortunes,
    servers,
);
