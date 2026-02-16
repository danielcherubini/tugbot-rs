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
