use crate::db::{
    models::{NewServer, Server},
    schema::servers,
    DbPool,
};
use diesel::prelude::*;

/// Helper to convert pool errors to Diesel errors
fn pool_error_to_diesel(e: diesel::r2d2::PoolError) -> diesel::result::Error {
    diesel::result::Error::QueryBuilderError(Box::new(e))
}

pub struct ServerQueries;

impl ServerQueries {
    /// Create a new server entry
    pub fn create(
        pool: &DbPool,
        guild_id: i64,
        gulag_id: i64,
    ) -> Result<Server, diesel::result::Error> {
        let mut conn = pool.get().map_err(pool_error_to_diesel)?;
        let new_server = NewServer { guild_id, gulag_id };

        diesel::insert_into(servers::table)
            .values(&new_server)
            .get_result(&mut conn)
    }

    /// Find a server by guild ID
    pub fn find_by_guild_id(pool: &DbPool, target_guild_id: i64) -> Option<Server> {
        let mut conn = pool.get().ok()?;
        use crate::db::schema::servers::dsl::*;

        servers
            .filter(guild_id.eq(target_guild_id))
            .first::<Server>(&mut conn)
            .ok()
    }

    /// Get all servers
    pub fn all(pool: &DbPool) -> Result<Vec<Server>, diesel::result::Error> {
        let mut conn = pool.get().map_err(pool_error_to_diesel)?;
        use crate::db::schema::servers::dsl::*;

        servers.load(&mut conn)
    }
}
