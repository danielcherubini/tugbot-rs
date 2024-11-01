use std::time::SystemTime;

use crate::db::schema::gulag_users::dsl::*;
use crate::db::{establish_connection, models::GulagUser};
use crate::handlers::eventhandlers::HandlerResponse;
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
                    .select(GulagUser::as_select())
                    .load(conn)
                    .expect("Error connecting to database");
                let mut userlist = String::from("");
                for gulaguser in gulagusers {
                    let user = ctx
                        .http
                        .get_user(gulaguser.user_id as u64)
                        .await
                        .expect("Couldn't get user");
                    let time_remaining = gulaguser
                        .release_at
                        .duration_since(SystemTime::now())
                        .unwrap();
                    userlist.push_str(&format!(
                        "{}\n{} release in {:?}",
                        userlist, user, time_remaining
                    ));
                }
                let content = format!("Here are the users in the Gulag:{}", userlist);
                HandlerResponse {
                    content,
                    components: None,
                    ephemeral: true,
                }
            }
        }
    }
}
