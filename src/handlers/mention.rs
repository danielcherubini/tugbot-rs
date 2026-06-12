use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use std::{sync::Arc, time::SystemTime};

use crate::db::{
    get_is_this_real_usage, get_or_create_is_this_real_usage, get_server_by_guild_id,
    update_is_this_real_usage, DbPool,
};
use crate::features::Features;
use crate::handlers::get_pool;
use crate::handlers::gulag::{Gulag, GulagParams};
use serenity::{
    all::{Http, Mentionable},
    builder::CreateMessage,
    model::prelude::Message,
    prelude::Context,
};

/// Check if a URL uses http or https scheme.
fn is_safe_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

pub struct Mention;

const SPECIAL_USER_ID: u64 = 163055057254875136;
const ADMIN_USER_ID: u64 = 212879017257205760;
const RESTRICTED_USER_ID: u64 = 776223452758540308;
const COOLDOWN_SECS: u64 = 7_200; // 2h between uses (12 per day)
const RESTRICTED_COOLDOWN_SECS: u64 = 86_400; // 24h (1 per day)
const GULAG_DURATION_SECS: u32 = 300; // 5 minutes

impl Mention {
    pub async fn handler(ctx: &Context, msg: &Message) {
        // 1. Feature flag check
        // Note: DB key is still "is_this_real" for backward compat — rename via migration later
        let pool = get_pool(ctx).await;
        if !Features::is_enabled(&pool, "is_this_real") {
            return;
        }
        eprintln!(
            "[mention] Handler called by {} in guild {:?}",
            msg.author.id.get(),
            msg.guild_id.map(|g| g.get())
        );

        // 2. Bot mention check
        let bot_user = match ctx.http.get_current_user().await {
            Ok(user) => user,
            Err(e) => {
                eprintln!("[mention] Failed to get current user: {}", e);
                return;
            }
        };
        if !msg.mentions.iter().any(|m| m.id == bot_user.id) {
            return;
        }

        // 3. Guild ID check (needed for special user)
        let guild_id = match msg.guild_id {
            Some(id) => id,
            None => return,
        };

        // 4. Special user — ANY mention of the bot sends them to gulag
        if msg.author.id.get() == SPECIAL_USER_ID {
            Mention::handle_special_user_gulag(&ctx.http, &pool, guild_id.get(), msg).await;
            return;
        }

        // 5. Extract question — strip bot mention
        let bot_mention = format!("<@{}>", bot_user.id.get());
        let bot_mention_with_exclamation = format!("<@!{}>", bot_user.id.get());
        let question = msg
            .content
            .replace(&bot_mention, "")
            .replace(&bot_mention_with_exclamation, "")
            .trim()
            .to_string();
        eprintln!("[mention] Question: '{}'", question);

        if question.is_empty() {
            if let Err(why) = msg
                .channel_id
                .send_message(
                    &ctx.http,
                    CreateMessage::new()
                        .content("You mentioned me for no reason — try asking something."),
                )
                .await
            {
                eprintln!("[mention] Failed to send empty question message: {}", why);
            }
            return;
        }

        // 6. Optional: fetch referenced message if this is a reply
        let referenced_msg = match msg.message_reference.as_ref().and_then(|r| r.message_id) {
            Some(referenced_id) => {
                match ctx.http.get_message(msg.channel_id, referenced_id).await {
                    Ok(m) => Some(m),
                    Err(e) => {
                        eprintln!("[mention] Failed to fetch referenced message: {}", e);
                        None
                    }
                }
            }
            None => None,
        };

        // 7. Cooldown check (normal users, admin gets unlimited)
        let user_id = msg.author.id.get();
        let guild_id_u64 = guild_id.get();

        let cooldown_limit = if user_id == RESTRICTED_USER_ID {
            RESTRICTED_COOLDOWN_SECS
        } else {
            COOLDOWN_SECS
        };
        if user_id != ADMIN_USER_ID {
            if let Some(usage) = get_is_this_real_usage(&pool, user_id as i64, guild_id_u64 as i64)
            {
                let elapsed = SystemTime::now()
                    .duration_since(usage.last_used_at)
                    .unwrap_or_default()
                    .as_secs();

                if elapsed < cooldown_limit {
                    let remaining = cooldown_limit - elapsed;
                    let hours = remaining / 3600;
                    let mins = (remaining % 3600) / 60;
                    let cooldown_msg = if hours > 0 {
                        format!("I'm still sleeping — try again in {}h {}m", hours, mins)
                    } else {
                        format!("I'm still sleeping — try again in {}m", mins)
                    };
                    if let Err(why) = msg
                        .channel_id
                        .send_message(
                            &ctx.http,
                            CreateMessage::new()
                                .content(cooldown_msg)
                                .reference_message((msg.channel_id, msg.id)),
                        )
                        .await
                    {
                        eprintln!("[mention] Failed to send cooldown message: {}", why);
                    }
                    return;
                }
            }
        }

        // 8. React with :eyes: to acknowledge, then :thinking: while processing
        match msg
            .channel_id
            .create_reaction(&ctx.http, msg.id, '\u{1F440}')
            .await
        {
            Ok(_) => eprintln!("[mention] Reacted with :eyes:"),
            Err(e) => eprintln!("[mention] Failed to react: {}", e),
        }
        match msg
            .channel_id
            .create_reaction(&ctx.http, msg.id, '\u{1F914}') // 🤔
            .await
        {
            Ok(_) => eprintln!("[mention] Reacted with 🤔"),
            Err(e) => eprintln!("[mention] Failed to react: {}", e),
        }

        // 9. Download images from referenced message if it exists
        let mut images: Vec<(String, String)> = Vec::new();
        if let Some(ref ref_msg) = referenced_msg {
            for attachment in &ref_msg.attachments {
                let content_type = attachment
                    .content_type
                    .as_deref()
                    .unwrap_or("application/octet-stream");
                if !content_type.starts_with("image/") {
                    continue;
                }
                if !is_safe_url(&attachment.url) {
                    eprintln!("[mention] Skipping unsafe URL: {}", attachment.url);
                    continue;
                }
                eprintln!(
                    "[mention] Downloading image: {} ({})",
                    attachment.url, content_type
                );
                match reqwest::get(&attachment.url).await {
                    Ok(resp) => match resp.bytes().await {
                        Ok(bytes) => {
                            let b64 = BASE64_STANDARD.encode(&bytes);
                            images.push((content_type.to_string(), b64));
                        }
                        Err(e) => {
                            eprintln!("[mention] Failed to read image bytes: {}", e);
                        }
                    },
                    Err(e) => {
                        eprintln!("[mention] Failed to download image: {}", e);
                    }
                }
            }

            // Also grab images from embeds
            for embed in &ref_msg.embeds {
                let mut embed_image_url = embed.image.as_ref().map(|i| &i.url as &str);
                if embed_image_url.is_none() {
                    embed_image_url = embed.thumbnail.as_ref().map(|t| &t.url as &str);
                }
                if let Some(url) = embed_image_url {
                    if ref_msg.attachments.iter().any(|a| a.url == *url) {
                        continue;
                    }
                    if !is_safe_url(url) {
                        eprintln!("[mention] Skipping unsafe embed URL: {}", url);
                        continue;
                    }
                    eprintln!("[mention] Downloading embed image: {}", url);
                    match reqwest::get(url).await {
                        Ok(resp) => match resp.bytes().await {
                            Ok(bytes) => {
                                let b64 = BASE64_STANDARD.encode(&bytes);
                                let ext = url.rsplit('.').next().unwrap_or("jpeg");
                                let content_type = match ext {
                                    "png" => "image/png",
                                    "gif" => "image/gif",
                                    "webp" => "image/webp",
                                    _ => "image/jpeg",
                                };
                                images.push((content_type.to_string(), b64));
                            }
                            Err(e) => {
                                eprintln!("[mention] Failed to read embed image bytes: {}", e);
                            }
                        },
                        Err(e) => {
                            eprintln!("[mention] Failed to download embed image: {}", e);
                        }
                    }
                }
            }
        }

        // 10. Ask pi via RPC
        let pi_rpc = match (ctx.data.read().await).get::<crate::handlers::PiRpcKey>() {
            Some(rpc) => rpc.clone(),
            None => {
                eprintln!("[mention] pi RPC not available");
                return;
            }
        };

        // Build prompt — include referenced message context if it exists
        let prompt = match &referenced_msg {
            Some(ref_msg) => {
                let context = match (!ref_msg.content.is_empty(), !images.is_empty()) {
                    (true, true) => format!("{} [also shared an image]", ref_msg.content),
                    (false, true) => format!("[shared an image ({})]", images.len()),
                    (true, false) => ref_msg.content.clone(),
                    (false, false) => String::from("[replied to an image]"),
                };
                format!(
                    "{} replied to: \"{}\" and asked: \"{}\"",
                    msg.author.name, context, question
                )
            }
            None => {
                format!(
                    "{} asked: \"{}\"",
                    msg.author.name, question
                )
            }
        };

        let final_text = match pi_rpc.ask_with_images(&prompt, &images).await {
            Ok(text) => text.trim().to_string(),
            Err(e) => {
                eprintln!("[mention] pi RPC ask failed: {}", e);
                let _ = msg
                    .channel_id
                    .delete_reaction(&ctx.http, msg.id, Some(bot_user.id), '\u{1F914}')
                    .await;
                if let Err(why) = msg
                    .channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new()
                            .content("I'm having trouble thinking right now, try again later")
                            .reference_message((msg.channel_id, msg.id)),
                    )
                    .await
                {
                    eprintln!("[mention] Failed to send error message: {}", why);
                }
                return;
            }
        };

        // 11. Remove thinking emoji and post response
        let _ = msg
            .channel_id
            .delete_reaction(&ctx.http, msg.id, Some(bot_user.id), '\u{1F914}')
            .await;
        eprintln!("[mention] Posting response...");
        let posted = match msg
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new()
                    .content(final_text.trim())
                    .reference_message((msg.channel_id, msg.id)),
            )
            .await
        {
            Ok(_) => {
                eprintln!("[mention] Response posted");
                true
            }
            Err(why) => {
                eprintln!("[mention] Failed to post response: {}", why);
                false
            }
        };

        // 12. Update cooldown only if response was delivered — skip for admin
        if posted && user_id != ADMIN_USER_ID {
            let usage_result =
                get_or_create_is_this_real_usage(&pool, user_id as i64, guild_id_u64 as i64);
            if let Ok(u) = usage_result {
                if let Err(e) = update_is_this_real_usage(&pool, u.id) {
                    eprintln!("[mention] Failed to update cooldown: {}", e);
                }
            }
        }
    }

    /// Special user gulag handler — triggers on ANY bot mention, no reply or keyword needed
    async fn handle_special_user_gulag(
        http: &Arc<Http>,
        pool: &DbPool,
        guild_id_u64: u64,
        msg: &Message,
    ) {
        let server = match get_server_by_guild_id(pool, guild_id_u64 as i64) {
            Some(s) => s,
            None => {
                eprintln!(
                    "[mention] No server config for guild {} (or DB unavailable)",
                    guild_id_u64
                );
                return;
            }
        };

        let gulag_channel =
            match Gulag::find_channel(http, guild_id_u64, "the-gulag".to_string()).await {
                Some(c) => c,
                None => {
                    eprintln!("[mention] No gulag channel found");
                    return;
                }
            };

        let params = GulagParams {
            guildid: guild_id_u64,
            userid: msg.author.id.get(),
            gulag_roleid: server.gulag_id as u64,
            gulaglength: GULAG_DURATION_SECS,
            channelid: gulag_channel.id.get(),
            messageid: msg.id.get(),
        };

        match Gulag::add_to_gulag(http, pool, params).await {
            Ok(_) => {
                if let Err(why) = gulag_channel
                    .id
                    .send_message(
                        http,
                        CreateMessage::new().content(format!(
                            "{} wanted to know if something was real... now they're in the gulag for 5m. Irony.",
                            msg.author.mention()
                        )),
                    )
                    .await
                {
                    eprintln!("[mention] Failed to send gulag message: {}", why);
                }
            }
            Err(e) => {
                eprintln!("[mention] Failed to gulag special user: {}", e);
            }
        }
    }
}
