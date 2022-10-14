// @generated automatically by Diesel CLI.

diesel::table! {
    servers (id) {
        id -> Int4,
        guild_id -> Int8,
        gulag_id -> Int8,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Int8,
        in_gulag -> Bool,
    }
}

diesel::allow_tables_to_appear_in_same_query!(servers, users,);
