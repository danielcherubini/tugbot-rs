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

#[derive(Queryable)]
pub struct User {
    pub user_id: i64,
    pub in_gulag: bool,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub user_id: i64,
    pub in_gulag: bool,
}
