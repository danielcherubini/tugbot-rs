use crate::db::DbPool;
use serenity::all::Http;
use std::sync::Arc;

/// Global application state that is accessible in all commands and handlers
#[derive(Clone)]
pub struct Data {
    /// Database connection pool
    pub db_pool: DbPool,
    /// HTTP client for Discord API calls in background tasks where no
    /// command context is available (e.g. gulag expiry checker, vote processor).
    /// Commands should use `ctx.http()` instead.
    pub http: Arc<Http>,
}

impl Data {
    pub fn new(db_pool: DbPool, http: Arc<Http>) -> Self {
        Self { db_pool, http }
    }
}
