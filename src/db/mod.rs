pub mod models;
pub mod schema;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

use self::{
    models::{NewServer, Server},
    schema::servers,
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
        .expect("Error saving new post")
}
