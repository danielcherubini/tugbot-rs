use crate::db::schema::features::dsl::*;
use diesel::prelude::*;
pub struct Features;

impl Features {
    pub fn is_enabled(feature_name: String) -> bool {
        let mut conn = super::establish_connection();
        features
            .filter(name.eq(feature_name))
            .select(enabled)
            .first::<bool>(&mut conn)
            .optional()
            .expect("Error checking feature")
            .unwrap_or(false)
    }

    pub fn update(feature_name: String, enable: bool) {
        let mut conn = super::establish_connection();
        diesel::update(features.filter(name.eq(feature_name)))
            .set(enabled.eq(enable))
            .execute(&mut conn)
            .expect("Error updating feature");
    }
}
