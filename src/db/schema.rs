// @generated automatically by Diesel CLI.

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
    reversal_of_fortunes,
    servers,
);
