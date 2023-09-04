use std::time::SystemTime;

use crate::db::schema::*;
use diesel::prelude::*;

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

#[derive(Queryable, Selectable)]
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
}
