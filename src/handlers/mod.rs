// pub mod elkmen;
pub mod ai_slop;
pub mod bsky;
pub mod cull;
pub mod derpies;
pub mod elon;
pub mod feat;
pub mod goku_poll;
pub mod gulag;
pub mod instagram;
pub mod mention;
pub mod prefix_handler;
pub mod teh;
pub mod tiktok;
pub mod twitter;

use crate::db::DbPool;
use crate::pi_rpc::PiRpc;
use crate::tugbot::config::Config;
use serenity::prelude::TypeMapKey;
use std::time::{SystemTime, UNIX_EPOCH};

// TypeMapKey for storing the database pool in Serenity's context
pub struct DbPoolKey;

impl TypeMapKey for DbPoolKey {
    type Value = DbPool;
}

// Helper function to get the database pool from context
pub async fn get_pool(ctx: &serenity::client::Context) -> DbPool {
    let data = ctx.data.read().await;
    data.get::<DbPoolKey>()
        .expect("Expected DbPool in TypeMap")
        .clone()
}

// TypeMapKey for storing the bot's Config (wrapped in Arc for cheap clones) in Serenity's context
pub struct ConfigKey;

impl TypeMapKey for ConfigKey {
    type Value = std::sync::Arc<Config>;
}

// Helper function to get an Arc<Config> from context
pub async fn get_config(ctx: &serenity::client::Context) -> std::sync::Arc<Config> {
    let data = ctx.data.read().await;
    data.get::<ConfigKey>()
        .expect("Expected Config in TypeMap")
        .clone()
}

// TypeMapKey for storing the pi RPC client in Serenity's context
pub struct PiRpcKey;

impl TypeMapKey for PiRpcKey {
    type Value = std::sync::Arc<PiRpc>;
}

// Helper function to get the pi RPC client from context
pub async fn get_pi_rpc(ctx: &serenity::client::Context) -> std::sync::Arc<PiRpc> {
    let data = ctx.data.read().await;
    data.get::<PiRpcKey>()
        .expect("Expected PiRpc in TypeMap")
        .clone()
}

use crate::handlers::{
    ai_slop::AiSlopHandler,
    bsky::Bsky,
    cull::CullHandler,
    feat::Feat,
    goku_poll::GokuPoll,
    gulag::{
        gulag_handler::GulagHandler, gulag_list_handler::GulagListHandler,
        gulag_message_command::GulagMessageCommandHandler, gulag_reaction::GulagReaction,
        gulag_remove_handler::GulagRemoveHandler, Gulag,
    },
    mention::Mention,
    prefix_handler::PrefixHandler,
    teh::Teh,
    twitter::Twitter,
};
use crate::tugbot::servers::Servers;
use instagram::Instagram;
use serenity::{
    all::{Interaction, Member, Message, MessageUpdateEvent, Reaction, Ready},
    async_trait,
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage},
    client::{Context, EventHandler},
};

#[derive(Default)]
pub struct HandlerResponse {
    pub content: String,
    pub components: Option<Vec<serenity::all::CreateActionRow>>,
    pub ephemeral: bool,
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        // Passive activity tracking for cull command
        // Skip: bots, webhooks, DMs (no guild)
        // ID conversion failures are rare (overflow) — log and skip tracking,
        // but never exit the handler chain.
        if !msg.author.bot && msg.webhook_id.is_none() {
            if let Some(guild_id) = msg.guild_id {
                let pool = get_pool(&ctx).await;
                let user_id = i64::try_from(msg.author.id.get()).ok();
                let guild_id_i64 = i64::try_from(guild_id.get()).ok();
                if let (Some(uid), Some(gid)) = (user_id, guild_id_i64) {
                    if let Err(e) = crate::db::upsert_user_activity(&pool, uid, gid) {
                        eprintln!("[cull] Failed to upsert user activity: {}", e);
                    }
                }
            }
        }

        Teh::handler(&ctx, &msg).await;
        Twitter::handler(&ctx, &msg).await;
        //TikTok::handler(&ctx, &msg).await;
        Bsky::handler(&ctx, &msg).await;
        Instagram::handler(&ctx, &msg).await;
        Mention::handler(&ctx, &msg).await;
    }

    async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
        GulagReaction::handler(&ctx, &add_reaction).await;
    }

    async fn reaction_remove(&self, ctx: Context, add_reaction: Reaction) {
        GulagReaction::handler(&ctx, &add_reaction).await;
    }

    async fn message_update(
        &self,
        ctx: Context,
        _old_if_available: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        // Try to use the full message if available, otherwise fetch it
        let message = match new {
            Some(msg) => msg,
            None => match ctx.http.get_message(event.channel_id, event.id).await {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("Failed to fetch message for update event: {}", e);
                    return;
                }
            },
        };

        GokuPoll::handle_message_update(&ctx, &message).await;
    }

    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        let pool = get_pool(&ctx).await;
        if let Some(user) = Gulag::is_user_in_gulag(&pool, member.user.id.get()) {
            // Safe conversion with overflow check
            let guild_id = match user.guild_id.try_into() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Guild ID conversion error: {}", e);
                    return;
                }
            };
            let user_id = match user.user_id.try_into() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("User ID conversion error: {}", e);
                    return;
                }
            };
            let gulag_role_id = match user.gulag_role_id.try_into() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Gulag role ID conversion error: {}", e);
                    return;
                }
            };
            let gulag_length = match user.gulag_length.try_into() {
                Ok(len) => len,
                Err(e) => {
                    eprintln!("Gulag length conversion error: {}", e);
                    return;
                }
            };
            let channel_id = match user.channel_id.try_into() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Channel ID conversion error: {}", e);
                    return;
                }
            };

            if let Err(e) = Gulag::add_to_gulag(
                &ctx.http,
                &pool,
                gulag::GulagParams {
                    guildid: guild_id,
                    userid: user_id,
                    gulag_roleid: gulag_role_id,
                    gulaglength: gulag_length,
                    channelid: channel_id,
                    messageid: 0,
                },
            )
            .await
            {
                eprintln!("Failed to re-add user to gulag on rejoin: {}", e);
            }

            let message = format!("You can't escape so easily {}", member);
            if let Ok(channel) = ctx.http.get_channel(channel_id.into()).await {
                if let Err(why) = channel
                    .id()
                    .send_message(&ctx.http, CreateMessage::new().content(message))
                    .await
                {
                    eprintln!("Failed to send gulag escape message: {}", why);
                }
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let handler_response = match command.data.name.as_str() {
                "gulag" => GulagHandler::setup_interaction(&ctx, &command).await,
                "gulag-release" => GulagRemoveHandler::setup_interaction(&ctx, &command).await,
                "gulag-list" => GulagListHandler::setup_interaction(&ctx, &command).await,
                "Add Gulag Vote" => {
                    GulagMessageCommandHandler::setup_interaction(&ctx, &command).await
                }
                "AI Slop" => AiSlopHandler::setup_interaction(&ctx, &command).await,
                "phony" => PrefixHandler::setup_interaction(&ctx, &command).await,
                "horny" => PrefixHandler::setup_interaction(&ctx, &command).await,
                "feature" => Feat::setup_interaction(&ctx, &command).await,
                "cull" => CullHandler::setup_interaction(&ctx, &command).await,
                _ => HandlerResponse {
                    content: "Not Implemented".to_string(),
                    components: None,
                    ephemeral: true,
                },
            };

            let mut message = CreateInteractionResponseMessage::new()
                .content(handler_response.content)
                .ephemeral(handler_response.ephemeral);
            if let Some(components) = handler_response.components {
                message = message.components(components);
            }

            match command
                .create_response(&ctx.http, CreateInteractionResponse::Message(message))
                .await
            {
                Ok(()) => {}
                Err(why) => eprintln!("Cannot respond to slash command: {}", why),
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        eprintln!("{} is connected!", ready.user.name);

        let pool = get_pool(&ctx).await;

        let servers = Servers::get_servers(&ctx, &pool).await;
        Gulag::run_gulag_check(&ctx.http, pool.clone());
        Gulag::run_gulag_vote_check(&ctx.http, pool.clone());

        // Spawn initial activity scan (seeds user_activity table from recent messages)
        // Paginates backwards through each channel until messages are older than SCAN_CUTOFF_DAYS
        let http_clone = ctx.http.clone();
        let pool_clone = pool.clone();
        let servers_clone = servers.clone();

        tokio::spawn(async move {
            // Cutoff: only scan messages newer than this (90 days)
            let cutoff_secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 90 * 86400;

            for server in &servers_clone {
                let guild_id = server.guild_id.get();

                // Fetch all channels for this guild
                let channels = match http_clone.get_channels(guild_id.into()).await {
                    Ok(chs) => chs,
                    Err(e) => {
                        eprintln!(
                            "[cull] Failed to get channels for guild {}: {}",
                            guild_id, e
                        );
                        continue;
                    }
                };

                // Filter to text channels only (skip voice, category, announcement, etc.)
                let text_channels: Vec<_> = channels
                    .into_iter()
                    .filter(|ch| matches!(ch.kind, serenity::all::ChannelType::Text))
                    .collect();

                let mut all_user_pairs: Vec<(i64, i64)> = Vec::new();
                let guild_id_i64 = match i64::try_from(guild_id) {
                    Ok(id) => id,
                    Err(e) => {
                        eprintln!("[cull] Guild ID conversion error for scan: {}", e);
                        continue;
                    }
                };

                for (i, channel) in text_channels.iter().enumerate() {
                    // Paginate backwards through message history until we hit the cutoff
                    let mut before_id: Option<serenity::all::MessageId> = None;
                    let mut channel_msg_count = 0u32;

                    loop {
                        let messages = match http_clone
                            .get_messages(
                                channel.id,
                                before_id.map(serenity::all::MessagePagination::Before),
                                Some(100),
                            )
                            .await
                        {
                            Ok(msgs) => msgs,
                            Err(e) => {
                                eprintln!(
                                    "[cull] Failed to get messages from channel {}: {}",
                                    channel.name, e
                                );
                                break;
                            }
                        };

                        if messages.is_empty() {
                            break;
                        }

                        // Messages are returned newest-first; check oldest (last) message
                        let oldest_timestamp = messages.last().unwrap().timestamp.unix_timestamp();
                        if oldest_timestamp < cutoff_secs as i64 {
                            // Oldest message in this batch is past the cutoff — stop this channel
                            break;
                        }

                        for msg in &messages {
                            if !msg.author.bot && msg.webhook_id.is_none() {
                                let user_id = match i64::try_from(msg.author.id.get()) {
                                    Ok(id) => id,
                                    Err(_) => continue,
                                };
                                all_user_pairs.push((user_id, guild_id_i64));
                            }
                            channel_msg_count += 1;
                        }

                        // If we got fewer than 100 messages, there are no more to fetch
                        if messages.len() < 100 {
                            break;
                        }

                        // Set cursor for next page (oldest message in this batch)
                        before_id = Some(messages.last().unwrap().id);

                        // Be nice with Discord's rate limits (50 req/5s per channel)
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    }

                    eprintln!(
                        "[cull] Scanned {}/{} channels ({} msgs in {}), found {} unique users so far",
                        i + 1,
                        text_channels.len(),
                        channel_msg_count,
                        channel.name,
                        all_user_pairs.len()
                    );
                }

                // Deduplicate
                let mut seen = std::collections::HashSet::new();
                all_user_pairs.retain(|pair| seen.insert(*pair));

                if !all_user_pairs.is_empty() {
                    match crate::db::bulk_upsert_activity(&pool_clone, all_user_pairs) {
                        Ok(rows) => {
                            eprintln!(
                                "[cull] Upserted {} user activity records for guild {}",
                                rows, guild_id
                            );
                        }
                        Err(e) => {
                            eprintln!("[cull] Failed to bulk upsert activity: {}", e);
                        }
                    }
                }
            }

            eprintln!("[cull] Initial activity scan complete");
        });

        // Start pi RPC subprocess
        match PiRpc::spawn().await {
            Ok(pi_rpc) => {
                let mut data = ctx.data.write().await;
                data.insert::<PiRpcKey>(pi_rpc);
                eprintln!("pi RPC subprocess started");
            }
            Err(e) => {
                eprintln!(
                    "Failed to start pi RPC subprocess: {} — mention feature will not work",
                    e
                );
            }
        }

        for server in servers {
            let commands = server
                .guild_id
                .set_commands(
                    &ctx.http,
                    vec![
                        GulagHandler::setup_command(),
                        GulagRemoveHandler::setup_command(),
                        GulagListHandler::setup_command(),
                        GulagMessageCommandHandler::setup_command(),
                        AiSlopHandler::setup_command(),
                        PrefixHandler::setup_command("horny", "Mark yourself as horny/lfg"),
                        PrefixHandler::setup_command("phony", "Mark yourself as phony/watching"),
                        Feat::setup_command(),
                        CullHandler::setup_command(),
                    ],
                )
                .await;

            eprintln!("I now have the following guild slash commands:");
            match commands {
                Ok(commandvec) => {
                    for command in commandvec {
                        eprintln!("{}", command.name)
                    }
                }
                Err(e) => eprintln!("{}", e),
            }
        }
    }
}
