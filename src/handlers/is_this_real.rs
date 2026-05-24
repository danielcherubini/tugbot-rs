use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

use crate::db::{
    get_or_create_is_this_real_usage, get_is_this_real_usage, get_server_by_guild_id,
    update_is_this_real_usage,
};
use crate::exa;
use crate::features::Features;
use crate::handlers::get_pool;
use crate::handlers::gulag::{Gulag, GulagParams};
use serenity::{
    all::Mentionable, builder::CreateMessage, model::prelude::Message, prelude::Context,
};

pub struct IsThisReal;

const SPECIAL_USER_ID: u64 = 163055057254875136;
const COOLDOWN_HOURS: u64 = 24;
const GULAG_DURATION_SECS: u32 = 300; // 5 minutes

const SYSTEM_PROMPT: &str = "You are Tugbot, a Discord bot that fact-checks claims. A user has asked you a question about something someone else said. Respond in one or two sentences max. Try to be funny, sarcastic, or sardonic when possible. Be helpful but keep it brief.";

const OLLAMA_URL: &str = "http://tama:11434/v1/chat/completions";
const OLLAMA_MODEL: &str = "whatevers-hot-n-fresh";

#[derive(Serialize)]
struct OllamaRequest {
    model: &'static str,
    messages: Vec<OllamaMessage>,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct OllamaResponse {
    choices: Vec<OllamaChoice>,
}

#[derive(Deserialize)]
struct OllamaChoice {
    message: OllamaMessageContent,
}

#[derive(Deserialize)]
struct OllamaMessageContent {
    content: Option<String>,
}

impl IsThisReal {
    pub async fn handler(ctx: &Context, msg: &Message) {
        // 1. Feature flag check
        let pool = get_pool(ctx).await;
        if !Features::is_enabled(&pool, "is_this_real") {
            return;
        }
        println!("[is_this_real] feature enabled, processing message");

        // 2. Bot mention check
        let bot_user = match ctx.http.get_current_user().await {
            Ok(user) => user,
            Err(e) => {
                eprintln!("[is_this_real] Failed to get current user: {}", e);
                return;
            }
        };
        println!("[is_this_real] msg.mentions={:?} bot_id={}", msg.mentions.iter().map(|m| m.id.get()).collect::<Vec<_>>(), bot_user.id.get());
        if !msg.mentions.iter().any(|m| m.id == bot_user.id) {
            return;
        }
        println!("[is_this_real] Bot mentioned, checking reply...");

        // 3. Reply check
        let referenced_id = match msg.message_reference.as_ref().and_then(|r| r.message_id) {
            Some(id) => id,
            None => return,
        };

        // 4. Guild ID check
        let guild_id = match msg.guild_id {
            Some(id) => id,
            None => return,
        };

        // 5. Fetch referenced message
        let referenced_msg = match ctx.http.get_message(msg.channel_id, referenced_id).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to fetch referenced message: {}", e);
                return;
            }
        };

        // 6. Extract question — strip bot mention
        let bot_mention = format!("<@{}>", bot_user.id.get());
        let bot_mention_with_exclamation = format!("<@!{}>", bot_user.id.get());
        let question = msg
            .content
            .replace(&bot_mention, "")
            .replace(&bot_mention_with_exclamation, "")
            .trim()
            .to_string();

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

        // 7. Special user check
        if msg.author.id.get() == SPECIAL_USER_ID {
            let guild_id_u64 = guild_id.get();
            let server = match get_server_by_guild_id(&pool, guild_id_u64 as i64) {
                Some(s) => s,
                None => {
                    eprintln!(
                        "No server config for guild {} (or DB unavailable)",
                        guild_id_u64
                    );
                    return;
                }
            };

            let gulag_channel =
                match Gulag::find_channel(&ctx.http, guild_id_u64, "the-gulag".to_string()).await {
                    Some(c) => c,
                    None => {
                        eprintln!("No gulag channel found");
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

            match Gulag::add_to_gulag(&ctx.http, &pool, params).await {
                Ok(_) => {
                    if let Err(why) = msg
                        .channel_id
                        .send_message(
                            &ctx.http,
                            CreateMessage::new().content(format!(
                                "{} wanted to know if something was real... now they're in the gulag for 5m. Irony.",
                                msg.author.mention()
                            )),
                        )
                        .await
                    {
                        eprintln!("Failed to send gulag message: {}", why);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to gulag special user: {}", e);
                    return;
                }
            }
            return;
        }

        // 8. Cooldown check (normal users)
        let user_id = msg.author.id.get();
        let guild_id_u64 = guild_id.get();

        // Only check existing records — if none exists, user hasn't used the feature yet
        if let Some(usage) = get_is_this_real_usage(&pool, user_id as i64, guild_id_u64 as i64) {
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
                        CreateMessage::new().content("Come back tomorrow, I need my sleep"),
                    )
                    .await
                {
                    eprintln!("Failed to send cooldown message: {}", why);
                }
                return;
            }
        }

        // 9. Web search
        let search_results = exa::search(&question).await;
        let search_context = match search_results {
            Ok(results) => {
                if results.is_empty() {
                    String::new()
                } else {
                    let entries: Vec<String> = results
                        .iter()
                        .map(|(title, snippet)| format!("\"{}\": \"{}\"", title, snippet))
                        .collect();
                    format!("Research findings:\n{}", entries.join("\n"))
                }
            }
            Err(e) => {
                eprintln!("Exa search failed: {}", e);
                String::new()
            }
        };

        // 10. Build LLM prompt
        let original_content = referenced_msg.content.replace("\"", "\\\"");
        let prompt = if search_context.is_empty() {
            format!(
                "Someone said: \"{}\"\nThe question is: \"{}\"",
                original_content, question
            )
        } else {
            format!(
                "Someone said: \"{}\"\nThe question is: \"{}\"\n\n{}",
                original_content, question, search_context
            )
        };

        // 11. Call Ollama
        let ollama_request = OllamaRequest {
            model: OLLAMA_MODEL,
            messages: vec![
                OllamaMessage {
                    role: "system",
                    content: SYSTEM_PROMPT.to_string(),
                },
                OllamaMessage {
                    role: "user",
                    content: prompt,
                },
            ],
        };

        let tama_token = std::env::var("TAMA_TOKEN").expect("TAMA_TOKEN must be set");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        let response = match client
            .post(OLLAMA_URL)
            .header("Authorization", format!("Bearer {}", tama_token))
            .json(&ollama_request)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Ollama call failed: {}", e);
                return;
            }
        };

        let ollama_response: OllamaResponse = match response.json().await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to parse Ollama response: {}", e);
                return;
            }
        };

        let llm_text = ollama_response
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("");

        if llm_text.is_empty() {
            return;
        }

        // 12. Post response
        if let Err(why) = msg
            .channel_id
            .send_message(&ctx.http, CreateMessage::new().content(llm_text.trim()))
            .await
        {
            eprintln!("Failed to post LLM response: {}", why);
        }

        // 13. Update cooldown (fire and forget) — record first use or update existing
        let usage_result = get_or_create_is_this_real_usage(&pool, user_id as i64, guild_id_u64 as i64);
        if let Ok(u) = usage_result {
            if let Err(e) = update_is_this_real_usage(&pool, u.id) {
                eprintln!("Failed to update cooldown: {}", e);
            }
        }
    }
}
