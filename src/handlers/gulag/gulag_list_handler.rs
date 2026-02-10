use std::time::SystemTime;

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
}
