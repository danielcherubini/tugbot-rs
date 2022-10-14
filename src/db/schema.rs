// @generated automatically by Diesel CLI.

diesel::table! {
    gulag_users (id) {
        id -> Int4,
        user_id -> Int8,
        guild_id -> Int8,
        gulag_role_id -> Int8,
        channel_id -> Int8,
        in_gulag -> Bool,
        created_at -> Timestamp,
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
    servers,
);
