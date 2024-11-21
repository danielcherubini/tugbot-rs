use crate::db::schema::features::dsl::*;
use diesel::prelude::*;
use diesel::PgConnection;
pub struct Feats;

impl Feats {
    pub fn enabled(conn: &mut PgConnection, feature_name: String) -> bool {
        let feature_enabled = features
            .filter(name.eq(feature_name))
            .select(enabled)
            .first::<bool>(conn)
            .optional()
            .expect("Error checking feature")
            .unwrap_or(false);
        return feature_enabled;
    }

    pub fn update(conn: &mut PgConnection, feature_name: String) {
        // Update a feature's status
        diesel::update(features.filter(name.eq(feature_name)))
            .set(enabled.eq(true))
            .execute(conn)
            .expect("Error updating feature");
    }
}
