use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),

    #[error("Database pool error: {0}")]
    DatabasePool(#[from] diesel::r2d2::PoolError),

    #[error("Serenity error: {0}")]
    Serenity(#[from] serenity::Error),

    #[error("User {user_id} not found in guild {guild_id}")]
    UserNotFound { user_id: u64, guild_id: u64 },

    #[error("Role '{role_name}' not found in guild {guild_id}")]
    RoleNotFound { role_name: String, guild_id: u64 },

    #[error("Channel '{channel_name}' not found in guild {guild_id}")]
    ChannelNotFound { channel_name: String, guild_id: u64 },

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Feature '{0}' is not enabled")]
    FeatureDisabled(String),

    /// Anyhow error wrapper for interop with existing code using anyhow.
    ///
    /// Note: The #[from] attribute creates bidirectional conversion between BotError
    /// and anyhow::Error. This means errors can round-trip (BotError -> anyhow -> BotError)
    /// and lose their original variant, becoming Anyhow(...) instead. This is an acceptable
    /// tradeoff for compatibility with existing anyhow-based code.
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, BotError>;

impl From<String> for BotError {
    fn from(s: String) -> Self {
        BotError::Other(s)
    }
}

impl From<&str> for BotError {
    fn from(s: &str) -> Self {
        BotError::Other(s.to_string())
    }
}
