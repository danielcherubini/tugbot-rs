use crate::db::{models, schema::features::dsl::*, DbPool};
use anyhow::{Context, Result};
use diesel::prelude::*;
pub struct Features;

impl Features {
    pub fn all(pool: &DbPool) -> Result<Vec<models::Features>> {
        let mut conn = pool.get().expect("Failed to get database connection from pool");
        features
            .load(&mut conn)
            .with_context(|| "Failed to get features")
    }
    pub fn is_enabled(pool: &DbPool, feature_name: &str) -> bool {
        let mut conn = pool.get().expect("Failed to get database connection from pool");
        features
            .filter(name.eq(feature_name))
            .select(enabled)
            .first::<bool>(&mut conn)
            .optional()
            .expect("Error checking feature")
            .unwrap_or(false)
    }

    pub fn update(pool: &DbPool, feature_name: &str, enable: bool) {
        let mut conn = pool.get().expect("Failed to get database connection from pool");
        diesel::update(features.filter(name.eq(feature_name)))
            .set(enabled.eq(enable))
            .execute(&mut conn)
            .expect("Error updating feature");
    }
}
