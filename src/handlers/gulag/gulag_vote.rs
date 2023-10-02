use anyhow::{bail, Result};
use diesel::*;
use serenity::{
    builder::CreateApplicationCommand,
    http::Http,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        channel::{Message, ReactionType},
        prelude::{application_command::CommandDataOptionValue, command::CommandOptionType, Role},
    },
};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::{
    db::{
        establish_connection,
        models::GulagVote,
        new_gulag_vote,
        schema::gulag_votes::{self, dsl::*},
    },
    handlers::handlers::HandlerResponse,
};

use super::Gulag;

pub struct GulagVoteHandler;

impl GulagVoteHandler {
    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command
            .name("gulag-vote")
            .description("Send to the gulag")
            .create_option(|option| {
                option
                    .name("user")
                    .description("The user to gulag")
                    .kind(CommandOptionType::User)
                    .required(true)
            });
    }

    pub async fn setup_interaction(
        ctx: &serenity::client::Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        let options = command
            .data
            .options
            .get(0)
            .expect("Expected user option")
            .resolved
            .as_ref()
            .expect("Expected user object");

        let requesterid = command.member.to_owned().unwrap().user.id.0;
        let conn = &mut establish_connection();
        match GulagVoteHandler::gulag_spam_detection(requesterid, conn).await {
            Ok(_) => {
                if let CommandDataOptionValue::User(user, _member) = options {
                    match command.guild_id {
                        None => return Gulag::send_error("no member"),
                        Some(guildid) => {
                            if Gulag::is_tugbot(&ctx.http, &user).await.unwrap() {
                                return HandlerResponse {
                                    content: format!("Sorry you can't add tugbot to the gulag"),
                                    components: None,
                                    ephemeral: true,
                                };
                            } else {
                                match Gulag::find_gulag_role(&ctx.http, *guildid.as_u64()).await {
                                    None => return Gulag::send_error("Couldn't find gulag role"),
                                    Some(role) => {
                                        let mem = ctx
                                            .http
                                            .get_member(*guildid.as_u64(), *user.id.as_u64())
                                            .await
                                            .unwrap();

                                        let jury_duty_role = GulagVoteHandler::find_jury_duty_role(
                                            &ctx.http, guildid.0,
                                        )
                                        .await
                                        .unwrap();

                                        let message = format!(
                                        "Should we add {} to the {}?\n{} you have 10 mins to vote",
                                        mem.to_string(),
                                        role.to_string(),
                                        jury_duty_role.to_string(),
                                    );

                                        return HandlerResponse {
                                            content: message,
                                            components: None,
                                            ephemeral: false,
                                        };
                                    }
                                }
                            }
                        }
                    }
                } else {
                    return Gulag::send_error("Please provide a valid user");
                };
            }
            Err(_) => {
                return Gulag::send_error(
                    "You've used the command too many times, 3 times in an hour is the max",
                )
            }
        }
    }

    pub async fn process_gulag_votes(
        http: &Arc<Http>,
        channelid: u64,
        messageid: u64,
    ) -> Result<bool> {
        let mut m = http.get_message(channelid, messageid).await?;
        let mut yay = 0;
        let mut nay = 0;
        let original_content = m.content.to_owned();
        let reactions = m.reactions.to_owned();
        for reaction in reactions {
            if reaction.reaction_type == ReactionType::from('👍') {
                yay = reaction.count - 1;
            }
            if reaction.reaction_type == ReactionType::from('👎') {
                nay = reaction.count - 1;
            }
        }

        m.edit(http, |msg| {
            msg.content(format!(
                "{}\nVote over, totals;\nYes: {}\nNo: {}",
                original_content, yay, nay
            ))
        })
        .await?;
        m.delete_reactions(http).await?;

        Ok(yay > nay)
    }

    pub async fn do_followup(
        ctx: &serenity::client::Context,
        command: &ApplicationCommandInteraction,
        msg: Message,
    ) {
        let options = command
            .data
            .options
            .get(0)
            .expect("Expected user option")
            .resolved
            .as_ref()
            .expect("Expected user object");

        let requesterid = command.member.to_owned().unwrap().user.id.0;
        let conn = &mut establish_connection();

        if let CommandDataOptionValue::User(user, _member) = options {
            if !Gulag::is_tugbot(&ctx.http, &user).await.unwrap() {
                let guildid = command.guild_id.unwrap();
                let role = Gulag::find_gulag_role(&ctx.http, guildid.0).await.unwrap();

                let _r = msg.react(&ctx, '👍').await.unwrap();
                let _r = msg.react(&ctx, '👎').await.unwrap();

                let _v = new_gulag_vote(
                    conn,
                    requesterid as i64,
                    user.id.0 as i64,
                    guildid.0 as i64,
                    role.id.0 as i64,
                    msg.id.0 as i64,
                    msg.channel_id.0 as i64,
                );
            }
        }
    }

    async fn find_jury_duty_role(http: &Arc<Http>, guildid: u64) -> Option<Role> {
        match http.get_guild_roles(guildid).await {
            Err(_why) => None,
            Ok(roles) => {
                for role in roles {
                    if role.name == "jury-duty" {
                        return Some(role);
                    }
                }
                None
            }
        }
    }

    async fn gulag_spam_detection(requesterid: u64, conn: &mut PgConnection) -> Result<()> {
        let yesterday = SystemTime::now() - Duration::from_secs(3600);
        let results = gulag_votes
            .filter(gulag_votes::created_at.between(yesterday, SystemTime::now()))
            .filter(gulag_votes::requester_id.eq(requesterid as i64))
            .load::<GulagVote>(conn)
            .expect("Error loading Servers");

        if results.len() > 300 {
            bail!("Spam detected")
        }
        Ok(())
    }
}
