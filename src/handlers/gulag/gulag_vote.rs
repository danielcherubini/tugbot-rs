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
        models::GulagVote, new_gulag_vote, schema::gulag_votes::{self, dsl::*}, DbPool,
    },
    handlers::{get_pool, HandlerResponse},
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
        let pool = get_pool(ctx).await;
        let options = &command
            .data
            .options
            .first()
            .expect("Expected user option")
            .value;

        let guildid = match command.guild_id {
            Some(gid) => gid,
            None => return Gulag::send_error("This command can only be used in a server"),
        };

        let requesterid = match command.member.as_ref() {
            Some(member) => member.user.id.get(),
            None => return Gulag::send_error("This command can only be used in a server"),
        };

        match GulagVoteHandler::gulag_spam_detection(requesterid, &pool).await {
            Ok(_) => {
                if let CommandDataOptionValue::User(user_id) = options {
                    let user = command
                        .data
                        .resolved
                        .users
                        .get(user_id)
                        .expect("User not found");

                    let is_tugbot = Gulag::is_tugbot(&ctx.http, user).await.unwrap_or(false);

                    if is_tugbot {
                        HandlerResponse {
                            content: "Sorry you can't add tugbot to the gulag".to_string(),
                            components: None,
                            ephemeral: true,
                        }
                    } else {
                        match Gulag::find_gulag_role(&ctx.http, guildid.get()).await {
                            None => Gulag::send_error("Couldn't find gulag role"),
                            Some(role) => {
                                let mem = match ctx.http.get_member(guildid, user.id).await {
                                    Ok(m) => m,
                                    Err(_) => {
                                        return Gulag::send_error(
                                            "Could not find member in server",
                                        );
                                    }
                                };

                                let jury_duty_role = match GulagVoteHandler::find_jury_duty_role(
                                    &ctx.http,
                                    guildid.get(),
                                )
                                .await
                                {
                                    Some(role) => role,
                                    None => {
                                        return Gulag::send_error("Couldn't find jury-duty role")
                                    }
                                };

                                let message = format!(
                                    "Should we add {} to the {}?\n{} you have 10 mins to vote",
                                    mem, role, jury_duty_role
                                );

                                HandlerResponse {
                                    content: message,
                                    components: None,
                                    ephemeral: false,
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
        let pool = get_pool(ctx).await;
        let options = &command
            .data
            .options
            .first()
            .expect("Expected user option")
            .value;

        let requesterid = command.member.to_owned().unwrap().user.id.get();

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
                    &pool,
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

    async fn gulag_spam_detection(requesterid: u64, pool: &DbPool) -> Result<()> {
        let mut conn = pool.get().expect("Failed to get database connection from pool");
        let yesterday = SystemTime::now() - Duration::from_secs(3600);
        let results = gulag_votes
            .filter(gulag_votes::created_at.between(yesterday, SystemTime::now()))
            .filter(gulag_votes::requester_id.eq(requesterid as i64))
            .load::<GulagVote>(&mut conn)
            .expect("Error loading Servers");

        if results.len() > 3 {
            bail!("Spam detected")
        }
        Ok(())
    }
}
