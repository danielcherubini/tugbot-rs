use serenity::{
    client::Context,
    model::prelude::{Channel, Message, ReactionType},
    utils::MessageBuilder,
};

use crate::tugbot::server::Server;

pub struct Reactions;
struct Game {
    name: String,
    role_name: String,
    emoji: ReactionType,
}

impl Reactions {
    fn setup_games() -> Vec<Game> {
        let mut games: Vec<Game> = Vec::new();
        games.push(Game {
            name: "Warzone".to_string(),
            role_name: "@tag-warzone".to_string(),
            emoji: ReactionType::Unicode("👍".to_string()),
        });
        games.push(Game {
            name: "Apex".to_string(),
            role_name: "@tag-apex".to_string(),
            emoji: ReactionType::Unicode("👎".to_string()),
        });
        games.push(Game {
            name: "Fuck".to_string(),
            role_name: "@tag-fuck".to_string(),
            emoji: ReactionType::Unicode("👹".to_string()),
        });
        return games;
    }

    fn reaction_role_message() -> String {
        let mut message = MessageBuilder::new();
        message.push_bold_line("React to this message to play the following");
        let games = Self::setup_games();
        for game in games {
            message
                .push(game.name)
                .push(": ")
                .push(game.role_name)
                .push(": ")
                .push_line(game.emoji.to_string());
        }

        return message.build();
    }
    async fn find_roles_channel(ctx: &Context, server: &Server) -> Option<Channel> {
        match ctx.http.get_channels(server.guild_id.0).await {
            Err(_why) => None,
            Ok(channels) => {
                for guild_channel in channels {
                    if guild_channel.name == "get-roles" {
                        let chan = ctx.http.get_channel(guild_channel.id.0).await.unwrap();
                        return Some(chan);
                    }
                }
                None
            }
        }
    }

    async fn find_reaction_message(ctx: &Context, channel: &Channel) -> Option<Message> {
        match channel.id().messages(&ctx.http, |b| b.after(0)).await {
            Err(_why) => None,
            Ok(messages) => {
                if messages.len() > 0 {
                    Some(messages[0].to_owned())
                } else {
                    None
                }
            }
        }
    }

    async fn add_reactions_to_message(ctx: &Context, message: Message) {
        let games = Self::setup_games();
        for game in games {
            let _ = message.react(&ctx.http, game.emoji).await.unwrap();
        }
        println!("adding reactions");
        let reaction = message.await_reaction(&ctx.shard).await;
        match reaction {
            Some(r) => {
                let emoji = &r.as_inner_ref().emoji;
                println!("{}", emoji.as_data().to_string());
            }
            None => {
                println!("fuck");
            }
        }
        println!("added reactions");
    }

    pub async fn setup(ctx: &Context, server: &Server) {
        match Self::find_roles_channel(ctx, server).await {
            Some(channel) => match Self::find_reaction_message(ctx, &channel).await {
                Some(message) => {
                    let mess = channel
                        .id()
                        .edit_message(&ctx.http, message.id, |m| {
                            m.content(Self::reaction_role_message())
                        })
                        .await
                        .unwrap();
                    Self::add_reactions_to_message(&ctx, mess).await;
                }
                None => {
                    let mess = channel
                        .id()
                        .send_message(&ctx.http, |m| m.content(Self::reaction_role_message()))
                        .await
                        .unwrap();

                    Self::add_reactions_to_message(&ctx, mess).await;
                }
            },
            None => {}
        }
    }
}
