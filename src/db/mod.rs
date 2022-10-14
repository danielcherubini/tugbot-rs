pub mod models;
pub mod schema;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::{env, time::SystemTime};

use self::{
    models::{GulagUser, NewGulagUser, NewServer, Server},
    schema::{gulag_users, servers},
};

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn create_server(conn: &mut PgConnection, guild_id: i64, gulag_id: i64) -> Server {
    let new_server = NewServer { guild_id, gulag_id };

    diesel::insert_into(servers::table)
        .values(&new_server)
        .get_result(conn)
        .expect("Error saving new server")
}

pub fn send_to_gulag(
    conn: &mut PgConnection,
    user_id: i64,
    guild_id: i64,
    gulag_role_id: i64,
    channel_id: i64,
) -> GulagUser {
    let new_user = NewGulagUser {
        user_id,
        guild_id,
        gulag_role_id,
        channel_id,
        in_gulag: true,
        created_at: SystemTime::now(),
    };

    diesel::insert_into(gulag_users::table)
        .values(&new_user)
        .get_result(conn)
        .expect("Error saving new User")
}
