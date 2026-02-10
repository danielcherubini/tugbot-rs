use super::Gulag;
use crate::db::{establish_connection, schema::gulag_users::dsl::*};
use crate::handlers::HandlerResponse;
use diesel::*;
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption, CreateMessage},
    client::Context,
};

pub struct GulagRemoveHandler;

impl GulagRemoveHandler {
    pub fn setup_command() -> CreateCommand {
        CreateCommand::new("gulag-release")
            .description("Release a user from the Gulag")
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "The user to lookup")
                    .required(true),
            )
    }

    pub async fn setup_interaction(ctx: &Context, command: &CommandInteraction) -> HandlerResponse {
        let user_options = match command.data.options.first() {
            Some(opt) => &opt.value,
            None => {
                return HandlerResponse {
                    content: "Expected user option".to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        if let CommandDataOptionValue::User(user) = user_options {
            match command.guild_id {
                None => HandlerResponse {
                    content: "no member".to_string(),
                    components: None,
                    ephemeral: true,
                },
                Some(guildid) => {
                    match Gulag::find_gulag_role(&ctx.http, guildid.get()).await {
                        Some(gulag_role) => {
                            match Gulag::is_user_in_gulag(user.get()) {
                                Some(db_gulag_user) => {
                                    // release
                                    match Gulag::find_channel(
                                        &ctx.http,
                                        guildid.get(),
                                        "the-gulag".to_string(),
                                    )
                                    .await
                                    {
                                        Some(gulag_channel) => {
                                            match ctx
                                                .http
                                                .get_member(
                                                    guildid,
                                                    (db_gulag_user.user_id as u64).into(),
                                                )
                                                .await
                                            {
                                                Ok(member) => {
                                                    if (member
                                                        .remove_role(&ctx.http, gulag_role.id.get())
                                                        .await)
                                                        .is_err()
                                                    {
                                                        return Gulag::send_error(
                                                            "Couldn't remove role",
                                                        );
                                                    };

                                                    let message = format!(
                                                        "Freeing {} from the gulag",
                                                        member
                                                    );

                                                    if (gulag_channel
                                                        .send_message(
                                                            &ctx.http,
                                                            CreateMessage::new().content(message),
                                                        )
                                                        .await)
                                                        .is_err()
                                                    {
                                                        return Gulag::send_error(
                                                            "Couldn't Send message to release",
                                                        );
                                                    };
                                                    let conn = &mut establish_connection();
                                                    match diesel::delete(
                                                        gulag_users.filter(id.eq(db_gulag_user.id)),
                                                    )
                                                    .execute(conn)
                                                    {
                                                        Ok(_) => println!("Removed from database"),
                                                        Err(e) => eprintln!(
                                                            "Failed to delete gulag user from DB: {}",
                                                            e
                                                        ),
                                                    }

                                                    HandlerResponse {
                                                        content: "Releasing User from the Gulag"
                                                            .to_string(),
                                                        components: None,
                                                        ephemeral: true,
                                                    }
                                                }
                                                Err(_) => Gulag::send_error("Couldn't get member"),
                                            }
                                        }
                                        None => Gulag::send_error("Couldn't find Gulag Channel"),
                                    }
                                }
                                None => Gulag::send_error("Couldn't find user in Database"),
                            }
                        }
                        None => Gulag::send_error("Couldn't find gulag Role"),
                    }
                }
            }
        } else {
            HandlerResponse {
                content: "Please provide a valid user".to_string(),
                components: None,
                ephemeral: false,
            }
        }
    }
}
