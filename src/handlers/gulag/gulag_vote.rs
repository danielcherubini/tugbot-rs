use anyhow::{bail, Result};
use diesel::*;
use serenity::{
    all::{
        CommandDataOptionValue, CommandInteraction, CommandOptionType, Message, ReactionType, Role,
    },
    builder::{CreateCommand, CreateCommandOption, EditMessage},
    http::Http,
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
    handlers::HandlerResponse,
};

use super::Gulag;

pub struct GulagVoteHandler;

impl GulagVoteHandler {
    pub fn setup_command() -> CreateCommand {
        CreateCommand::new("gulag-vote")
            .description("Send to the gulag")
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "The user to gulag")
                    .required(true),
            )
    }

    pub async fn setup_interaction(
        ctx: &serenity::client::Context,
        command: &CommandInteraction,
    ) -> HandlerResponse {
        let options = &command
            .data
            .options
            .first()
            .expect("Expected user option")
            .value;

        let requesterid = command.member.to_owned().unwrap().user.id.get();
        let conn = &mut establish_connection();
        match GulagVoteHandler::gulag_spam_detection(requesterid, conn).await {
            Ok(_) => {
                if let CommandDataOptionValue::User(user_id) = options {
                    let user = command
                        .data
                        .resolved
                        .users
                        .get(user_id)
                        .expect("User not found");
                    match command.guild_id {
                        None => Gulag::send_error("no member"),
                        Some(guildid) => {
                            if Gulag::is_tugbot(&ctx.http, user).await.unwrap() {
                                HandlerResponse {
                                    content: "Sorry you can't add tugbot to the gulag".to_string(),
                                    components: None,
                                    ephemeral: true,
                                }
                            } else {
                                match Gulag::find_gulag_role(&ctx.http, guildid.get()).await {
                                    None => Gulag::send_error("Couldn't find gulag role"),
                                    Some(role) => {
                                        let mem =
                                            ctx.http.get_member(guildid, user.id).await.unwrap();

                                        let jury_duty_role =
                                            match GulagVoteHandler::find_jury_duty_role(
                                                &ctx.http,
                                                guildid.get(),
                                            )
                                            .await
                                            {
                                                Some(role) => role,
                                                None => {
                                                    return Gulag::send_error(
                                                        "Couldn't find jury-duty role",
                                                    )
                                                }
                                            };

                                        let message = format!(
                                        "Should we add {} to the {}?\n{} you have 10 mins to vote",
                                        mem,
                                        role,
                                        jury_duty_role);

                                        HandlerResponse {
                                            content: message,
                                            components: None,
                                            ephemeral: false,
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    Gulag::send_error("Please provide a valid user")
                }
            }
            Err(_) => Gulag::send_error(
                "You've used the command too many times, 3 times in an hour is the max",
            ),
        }
    }

    pub async fn process_gulag_votes(
        http: &Arc<Http>,
        channelid: u64,
        messageid: u64,
    ) -> Result<bool> {
        let mut m = http.get_message(channelid.into(), messageid.into()).await?;
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

        m.edit(
            http,
            EditMessage::new().content(format!(
                "{}\nVote over, totals;\nYes: {}\nNo: {}",
                original_content, yay, nay
            )),
        )
        .await?;
        m.delete_reactions(http).await?;

        Ok(yay > nay)
    }

    pub async fn do_followup(
        ctx: &serenity::client::Context,
        command: &CommandInteraction,
        msg: Message,
    ) {
        let options = &command
            .data
            .options
            .first()
            .expect("Expected user option")
            .value;

        let requesterid = command.member.to_owned().unwrap().user.id.get();
        let conn = &mut establish_connection();

        if let CommandDataOptionValue::User(user_id) = options {
            let user = command
                .data
                .resolved
                .users
                .get(user_id)
                .expect("User not found");
            if !Gulag::is_tugbot(&ctx.http, user).await.unwrap() {
                let guildid = command.guild_id.unwrap();
                let role = Gulag::find_gulag_role(&ctx.http, guildid.get())
                    .await
                    .unwrap();

                let _r = msg.react(&ctx, '👍').await.unwrap();
                let _r = msg.react(&ctx, '👎').await.unwrap();

                let _v = new_gulag_vote(
                    conn,
                    requesterid as i64,
                    user_id.get() as i64,
                    guildid.get() as i64,
                    role.id.get() as i64,
                    msg.id.get() as i64,
                    msg.channel_id.get() as i64,
                );
            }
        }
    }

    async fn find_jury_duty_role(http: &Arc<Http>, guildid: u64) -> Option<Role> {
        match http.get_guild_roles(guildid.into()).await {
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

        if results.len() > 3 {
            bail!("Spam detected")
        }
        Ok(())
    }
}
