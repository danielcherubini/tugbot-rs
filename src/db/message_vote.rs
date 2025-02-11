use super::{
    models::{JobStatus, MessageVotes, NewMessageVotes},
    schema::message_votes::{self},
};
use anyhow::{anyhow, Result};
use diesel::prelude::*;
use diesel::PgConnection;

pub struct MessageVoteHandler;
pub enum MessageVoteHanderResponseType {
    ADDED,
    REMOVED,
}
pub struct MessageVoteHandlerResponse {
    pub response_type: MessageVoteHanderResponseType,
    pub content: MessageVotes,
}

impl MessageVoteHandler {
    pub fn message_vote_create_or_update(
        conn: &mut PgConnection,
        message_id: u64,
        guild_id: u64,
        channel_id: u64,
        user_id: u64,
        voter_id: u64,
    ) -> Result<MessageVoteHandlerResponse> {
        let message: Result<Option<MessageVotes>, diesel::result::Error> = message_votes::table
            .find(message_id as i64)
            .select(MessageVotes::as_select())
            .first(conn)
            .optional();

        println!("{:?}", message);
        match message {
            Ok(Some(mut message)) => {
                // Check if the voter_id has already voted
                if message.current_vote_tally < 6 && message.voters.contains(&Some(voter_id as i64))
                {
                    Err(anyhow!("You have already Voted"))
                } else {
                    message.voters.push(Some(voter_id as i64));
                    current_vote_tally = message.current_vote_tally + 1;
                    match diesel::update(message_votes::dsl::message_votes.find(message_id as i64))
                        .set((
                            message_votes::current_vote_tally.eq(current_vote_tally),
                            message_votes::voters.eq(message.voters),
                        ))
                        .get_result(conn)
                    {
                        Ok(c) => Ok(MessageVoteHandlerResponse {
                            response_type: MessageVoteHanderResponseType::ADDED,
                            content: c,
                        }),
                        Err(_) => Err(anyhow!("DB Error whilst trying to add vote")),
                    }
                }
            }
            Ok(None) => {
                let new_message_vote = NewMessageVotes {
                    message_id: message_id as i64,
                    channel_id: channel_id as i64,
                    guild_id: guild_id as i64,
                    user_id: user_id as i64,
                    current_vote_tally: 1,
                    total_vote_tally: 0,
                    voters: [Some(voter_id as i64)].to_vec(),
                    job_status: JobStatus::Created,
                };
                match diesel::insert_into(message_votes::table)
                    .values(&new_message_vote)
                    .returning(MessageVotes::as_returning())
                    .get_result(conn)
                {
                    Ok(c) => Ok(MessageVoteHandlerResponse {
                        response_type: MessageVoteHanderResponseType::ADDED,
                        content: c,
                    }),
                    Err(_) => Err(anyhow!("Database Error Creating Vote")),
                }
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn message_vote_remove(
        conn: &mut PgConnection,
        message_id: u64,
        voter_id: u64,
    ) -> Result<MessageVoteHandlerResponse> {
        let message: Result<Option<MessageVotes>, diesel::result::Error> = message_votes::table
            .find(message_id as i64)
            .select(MessageVotes::as_select())
            .first(conn)
            .optional();

        match message {
            Ok(Some(mut message)) => {
                // Check if the voter_id has already voted
                if !message.voters.contains(&Some(voter_id as i64)) {
                    Err(anyhow!("Not Found in Database"))
                } else {
                    let index = message
                        .voters
                        .iter()
                        .position(|x| *x == Some(voter_id as i64))
                        .unwrap();
                    message.voters.remove(index);
                    if message.current_vote_tally == 0 {
                        message.current_vote_tally = 0;
                    } else {
                        message.current_vote_tally -= 1;
                    }
                    match diesel::update(message_votes::dsl::message_votes.find(message_id as i64))
                        .set((
                            message_votes::current_vote_tally.eq(message.current_vote_tally),
                            message_votes::voters.eq(message.voters),
                        ))
                        .get_result(conn)
                    {
                        Ok(c) => Ok(MessageVoteHandlerResponse {
                            response_type: MessageVoteHanderResponseType::REMOVED,
                            content: c,
                        }),
                        Err(_) => Err(anyhow!("Database Error Removing Vote")),
                    }
                }
            }
            Ok(None) => Err(anyhow!("No vote for this message")),
            Err(e) => Err(e.into()),
        }
    }
}
