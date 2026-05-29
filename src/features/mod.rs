use crate::db::{models, schema::features::dsl::*, DbPool};
use anyhow::{Context, Result};
use diesel::prelude::*;
pub struct Features;

impl Features {
    pub fn all(pool: &DbPool) -> Result<Vec<models::Features>> {
        let mut conn = pool
            .get()
            .with_context(|| "Failed to get database connection from pool")?;
        features
            .load(&mut conn)
            .with_context(|| "Failed to get features")
    }
    /// Check if a feature is enabled, returning an error if the database is unreachable.
    /// Use this for user-facing commands where you want to report the actual error.
    pub fn check_enabled(pool: &DbPool, feature_name: &str) -> Result<bool> {
        let mut conn = pool
            .get()
            .with_context(|| "Failed to get database connection from pool")?;
        let is_on = features
            .filter(name.eq(feature_name))
            .select(enabled)
            .first::<bool>(&mut conn)
            .optional()
            .with_context(|| format!("Failed to query feature '{}'", feature_name))?;
        Ok(is_on.unwrap_or(false))
    }

    /// Check if a feature is enabled, silently returning false on any error.
    /// Use this for background tasks where you don't want to crash on DB errors.
    pub fn is_enabled(pool: &DbPool, feature_name: &str) -> bool {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to get database connection: {}", e);
                return false;
            }
        };
        features
            .filter(name.eq(feature_name))
            .select(enabled)
            .first::<bool>(&mut conn)
            .optional()
            .unwrap_or_else(|e| {
                eprintln!("Error checking feature '{}': {}", feature_name, e);
                None
            })
            .unwrap_or(false)
    }

    pub fn update(pool: &DbPool, feature_name: &str, enable: bool) -> Result<()> {
        let mut conn = pool
            .get()
            .with_context(|| "Failed to get database connection from pool")?;
        let rows_affected = diesel::update(features.filter(name.eq(feature_name)))
            .set(enabled.eq(enable))
            .execute(&mut conn)
            .with_context(|| format!("Error updating feature '{}'", feature_name))?;

        if rows_affected == 0 {
            anyhow::bail!("Feature '{}' not found in database", feature_name);
        }

        Ok(())
    }
}
