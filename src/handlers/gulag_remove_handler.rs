use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        prelude::{application_command::CommandDataOptionValue, command::CommandOptionType},
    },
};

use crate::db::{establish_connection, schema::gulag_users::dsl::*};

use diesel::*;

use super::{gulag_handler::GulagHandler, handlers::HandlerResponse};

pub struct GulagRemoveHandler;

impl GulagRemoveHandler {
    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command
            .name("gulag-release")
            .description("Release a user from the Gulag")
            .create_option(|option| {
                option
                    .name("user")
                    .description("The user to lookup")
                    .kind(CommandOptionType::User)
                    .required(true)
            });
    }

    fn send_error(err: &str) -> HandlerResponse {
        return HandlerResponse {
            content: format!("Error: {}", err),
            components: None,
            ephemeral: true,
        };
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        let user_options = command
            .data
            .options
            .get(0)
            .expect("Expected user option")
            .resolved
            .as_ref()
            .expect("Expected user object");

        if let CommandDataOptionValue::User(user, _member) = user_options {
            match command.guild_id {
                None => {
                    return HandlerResponse {
                        content: "no member".to_string(),
                        components: None,
                        ephemeral: true,
                    }
                }
                Some(guildid) => {
                    match GulagHandler::find_gulag_role(&ctx, *guildid.as_u64()).await {
                        Some(gulag_role) => {
                            match GulagHandler::is_user_in_gulag(user.id.0) {
                                Some(db_gulag_user) => {
                                    // release
                                    match GulagHandler::find_gulag_channel(&ctx.http, guildid.0)
                                        .await
                                    {
                                        Some(gulag_channel) => {
                                            match ctx
                                                .http
                                                .get_member(guildid.0, db_gulag_user.user_id as u64)
                                                .await
                                            {
                                                Ok(mut member) => {
                                                    if let Err(_) = member
                                                        .remove_role(&ctx.http, gulag_role.id.0)
                                                        .await
                                                    {
                                                        return GulagRemoveHandler::send_error(
                                                            "Couldn't remove role",
                                                        );
                                                    };

                                                    let message = format!(
                                                        "Freeing {} from the gulag",
                                                        member.to_string()
                                                    );

                                                    if let Err(_) = gulag_channel
                                                        .send_message(ctx, |m| m.content(message))
                                                        .await
                                                    {
                                                        return GulagRemoveHandler::send_error(
                                                            "Couldn't Send message to release",
                                                        );
                                                    };
                                                    let conn = &mut establish_connection();
                                                    diesel::delete(
                                                        gulag_users.filter(id.eq(db_gulag_user.id)),
                                                    )
                                                    .execute(conn)
                                                    .expect("delete user");
                                                    println!("Removed from database");

                                                    return HandlerResponse {
                                                        content: "Releasing User from the Gulag"
                                                            .to_string(),
                                                        components: None,
                                                        ephemeral: true,
                                                    };
                                                }
                                                Err(_) => GulagRemoveHandler::send_error(
                                                    "Couldn't get member",
                                                ),
                                            }
                                        }
                                        None => GulagRemoveHandler::send_error(
                                            "Couldn't find Gulag Channel",
                                        ),
                                    }
                                }
                                None => {
                                    GulagRemoveHandler::send_error("Couldn't find user in Database")
                                }
                            }
                        }
                        None => GulagRemoveHandler::send_error("Couldn't find gulag Role"),
                    }
                }
            }
        } else {
            return HandlerResponse {
                content: "Please provide a valid user".to_string(),
                components: None,
                ephemeral: false,
            };
        }
    }
}
