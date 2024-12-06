use crate::db::{establish_connection, models, schema::features::dsl::*};
use anyhow::{Context, Result};
use diesel::prelude::*;
pub struct Features;

impl Features {
    pub fn all() -> Result<Vec<models::Features>> {
        let mut conn = establish_connection();
        features
            .load(&mut conn)
            .with_context(|| "Failed to get features")
    }
    pub fn is_enabled(feature_name: &str) -> bool {
        let mut conn = establish_connection();
        features
            .filter(name.eq(feature_name))
            .select(enabled)
            .first::<bool>(&mut conn)
            .optional()
            .expect("Error checking feature")
            .unwrap_or(false)
    }

    pub fn update(feature_name: &str, enable: bool) {
        let mut conn = establish_connection();
        diesel::update(features.filter(name.eq(feature_name)))
            .set(enabled.eq(enable))
            .execute(&mut conn)
            .expect("Error updating feature");
    }
}
