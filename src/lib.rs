pub mod commands;
pub mod data;
pub mod db;
pub mod error;
pub mod event_handlers;
pub mod features;
pub mod handlers;
pub mod services;
pub mod tasks;
pub mod tugbot;

// Re-export commonly used types
pub use data::Data;
pub use error::{BotError, Result};

// Type aliases for Poise
pub type Error = BotError;
pub type Context<'a> = poise::Context<'a, Data, Error>;
