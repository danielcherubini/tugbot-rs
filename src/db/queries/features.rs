use crate::db::{models, pool_error_to_diesel, schema::features::dsl::*, DbPool};
use diesel::prelude::*;

pub struct FeatureQueries;

impl FeatureQueries {
    /// Get all features
    pub fn all(pool: &DbPool) -> Result<Vec<models::Features>, diesel::result::Error> {
        let mut conn = pool.get().map_err(pool_error_to_diesel)?;
        features.load(&mut conn)
    }

    /// Check if a feature is enabled
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

    /// Update a feature's enabled status.
    /// Returns NotFound if the feature doesn't exist.
    pub fn update(
        pool: &DbPool,
        feature_name: &str,
        enable: bool,
    ) -> Result<usize, diesel::result::Error> {
        let mut conn = pool.get().map_err(pool_error_to_diesel)?;
        let rows_affected = diesel::update(features.filter(name.eq(feature_name)))
            .set(enabled.eq(enable))
            .execute(&mut conn)?;

        if rows_affected == 0 {
            return Err(diesel::result::Error::NotFound);
        }

        Ok(rows_affected)
    }
}
