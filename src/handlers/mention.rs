use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use std::{path::Path, sync::Arc, time::Duration, time::SystemTime};

use crate::db::{
    get_is_this_real_usage, get_or_create_is_this_real_usage, get_server_by_guild_id,
    update_is_this_real_usage, DbPool,
};
use crate::features::Features;
use crate::handlers::get_config;
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

/// Map a URL's file extension to a MIME type, defaulting to image/jpeg.
/// Strips the query string and fragment first so URLs like `image.png?v=2`
/// don't get misclassified.
fn mime_for_url(url: &str) -> &'static str {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url);
    match Path::new(path).extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "image/jpeg",
    }
}

/// Build a shared HTTP client with a 10s timeout for image downloads.
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Format a remaining-cooldown duration in human-readable units.
/// Avoids the "0m" bug when fewer than 60 seconds remain.
fn format_remaining(seconds: u64) -> String {
    if seconds >= 3600 {
        let hours = seconds / 3600;
        let mins = (seconds % 3600) / 60;
        if mins > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}h", hours)
        }
    } else if seconds >= 60 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}s", seconds)
    }
}

pub struct Mention;

const COOLDOWN_SECS: u64 = 300; // 5m between uses
const SLOW_COOLDOWN_SECS: u64 = 7_200; // 2h between uses
const GULAG_DURATION_SECS: u32 = 300; // 5 minutes
const SLOW_USER_AUTO_GULAG_FEATURE: &str = "slow_user_auto_gulag";
const ASK_TUGBOT_CHANNEL_ID: u64 = 1515343076401479790; // #ask-tugbot

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

        // 4. Channel restriction — only respond to mentions in #ask-tugbot
        if msg.channel_id.get() != ASK_TUGBOT_CHANNEL_ID {
            return;
        }

        // 5. Config — slow_user_ids only affects the per-user cooldown (longer
        //    cooldown for throttled users). The auto-gulag-on-mention behavior
        //    is gated by the `slow_user_auto_gulag` feature flag (default off
        //    via migration 2026-06-13-200000). When the flag is enabled and
        //    the user is in SLOW_USER_IDS, any mention gulags them. When the
        //    flag is disabled (default), slow users just get the
        //    SLOW_COOLDOWN_SECS cooldown at step 8.
        let config = get_config(ctx).await;
        let slow_user_ids = &config.slow_user_ids;
        let cooldown_exempt_user_ids = &config.cooldown_exempt_user_ids;
        if slow_user_ids.contains(&msg.author.id.get())
            && Features::is_enabled(&pool, SLOW_USER_AUTO_GULAG_FEATURE)
        {
            Mention::handle_slow_user_auto_gulag(&ctx.http, &pool, guild_id.get(), msg).await;
            return;
        }

        // 6. Extract question — strip bot mentions by tokenizing on whitespace
        //    and filtering out anything that looks like <@...> matching the bot ID.
        //    This handles <@ID>, <@!ID>, and avoids any false-positive replace()
        //    matches if a user types text containing "<@".
        let bot_id_str = bot_user.id.get().to_string();
        let question = msg
            .content
            .split_whitespace()
            .filter(|tok| {
                let stripped = tok
                    .trim_start_matches("<@")
                    .trim_start_matches('!')
                    .trim_end_matches('>');
                stripped != bot_id_str
            })
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        eprintln!("[mention] Question: '{}'", question);

        if question.is_empty() {
            if let Err(why) = msg
                .channel_id
                .send_message(
                    &ctx.http,
                    CreateMessage::new()
                        .content("You mentioned me but didn't ask anything — what's up?"),
                )
                .await
            {
                eprintln!("[mention] Failed to send empty question message: {}", why);
            }
            return;
        }

        // 7. Optional: fetch referenced message if this is a reply
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

        // 8. Cooldown check (normal users, admin gets unlimited)
        let user_id = msg.author.id.get();
        let guild_id_u64 = guild_id.get();

        let cooldown_limit = if slow_user_ids.contains(&user_id) {
            SLOW_COOLDOWN_SECS
        } else {
            COOLDOWN_SECS
        };
        if !cooldown_exempt_user_ids.contains(&user_id) {
            if let Some(usage) = get_is_this_real_usage(&pool, user_id as i64, guild_id_u64 as i64)
            {
                let elapsed = SystemTime::now()
                    .duration_since(usage.last_used_at)
                    .unwrap_or_default()
                    .as_secs();

                if elapsed < cooldown_limit {
                    let remaining = cooldown_limit - elapsed;
                    let time_str = format_remaining(remaining);
                    let cooldown_msg = if slow_user_ids.contains(&user_id) {
                        format!(
                            "Easy there, {} — give it a rest for {}",
                            msg.author.mention(),
                            time_str
                        )
                    } else {
                        format!("I'm still waking up — try again in {}", time_str)
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

        // 9. React with :eyes: to acknowledge, then :thinking: while processing
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

        // 10. Download images from referenced message if it exists
        let mut images: Vec<(String, String)> = Vec::new();
        if let Some(ref ref_msg) = referenced_msg {
            let client = http_client();
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
                match client.get(&attachment.url).send().await {
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
                    let content_type = mime_for_url(url).to_string();
                    match client.get(url).send().await {
                        Ok(resp) => match resp.bytes().await {
                            Ok(bytes) => {
                                let b64 = BASE64_STANDARD.encode(&bytes);
                                images.push((content_type, b64));
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

        // 11. Get pi RPC
        let pi_rpc = match (ctx.data.read().await).get::<crate::handlers::PiRpcKey>() {
            Some(rpc) => rpc.clone(),
            None => {
                eprintln!("[mention] pi RPC not available");
                return;
            }
        };

        // 12. Build prompt — include referenced message context
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

        // Don't post or update cooldown for empty responses
        if final_text.is_empty() {
            eprintln!("[mention] pi returned empty response, skipping post and cooldown update");
            let _ = msg
                .channel_id
                .delete_reaction(&ctx.http, msg.id, Some(bot_user.id), '\u{1F914}')
                .await;
            return;
        }

        // 14. Remove thinking emoji and post response
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

        // 15. Update cooldown only if response was delivered — skip exempt users
        if posted && !cooldown_exempt_user_ids.contains(&user_id) {
            let usage_result =
                get_or_create_is_this_real_usage(&pool, user_id as i64, guild_id_u64 as i64);
            if let Ok(u) = usage_result {
                if let Err(e) = update_is_this_real_usage(&pool, u.id) {
                    eprintln!("[mention] Failed to update cooldown: {}", e);
                }
            }
        }
    }

    /// Slow-user auto-gulag handler — fires when the `slow_user_auto_gulag`
    /// feature flag is enabled and the message author is in SLOW_USER_IDS.
    /// Any mention in #ask-tugbot gulags them for GULAG_DURATION_SECS.
    async fn handle_slow_user_auto_gulag(
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
                Ok(Some(c)) => c,
                Ok(None) => {
                    eprintln!("[mention] No gulag channel found");
                    return;
                }
                Err(e) => {
                    eprintln!("[mention] Error looking up gulag channel: {}", e);
                    return;
                }
            };

        let params = GulagParams {
            guildid: guild_id_u64,
            userid: msg.author.id.get(),
            gulag_roleid: match u64::try_from(server.gulag_id) {
                Ok(id) => id,
                Err(_) => {
                    eprintln!("[mention] gulag role ID {} overflows u64", server.gulag_id);
                    return;
                }
            },
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
                eprintln!("[mention] Failed to gulag slow user: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{format_remaining, mime_for_url};

    #[test]
    fn mime_for_url_png() {
        assert_eq!(mime_for_url("https://cdn.discordapp.com/foo.png"), "image/png");
    }

    #[test]
    fn mime_for_url_png_with_query_string() {
        assert_eq!(
            mime_for_url("https://cdn.discordapp.com/foo.png?v=2&hm=abc"),
            "image/png"
        );
    }

    #[test]
    fn mime_for_url_png_with_fragment() {
        assert_eq!(
            mime_for_url("https://cdn.discordapp.com/foo.png#section"),
            "image/png"
        );
    }

    #[test]
    fn mime_for_url_gif() {
        assert_eq!(mime_for_url("https://example.com/a/b/c.gif"), "image/gif");
    }

    #[test]
    fn mime_for_url_webp() {
        assert_eq!(mime_for_url("https://example.com/img.webp"), "image/webp");
    }

    #[test]
    fn mime_for_url_jpg() {
        assert_eq!(mime_for_url("https://example.com/photo.jpg"), "image/jpeg");
    }

    #[test]
    fn mime_for_url_no_extension_defaults_to_jpeg() {
        assert_eq!(mime_for_url("https://example.com/photo"), "image/jpeg");
    }

    #[test]
    fn mime_for_url_unknown_extension_defaults_to_jpeg() {
        assert_eq!(mime_for_url("https://example.com/photo.bmp"), "image/jpeg");
    }

    #[test]
    fn format_remaining_sub_minute_shows_seconds() {
        assert_eq!(format_remaining(59), "59s");
        assert_eq!(format_remaining(1), "1s");
    }

    #[test]
    fn format_remaining_minutes() {
        assert_eq!(format_remaining(60), "1m");
        assert_eq!(format_remaining(3_599), "59m");
    }

    #[test]
    fn format_remaining_hours_with_minutes() {
        assert_eq!(format_remaining(3_600), "1h");
        assert_eq!(format_remaining(3_660), "1h 1m");
        assert_eq!(format_remaining(7_200), "2h");
        assert_eq!(format_remaining(7_260), "2h 1m");
    }
}
