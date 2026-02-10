use std::time::{Duration, SystemTime};

use crate::db::schema::gulag_users::dsl::*;
use crate::db::{establish_connection, models::GulagUser};
use crate::handlers::HandlerResponse;
use diesel::*;
use serenity::{
    builder::CreateApplicationCommand, client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
};

pub struct GulagListHandler;

impl GulagListHandler {
    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command
            .name("gulag-list")
            .description("List users in the gulag");
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        match command.guild_id {
            None => HandlerResponse {
                content: "no member".to_string(),
                components: None,
                ephemeral: false,
            },
            Some(_guildid) => {
                let conn = &mut establish_connection();
                let gulagusers = gulag_users
                    .filter(in_gulag.eq(true))
                    .select(GulagUser::as_select())
                    .load(conn)
                    .expect("Error connecting to database");

                if gulagusers.is_empty() {
                    return HandlerResponse {
                        content: "No users currently in the Gulag.".to_string(),
                        components: None,
                        ephemeral: true,
                    };
                }

                let mut userlist = String::from("");
                for gulaguser in gulagusers {
                    let user = ctx
                        .http
                        .get_user(gulaguser.user_id as u64)
                        .await
                        .expect("Couldn't get user");

                    let time_info = match gulaguser.release_at.duration_since(SystemTime::now()) {
                        Ok(duration) => format!("releases in {:?}", duration),
                        Err(_) => {
                            // release_at is in the past
                            let overdue = SystemTime::now()
                                .duration_since(gulaguser.release_at)
                                .unwrap_or_default();
                            format!("overdue for release ({}s ago)", overdue.as_secs())
                        }
                    };

                    userlist.push_str(&format!("\n{} - {}", user, time_info));
                }
                let content = format!("Users in the Gulag:{}", userlist);
                HandlerResponse {
                    content,
                    components: None,
                    ephemeral: true,
                }
            }
        }
    }

    // Helper function to format time remaining/overdue for testing
    pub fn format_time_info(release_time: SystemTime) -> String {
        match release_time.duration_since(SystemTime::now()) {
            Ok(duration) => format!("releases in {:?}", duration),
            Err(_) => {
                let overdue = SystemTime::now()
                    .duration_since(release_time)
                    .unwrap_or_default();
                format!("overdue for release ({}s ago)", overdue.as_secs())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_format_time_info_future_release() {
        // Test with a release time 5 minutes in the future
        let future = SystemTime::now() + Duration::from_secs(300);
        let result = GulagListHandler::format_time_info(future);
        assert!(result.contains("releases in"));
    }

    #[test]
    fn test_format_time_info_past_release() {
        // Test with a release time in the past (should not panic!)
        let past = SystemTime::now() - Duration::from_secs(3600); // 1 hour ago
        let result = GulagListHandler::format_time_info(past);
        assert!(result.contains("overdue for release"));
        assert!(result.contains("3600s ago"));
    }

    #[test]
    fn test_format_time_info_very_old_release() {
        // Test with a very old release time (like the bug we fixed)
        let very_old = SystemTime::now() - Duration::from_secs(41477253); // ~480 days ago
        let result = GulagListHandler::format_time_info(very_old);
        assert!(result.contains("overdue for release"));
        assert!(result.contains("41477253s ago"));
    }

    #[test]
    fn test_format_time_info_exactly_now() {
        // Test with release time approximately now
        let now = SystemTime::now();
        let result = GulagListHandler::format_time_info(now);
        // Should handle either case gracefully (might be slightly past or future due to timing)
        assert!(result.contains("releases in") || result.contains("overdue for release"));
    }
}
