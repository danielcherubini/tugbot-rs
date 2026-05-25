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
const ADMIN_USER_ID: u64 = 212879017257205760;
const COOLDOWN_HOURS: u64 = 8; // 24h / 3 = 8h between uses (3 per day)
const GULAG_DURATION_SECS: u32 = 300; // 5 minutes

const SYSTEM_PROMPT: &str = "You are Tugbot, a Discord bot that fact-checks claims. A user has asked you a question about something someone else said. Respond in one or two sentences max. Try to be funny, sarcastic, or sardonic when possible. Be helpful but keep it brief.";

fn get_ollama_url() -> String {
    std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://tama:11434/v1/chat/completions".to_string())
}

fn get_ollama_model() -> String {
    std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "whatevers-hot-n-fresh".to_string())
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
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

        // 2. Bot mention check
        let bot_user = match ctx.http.get_current_user().await {
            Ok(user) => user,
            Err(e) => {
                eprintln!("[is_this_real] Failed to get current user: {}", e);
                return;
            }
        };
        if !msg.mentions.iter().any(|m| m.id == bot_user.id) {
            return;
        }

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

        // 6b. Require "is this real" / "is that real" keyword — case insensitive
        let lower = question.to_lowercase();
        if !lower.contains("is this real") && !lower.contains("is that real") {
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
                            CreateMessage::new()
                                .content(format!(
                                    "{} wanted to know if something was real... now they're in the gulag for 5m. Irony.",
                                    msg.author.mention()
                                ))
                                .reference_message((msg.channel_id, msg.id)),
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

        // 8. Cooldown check (normal users, admin gets unlimited)
        let user_id = msg.author.id.get();
        let guild_id_u64 = guild_id.get();

        // Only check existing records — if none exists, user hasn't used the feature yet
        // Admin user skips cooldown entirely
        if user_id != ADMIN_USER_ID {
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

        // 9. First LLM call — without search (save Exa costs)
        let original_content = referenced_msg.content.replace("\"", "\\\"");
        let first_prompt = format!(
            "Someone said: \"{}\"\nThe question is: \"{}\"",
            original_content, question
        );

        let first_response =
            call_ollama(SYSTEM_PROMPT.to_string(), first_prompt, "first pass").await;

        let Some(llm_text) = first_response else {
            return;
        };

        // Check if LLM is uncertain — if so, search and retry
        let uncertainty_markers = [
            "i don't know", "i'm not sure", "not sure", "uncertain", "might be",
            "possibly", "can't verify", "need more", "hard to say", "not enough",
            "don't have enough", "could be", "it's unclear", "i can't",
        ];
        let is_uncertain = uncertainty_markers.iter().any(|m| {
            llm_text.to_lowercase().contains(m)
        });

        let final_text = if is_uncertain {
            let search_query = format!("{} {}", referenced_msg.content, question);
            let search_results = exa::search(&search_query).await;
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

            let second_prompt = if search_context.is_empty() {
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

            match call_ollama(SYSTEM_PROMPT.to_string(), second_prompt, "second pass").await {
                Some(text) => text,
                None => return,
            }
        } else {
            llm_text
        };

        // 12. Post response (reply to the user's question)
        if let Err(why) = msg
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new()
                    .content(final_text.trim())
                    .reference_message((msg.channel_id, msg.id)),
            )
            .await
        {
            eprintln!("Failed to post LLM response: {}", why);
        }

        // 13. Update cooldown (fire and forget) — skip for admin
        if user_id != ADMIN_USER_ID {
            let usage_result = get_or_create_is_this_real_usage(&pool, user_id as i64, guild_id_u64 as i64);
            if let Ok(u) = usage_result {
                if let Err(e) = update_is_this_real_usage(&pool, u.id) {
                    eprintln!("Failed to update cooldown: {}", e);
                }
            }
        }
    }
}

async fn call_ollama(system: String, user: String, label: &str) -> Option<String> {
    let ollama_request = OllamaRequest {
        model: get_ollama_model(),
        messages: vec![
            OllamaMessage {
                role: "system",
                content: system,
            },
            OllamaMessage {
                role: "user",
                content: user,
            },
        ],
    };

    let tama_token = std::env::var("TAMA_TOKEN").expect("TAMA_TOKEN must be set");
    let ollama_url = get_ollama_url();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("Failed to build HTTP client");

    let response = match client
        .post(&ollama_url)
        .header("Authorization", format!("Bearer {}", tama_token))
        .json(&ollama_request)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[is_this_real] Ollama call failed ({}): {}", label, e);
            return None;
        }
    };

    let status = response.status();
    let raw_body = match response.text().await {
        Ok(body) => body,
        Err(e) => {
            eprintln!("[is_this_real] Failed to read Ollama response body ({}): {}", label, e);
            return None;
        }
    };

    let ollama_response: OllamaResponse = match serde_json::from_str(&raw_body) {
        Ok(r) => r,
        Err(_e) => {
            eprintln!(
                "[is_this_real] Failed to parse Ollama response ({}): status={}, body={}",
                label,
                status,
                raw_body.chars().take(500).collect::<String>()
            );
            return None;
        }
    };

    let text = ollama_response
        .choices
        .first()
        .and_then(|c| c.message.content.as_deref())
        .unwrap_or("")
        .to_string();

    if text.is_empty() {
        eprintln!("[is_this_real] Ollama returned empty response ({})", label);
        None
    } else {
        Some(text)
    }
}
