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

pub struct IsThisReal;

const SPECIAL_USER_ID: u64 = 163055057254875136;
const ADMIN_USER_ID: u64 = 212879017257205760;
const COOLDOWN_HOURS: u64 = 8; // 24h / 3 = 8h between uses (3 per day)
const GULAG_DURATION_SECS: u32 = 300; // 5 minutes

impl IsThisReal {
    pub async fn handler(ctx: &Context, msg: &Message) {
        // 1. Feature flag check
        let pool = get_pool(ctx).await;
        if !Features::is_enabled(&pool, "is_this_real") {
            return;
        }
        eprintln!(
            "[is_this_real] Handler called by {} in guild {:?}",
            msg.author.id.get(),
            msg.guild_id.map(|g| g.get())
        );

        // 2. Bot mention check
        let bot_user = match ctx.http.get_current_user().await {
            Ok(user) => user,
            Err(e) => {
                eprintln!("[is_this_real] Failed to get current user: {}", e);
                return;
            }
        };
        if !msg.mentions.iter().any(|m| m.id == bot_user.id) {
            eprintln!("[is_this_real] Bot not mentioned");
            return;
        }

        // 3. Guild ID check (needed for special user)
        let guild_id = match msg.guild_id {
            Some(id) => id,
            None => return,
        };

        // 4. Special user — ANY mention of the bot sends them to gulag
        if msg.author.id.get() == SPECIAL_USER_ID {
            IsThisReal::handle_special_user_gulag(&ctx.http, &pool, guild_id.get(), msg).await;
            return;
        }

        // 5. Reply check
        let referenced_id = match msg.message_reference.as_ref().and_then(|r| r.message_id) {
            Some(id) => id,
            None => {
                eprintln!("[is_this_real] Not a reply");
                return;
            }
        };

        // 6. Fetch referenced message
        let referenced_msg = match ctx.http.get_message(msg.channel_id, referenced_id).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to fetch referenced message: {}", e);
                return;
            }
        };

        // 7. Extract question — strip bot mention
        let bot_mention = format!("<@{}>", bot_user.id.get());
        let bot_mention_with_exclamation = format!("<@!{}>", bot_user.id.get());
        let question = msg
            .content
            .replace(&bot_mention, "")
            .replace(&bot_mention_with_exclamation, "")
            .trim()
            .to_string();
        eprintln!("[is_this_real] Question: '{}'", question);

        if question.is_empty() {
            if let Err(why) = msg
                .channel_id
                .send_message(
                    &ctx.http,
                    CreateMessage::new()
                        .content("Ask me a question about the message you replied to!"),
                )
                .await
            {
                eprintln!("Failed to send empty question message: {}", why);
            }
            return;
        }

        // 8. Fuzzy trigger match — strip punctuation then compare
        let clean = |s: &str| -> String {
            s.to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == ' ')
                .collect::<String>()
                .trim()
                .to_string()
        };
        let cleaned_question = clean(&question);
        let triggers = [
            "is this real",
            "is that real",
            "is this true",
            "is that true",
            "is this legit",
            "is that legit",
        ];
        let mut matched = false;
        for trigger in &triggers {
            let score = rapidfuzz::fuzz::ratio(cleaned_question.chars(), trigger.chars());
            eprintln!(
                "[is_this_real] Fuzzy match '{}' vs '{}': {:.0}%",
                cleaned_question,
                trigger,
                score * 100.0
            );
            if score >= 0.8 {
                matched = true;
                break;
            }
        }
        if !matched {
            eprintln!("[is_this_real] No fuzzy match found");
            return;
        }

        // 9. Cooldown check (normal users, admin gets unlimited)
        let user_id = msg.author.id.get();
        let guild_id_u64 = guild_id.get();

        // Only check existing records — if none exists, user hasn't used the feature yet
        // Admin user skips cooldown entirely
        if user_id != ADMIN_USER_ID {
            if let Some(usage) = get_is_this_real_usage(&pool, user_id as i64, guild_id_u64 as i64)
            {
                let elapsed = SystemTime::now()
                    .duration_since(usage.last_used_at)
                    .unwrap_or_default()
                    .as_secs();
                let cooldown_secs = COOLDOWN_HOURS * 3600;

                if elapsed < cooldown_secs {
                    if let Err(why) = msg
                        .channel_id
                        .send_message(
                            &ctx.http,
                            CreateMessage::new()
                                .content("Come back tomorrow, I need my sleep")
                                .reference_message((msg.channel_id, msg.id)),
                        )
                        .await
                    {
                        eprintln!("Failed to send cooldown message: {}", why);
                    }
                    return;
                }
            }
        }

        // 10. React with :eyes: to acknowledge
        match msg
            .channel_id
            .create_reaction(&ctx.http, msg.id, '\u{1F440}')
            .await
        {
            Ok(_) => eprintln!("[is_this_real] Reacted with :eyes:"),
            Err(e) => eprintln!("[is_this_real] Failed to react: {}", e),
        }

        // 11. Download any image attachments as base64
        let mut images: Vec<(String, String)> = Vec::new();
        for attachment in &referenced_msg.attachments {
            // Only handle images
            let content_type = attachment
                .content_type
                .as_deref()
                .unwrap_or("application/octet-stream");
            if !content_type.starts_with("image/") {
                continue;
            }
            eprintln!(
                "[is_this_real] Downloading image: {} ({})",
                attachment.url, content_type
            );
            match reqwest::get(&attachment.url).await {
                Ok(resp) => match resp.bytes().await {
                    Ok(bytes) => {
                        let b64 = BASE64_STANDARD.encode(&bytes);
                        images.push((content_type.to_string(), b64));
                    }
                    Err(e) => {
                        eprintln!("[is_this_real] Failed to read image bytes: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("[is_this_real] Failed to download image: {}", e);
                }
            }
        }

        // 12. Ask pi via RPC
        let pi_rpc = match (ctx.data.read().await).get::<crate::handlers::PiRpcKey>() {
            Some(rpc) => rpc.clone(),
            None => {
                eprintln!("[is_this_real] pi RPC not available");
                return;
            }
        };

        // If no text but images exist, note the attachment
        let claim_text = if referenced_msg.content.is_empty() && !images.is_empty() {
            format!("[shared an image ({})]", images.len())
        } else {
            referenced_msg.content.clone()
        };

        let prompt = format!(
            "/skill:is-this-real Someone said: \"{}\" — The question is: \"{}\"",
            claim_text, question
        );

        let final_text = match pi_rpc.ask_with_images(&prompt, &images).await {
            Ok(text) => text.trim().to_string(),
            Err(e) => {
                eprintln!("[is_this_real] pi RPC ask failed: {}", e);
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
                    eprintln!("[is_this_real] Failed to send error message: {}", why);
                }
                return;
            }
        };

        // 13. Post response (reply to the user's question)
        eprintln!("[is_this_real] Posting response...");
        match msg
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new()
                    .content(final_text.trim())
                    .reference_message((msg.channel_id, msg.id)),
            )
            .await
        {
            Ok(_) => eprintln!("[is_this_real] Response posted"),
            Err(why) => eprintln!("[is_this_real] Failed to post response: {}", why),
        }

        // 13. Update cooldown (fire and forget) — skip for admin
        if user_id != ADMIN_USER_ID {
            let usage_result =
                get_or_create_is_this_real_usage(&pool, user_id as i64, guild_id_u64 as i64);
            if let Ok(u) = usage_result {
                if let Err(e) = update_is_this_real_usage(&pool, u.id) {
                    eprintln!("Failed to update cooldown: {}", e);
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
                    "[is_this_real] No server config for guild {} (or DB unavailable)",
                    guild_id_u64
                );
                return;
            }
        };

        let gulag_channel =
            match Gulag::find_channel(http, guild_id_u64, "the-gulag".to_string()).await {
                Some(c) => c,
                None => {
                    eprintln!("[is_this_real] No gulag channel found");
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
                    eprintln!("[is_this_real] Failed to send gulag message: {}", why);
                }
            }
            Err(e) => {
                eprintln!("[is_this_real] Failed to gulag special user: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rapidfuzz::fuzz;

    #[test]
    fn test_rapidfuzz_ratio_scale() {
        // rapidfuzz::fuzz::ratio returns 0.0-1.0
        let perfect = fuzz::ratio("is this real".chars(), "is this real".chars());
        assert!(
            (perfect - 1.0).abs() < 0.001,
            "perfect match should be 1.0, got {}",
            perfect
        );

        let one_off = fuzz::ratio("is this reai".chars(), "is this real".chars());
        assert!(
            one_off > 0.9,
            "one char diff should be >90%, got {}",
            one_off
        );

        let diff = fuzz::ratio("something else".chars(), "is this real".chars());
        assert!(diff < 0.6, "unrelated strings should be <60%, got {}", diff);
    }

    #[test]
    fn test_fuzzy_trigger_matching() {
        let clean = |s: &str| -> String {
            s.to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == ' ')
                .collect::<String>()
                .trim()
                .to_string()
        };

        let triggers = [
            "is this real",
            "is that real",
            "is this true",
            "is that true",
            "is this legit",
            "is that legit",
        ];

        let test_inputs = vec![
            ("is this reaI?", true),
            ("is this real?", true),
            ("is this real", true),
            ("is that legit?", true),
            ("is dis real", true),
            ("what's for dinner", false),
            ("hello world", false),
        ];

        for (input, should_match) in test_inputs {
            let cleaned = clean(input);
            let mut matched = false;
            for trigger in &triggers {
                let score = fuzz::ratio(cleaned.chars(), trigger.chars());
                if score >= 0.8 {
                    matched = true;
                    break;
                }
            }
            assert_eq!(
                matched, should_match,
                "input='{}' cleaned='{}'",
                input, cleaned
            );
        }
    }
}
