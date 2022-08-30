use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use serenity::{
    client::Context,
    model::prelude::{Channel, GuildId, Message, ReactionType, RoleId},
    utils::MessageBuilder,
};
use tokio::spawn;

use crate::tugbot::server::Server;

pub struct Reactions {
    games: Vec<Game>,
    is_loop_running: AtomicBool,
}
struct Game {
    name: String,
    role_name: String,
    emoji: ReactionType,
}

impl Reactions {
    pub fn new() -> Self {
        Self {
            is_loop_running: AtomicBool::new(false),
            games: Self::build_games(),
        }
    }

    fn build_games() -> Vec<Game> {
        let mut g: Vec<Game> = Vec::new();
        g.push(Game {
            name: "Warzone".to_string(),
            role_name: "@tag-warzone".to_string(),
            emoji: ReactionType::Unicode("👍".to_string()),
        });
        g.push(Game {
            name: "Apex".to_string(),
            role_name: "@tag-apex".to_string(),
            emoji: ReactionType::Unicode("👎".to_string()),
        });
        g.push(Game {
            name: "Fuck".to_string(),
            role_name: "@tag-fuck".to_string(),
            emoji: ReactionType::Unicode("👹".to_string()),
        });
        return g;
    }

    pub async fn setup(&self, ctx: &Context, server: &Server) {
        match Self::find_roles_channel(ctx, server).await {
            Some(channel) => match Self::find_reaction_message(ctx, &channel).await {
                Some(message) => {
                    let mess = channel
                        .id()
                        .edit_message(&ctx.http, message.id, |m| {
                            m.content(self.reaction_role_message())
                        })
                        .await
                        .unwrap();
                    self.add_reactions_to_message(&ctx, mess, server.guild_id)
                        .await;
                }
                None => {
                    let mess = channel
                        .id()
                        .send_message(&ctx.http, |m| m.content(self.reaction_role_message()))
                        .await
                        .unwrap();

                    self.add_reactions_to_message(&ctx, mess, server.guild_id)
                        .await;
                }
            },
            None => {}
        }
    }

    fn reaction_role_message(&self) -> String {
        let mut message = MessageBuilder::new();
        message.push_bold_line("React to this message to play the following");
        for game in &self.games {
            message
                .push(game.name.to_owned())
                .push(": ")
                .push(game.role_name.to_string())
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

    async fn add_reactions_to_message(&self, ctx: &Context, message: Message, guild_id: GuildId) {
        for game in &self.games {
            let _ = message
                .react(&ctx.http, game.emoji.to_owned())
                .await
                .unwrap();
        }
        // self.is_loop_running = AtomicBool::new(false);
        self.poll_reactions(&ctx, message, guild_id).await;
    }

    async fn poll_reactions(&self, ctx: &Context, message: Message, guild_id: GuildId) {
        println!("Beginning Poll");
        match Self::find_role_id(ctx, guild_id).await {
            Some(role) => {
                let http = Arc::clone(&ctx.http);
                let channel_id = message.channel_id.0;
                if !self.is_loop_running.load(Ordering::Relaxed) {
                    println!("Running Loop");
                    spawn(async move {
                        loop {
                            let m = http
                                .get_message(channel_id, *message.id.as_u64())
                                .await
                                .unwrap();
                            for reaction in m.reactions.to_owned() {
                                let reaction_users = http
                                    .get_reaction_users(
                                        channel_id,
                                        *message.id.as_u64(),
                                        &reaction.reaction_type,
                                        10,
                                        Some(0),
                                    )
                                    .await
                                    .unwrap();
                                for user in reaction_users.to_owned() {
                                    if user.name != "tugbot-dev" {
                                        let has_role =
                                            user.has_role(&http, guild_id, role).await.unwrap();
                                        println!("{:#?}", has_role);
                                    }
                                }
                                // for game in Self::build_games() {
                                //     if game.emoji == reaction.reaction_type {
                                //         println!("{:#?}", reaction);
                                //     }
                                // }
                            }

                            // Sleep
                            tokio::time::sleep(Duration::from_secs(10)).await;
                        }
                    });
                    self.is_loop_running.swap(true, Ordering::Relaxed);
                }
            }
            None => {
                println!("No Role Found");
            }
        }
    }

    async fn find_role_id(ctx: &Context, guild_id: GuildId) -> Option<RoleId> {
        let roles = ctx.http.get_guild_roles(guild_id.0).await.unwrap();
        let mut found_role = Some(RoleId::default());
        for role in roles {
            if role.name == "foo" {
                println!("{}", role.name);
                found_role = Some(role.id);
            }
        }
        return found_role;
    }
}
