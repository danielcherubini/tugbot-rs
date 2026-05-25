# is_this_real Feature Plan

**Goal:** Add a feature where `@tugbot` in a reply triggers an LLM-powered fact-check with web search, a daily cooldown per user, and a special user that gets gulaged for trying.

**Architecture:** Message handler in `src/handlers/is_this_real.rs` checks for bot mention + reply. Special user (163055057254875136) gets gulaged for 300s. Everyone else gets a daily cooldown check, then Exa web search + Ollama LLM call. Response posted to channel.

**Tech Stack:** Rust, Serenity (Discord), Diesel (PostgreSQL), reqwest (HTTP), Exa API (web search), Ollama (LLM at `http://tama:11434`)

**Auth:** Ollama requires `Authorization: Bearer <TAMA_TOKEN>` header. Token stored in env as `TAMA_TOKEN`.

---

### Task 1: Database migration, feature flag, and reqwest json feature

**Context:**
We need a table to track daily cooldown usage per user per guild, a feature flag row, and we need to enable reqwest's `json` feature so `RequestBuilder::json()` and `Response::json()` compile. Without the `json` feature, both the Exa module and the handler's Ollama calls will fail to compile.

**Files:**
- Create: `migrations/2026-05-24-000000_create_is_this_real_usage/up.sql`
- Create: `migrations/2026-05-24-000000_create_is_this_real_usage/down.sql`
- Create: `migrations/2026-05-24-000001_add_is_this_real_feature/up.sql`
- Create: `migrations/2026-05-24-000001_add_is_this_real_feature/down.sql`
- Modify: `Cargo.toml` — add `features = ["json"]` to reqwest dependency

**What to implement:**

1. **Create `is_this_real_usage` table:**
```sql
CREATE TABLE "is_this_real_usage" (
  "id" SERIAL PRIMARY KEY,
  "user_id" bigint NOT NULL,
  "guild_id" bigint NOT NULL,
  "last_used_at" timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  "created_at" timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(user_id, guild_id)
);
```
Down: `DROP TABLE "is_this_real_usage";`

2. **Insert feature flag:**
```sql
INSERT INTO features (name, enabled) VALUES ('is_this_real', true)
ON CONFLICT (name) DO NOTHING;
```
Down: `DELETE FROM features WHERE name = 'is_this_real';`

3. **Update Cargo.toml:** Change `reqwest = "0.12.7"` to `reqwest = { version = "0.12.7", features = ["json"] }`

**Steps:**
- [ ] Create migration directory `migrations/2026-05-24-000000_create_is_this_real_usage/`
- [ ] Write `up.sql` and `down.sql` for the table migration
- [ ] Create migration directory `migrations/2026-05-24-000001_add_is_this_real_feature/`
- [ ] Write `up.sql` and `down.sql` for the feature flag migration
- [ ] Update `Cargo.toml`: change `reqwest = "0.12.7"` to `reqwest = { version = "0.12.7", features = ["json"] }`
- [ ] Run `diesel migration run` (with DATABASE_URL from .env) to verify migrations apply cleanly
- [ ] Run `diesel migration redo` twice to verify both migrations can be reverted and re-applied
- [ ] Run `cargo build` to verify nothing is broken
- [ ] Commit with message: "feat: add is_this_real migration, feature flag, and reqwest json feature"

**Acceptance criteria:**
- [ ] `diesel migration run` succeeds with no errors
- [ ] `diesel migration redo` succeeds for both migrations
- [ ] `is_this_real_usage` table exists with correct columns and unique constraint
- [ ] `features` table has a row `('is_this_real', true)`
- [ ] `Cargo.toml` has `reqwest` with `features = ["json"]`
- [ ] `cargo build` succeeds

---

### Task 2: DB models and functions

**Context:**
Add Diesel models for the `is_this_real_usage` table and DB functions for checking/updating cooldown. Follows the exact pattern of `AiSlopUsage` / `NewAiSlopUsage` and `get_or_create_goku_poll_usage`.

**Files:**
- Modify: `src/db/schema.rs` — add `is_this_real_usage` table definition
- Modify: `src/db/models.rs` — add `IsThisRealUsage` and `NewIsThisRealUsage` structs
- Modify: `src/db/mod.rs` — add DB functions and imports

**What to implement:**

In `src/db/schema.rs`, add:
```rust
diesel::table! {
    is_this_real_usage (id) {
        id -> Int4,
        user_id -> Int8,
        guild_id -> Int8,
        last_used_at -> Timestamp,
        created_at -> Timestamp,
    }
}
```
And add `is_this_real_usage` to `diesel::allow_tables_to_appear_in_same_query!()`.

> **Note:** You can also run `diesel print-schema > src/db/schema.rs` after the migration to auto-generate this. Either approach is fine.

In `src/db/models.rs`, add:
```rust
#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = is_this_real_usage)]
pub struct IsThisRealUsage {
    pub id: i32,
    pub user_id: i64,
    pub guild_id: i64,
    pub last_used_at: SystemTime,
    pub created_at: SystemTime,
}

#[derive(Insertable)]
#[diesel(table_name = is_this_real_usage)]
pub struct NewIsThisRealUsage {
    pub user_id: i64,
    pub guild_id: i64,
    pub last_used_at: SystemTime,
    pub created_at: SystemTime,
}
```

In `src/db/mod.rs`, add these imports at the top:
```rust
use self::models::{IsThisRealUsage, NewIsThisRealUsage};
```
And add to the existing `use self::schema::{...}` block: `is_this_real_usage::{self, dsl::*}`.

Then add two functions:

1. **`get_or_create_is_this_real_usage(pool, user_id, guild_id) -> Result<IsThisRealUsage>`**
   - Look up by `(user_id, guild_id)` using `is_this_real_usage::dsl::*`
   - If not found (`NotFound` error), insert a new record with `last_used_at = SystemTime::now()`, `created_at = SystemTime::now()`
   - Return the record (existing or newly created)
   - Follow `get_or_create_goku_poll_usage` exactly — copy its structure

2. **`update_is_this_real_usage(pool, usage_id) -> Result<IsThisRealUsage>`**
   - `diesel::update(is_this_real_usage::dsl::is_this_real_usage.find(usage_id))`
   - `.set(is_this_real_usage::dsl::last_used_at.eq(SystemTime::now()))`
   - `.get_result(&mut conn)`
   - Return the updated record

**Steps:**
- [ ] Run `diesel print-schema > src/db/schema.rs` (or manually add the table definition)
- [ ] Add `IsThisRealUsage` and `NewIsThisRealUsage` to `src/db/models.rs`
- [ ] Add `get_or_create_is_this_real_usage` and `update_is_this_real_usage` to `src/db/mod.rs`
- [ ] Add the new models to imports in `src/db/mod.rs`
- [ ] Run `cargo build`
  - Did it succeed? If not, fix compilation errors and re-run.
- [ ] Run `cargo test`
  - Did all tests pass? If not, fix and re-run.
- [ ] Commit with message: "feat: add is_this_real DB models and functions"

**Acceptance criteria:**
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes all existing tests
- [ ] Models follow the same pattern as `AiSlopUsage` / `GokuPollUsage`

---

### Task 3: Exa web search module

**Context:**
Create a module that wraps the Exa API for web searching. The bot will search using the user's question and return formatted results to include in the LLM prompt. Uses `reqwest` with the `json` feature (enabled in Task 1).

**Files:**
- Create: `src/exa.rs`
- Modify: `src/lib.rs` — add `pub mod exa;`

**What to implement:**

In `src/lib.rs`, add `pub mod exa;`.

In `src/exa.rs`:

**Imports:**
```rust
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
```

**Constants:**
```rust
const EXA_API_URL: &str = "https://api.exa.ai/search";
```

**Structs:**
```rust
#[derive(Serialize)]
struct ExaSearchRequest {
    query: String,
    #[serde(rename = "type")]
    search_type: String,
    num_results: u8,
    contents: ExaContents,
}

#[derive(Serialize)]
struct ExaContents {
    highlights: bool,
}

#[derive(Deserialize)]
struct ExaSearchResponse {
    results: Vec<ExaResult>,
}

#[derive(Deserialize)]
struct ExaResult {
    title: String,
    #[serde(default)]
    highlights: Vec<String>,
}
```

**Function:** `pub async fn search(query: &str) -> Result<Vec<(String, String)>>`

Implementation:
1. Read `EXA_API_KEY` from env: `std::env::var("EXA_API_KEY").context("EXA_API_KEY not set")?`
2. Build request:
```rust
let request = ExaSearchRequest {
    query: query.to_string(),
    search_type: "auto".to_string(),
    num_results: 3,
    contents: ExaContents { highlights: true },
};
```
3. Make POST request with 10-second timeout:
```rust
let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()
    .context("Failed to build HTTP client")?;

let response = client
    .post(EXA_API_URL)
    .header("x-api-key", &api_key)
    .json(&request)
    .send()
    .await
    .context("Failed to send Exa search request")?;

let search_response: ExaSearchResponse = response
    .json()
    .await
    .context("Failed to parse Exa search response")?;
```
4. Build results: For each result, join all `highlights` with `" "` as the snippet. If no highlights, use empty string.
5. Return `Vec<(String, String)>` of `(title, snippet)` tuples.

**Steps:**
- [ ] Add `pub mod exa;` to `src/lib.rs`
- [ ] Create `src/exa.rs` with imports, constants, structs, and `search()` function
- [ ] Run `cargo build`
  - Did it succeed? If not, fix compilation errors and re-run.
- [ ] Run `cargo test`
- [ ] Commit with message: "feat: add Exa web search module"

**Acceptance criteria:**
- [ ] `cargo build` succeeds
- [ ] `search()` function makes a POST request to Exa with correct headers and JSON body
- [ ] Returns `Vec<(String, String)>` of (title, highlights) tuples
- [ ] Uses a 10-second timeout on the HTTP client
- [ ] Gracefully returns `Err` on missing API key, HTTP errors, or parse failures

---

### Task 4: Handler implementation — is_this_real.rs

**Context:**
The core handler that processes messages, checks cooldowns, calls the LLM, and posts responses. Also handles the special user gulag trap.

**Files:**
- Create: `src/handlers/is_this_real.rs`
- Modify: `src/handlers/mod.rs` — add module import and handler dispatch

**What to implement:**

Create `src/handlers/is_this_real.rs` with the following structure:

**Imports:**
```rust
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

use crate::db::{get_or_create_is_this_real_usage, get_server_by_guild_id, update_is_this_real_usage};
use crate::exa;
use crate::features::Features;
use crate::handlers::get_pool;
use crate::handlers::gulag::{Gulag, GulagParams};
use serenity::{
    builder::CreateMessage,
    model::prelude::Message,
    prelude::Context,
};
```

**Struct and constants:**
```rust
pub struct IsThisReal;

const SPECIAL_USER_ID: u64 = 163055057254875136;
const COOLDOWN_HOURS: u64 = 24;
const GULAG_DURATION_SECS: u32 = 300; // 5 minutes — must be u32 to match GulagParams

const SYSTEM_PROMPT: &str = "You are Tugbot, a Discord bot that fact-checks claims. A user has asked you a question about something someone else said. Respond in one or two sentences max. Try to be funny, sarcastic, or sardonic when possible. Be helpful but keep it brief.";

const OLLAMA_URL: &str = "http://tama:11434/v1/chat/completions";
const OLLAMA_MODEL: &str = "whatevers-hot-n-fresh";

// Ollama requires a Bearer token from TAMA_TOKEN env var
```

**Ollama serde structs:**
```rust
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
```

**Handler method:** `pub async fn handler(ctx: &Context, msg: &Message)`

**Logic (in order):**

1. **Feature flag check:** Get pool via `get_pool(ctx)`. Check `Features::is_enabled(&pool, "is_this_real")` — if false, return early.

2. **Bot mention check:** Get bot's own user ID: `ctx.http.get_current_user().await` — if fails, `eprintln!` and return. Check `msg.mentions.iter().any(|m| m.id == bot_user.id)` — if false, return early.

3. **Reply check:** Get referenced message ID: `msg.references.as_ref().and_then(|r| r.message_id)` — if `None`, return early (not a reply).

4. **Guild ID check:** `msg.guild_id` — if `None`, return early (DM, not guild).

5. **Fetch referenced message:** `ctx.http.get_message(msg.channel_id, referenced_id.into()).await` — if fails, `eprintln!` and return.

6. **Extract question:** Strip the bot mention from `msg.content` (replace `<@BOT_ID>` or `<@!BOT_ID>` with empty string), then `.trim().to_string()`. If `question.is_empty()`, post `"Ask me a question about the message you replied to!"` to the channel and return.

7. **Special user check:** If `msg.author.id.get() == SPECIAL_USER_ID`:
   a. Get guild_id from `msg.guild_id.unwrap().get()`
   b. Get server config: `get_server_by_guild_id(&pool, guild_id as i64)` — if `None`, `eprintln!("No server config for guild {}", guild_id)` and return
   c. Find gulag channel: `Gulag::find_channel(&ctx.http, guild_id, "the-gulag".to_string()).await` — if `None`, `eprintln!("No gulag channel found")` and return
   d. Build `GulagParams`:
   ```rust
   let params = GulagParams {
       guildid: guild_id,
       userid: msg.author.id.get(),
       gulag_roleid: server.gulag_id as u64,
       gulaglength: GULAG_DURATION_SECS,
       channelid: gulag_channel.id.get(),
       messageid: msg.id.get(),
   };
   ```
   e. Call `Gulag::add_to_gulag(&ctx.http, &pool, params).await`:
      - On `Ok(_)`: Post to channel using `send_message`:
        ```rust
        msg.channel_id.send_message(
            &ctx.http,
            CreateMessage::new().content(format!(
                "{} wanted to know if something was real... now they're in the gulag for 5m. Irony.",
                msg.author.mention()
            )),
        ).await;
        ```
      - On `Err(e)`: `eprintln!("Failed to gulag special user: {}", e)` and return
   f. Return

8. **Cooldown check (normal users):**
   a. Get user_id and guild_id: `let user_id = msg.author.id.get(); let guild_id = msg.guild_id.unwrap().get();`
   b. Call `get_or_create_is_this_real_usage(&pool, user_id as i64, guild_id as i64)`:
      - On `Err(e)`: `eprintln!("Failed to check cooldown: {}", e)` and return
   c. On `Ok(usage)`: Check cooldown:
      ```rust
      // Note: unwrap_or_default() returns Duration::ZERO if last_used_at is in the future (clock skew)
      // This is intentional fail-open behavior — user can still use the feature
      let elapsed = SystemTime::now()
          .duration_since(usage.last_used_at)
          .unwrap_or_default()
          .as_secs();
      let cooldown_secs = COOLDOWN_HOURS * 3600;
      ```
      If `elapsed < cooldown_secs`: Post `"Come back tomorrow, I need my sleep"` to channel via `send_message` and return.

9. **Web search:** Call `exa::search(&question)`:
   - On `Ok(results)`: Build search context string:
     ```
     Research findings:
     "{title1}": "{snippet1}"
     "{title2}": "{snippet2}"
     "{title3}": "{snippet3}"
     ```
   - On `Err(e)`: `eprintln!("Exa search failed: {}", e)` and use empty string for search context
   - If search context is empty (no results or search failed), omit the "Research findings:" section entirely

10. **Build LLM prompt:**
    ```rust
    let prompt = if search_context.is_empty() {
        format!("Someone said: \"{original_content}\"\nThe question is: \"{question}\"")
    } else {
        format!("Someone said: \"{original_content}\"\nThe question is: \"{question}\"\n\n{search_context}")
    };
    ```
    Where `original_content` is the referenced message's content (escaped double quotes with `\"`).

11. **Call Ollama:**
    a. Build request:
    ```rust
    let ollama_request = OllamaRequest {
        model: OLLAMA_MODEL,
        messages: vec![
            OllamaMessage { role: "system", content: SYSTEM_PROMPT.to_string() },
            OllamaMessage { role: "user", content: prompt },
        ],
    };
    ```
    b. Make POST with 30-second timeout and Bearer auth:
    ```rust
    let tama_token = std::env::var("TAMA_TOKEN")
        .expect("TAMA_TOKEN must be set");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    let response: OllamaResponse = client
        .post(OLLAMA_URL)
        .header("Authorization", format!("Bearer {}", tama_token))
        .json(&ollama_request)
        .send()
        .await
        .context("Failed to send Ollama request")?;
    ```
    c. Parse response:
    ```rust
    let parsed: OllamaResponse = response
        .json()
        .await
        .context("Failed to parse Ollama response")?;
    ```
    c. On send error: `eprintln!("Ollama call failed: {}", e)` and return
    d. On parse error: `eprintln!("Failed to parse Ollama response: {}", e)` and return
    e. Extract response text:
    ```rust
    let llm_text = response
        .choices
        .first()
        .and_then(|c| c.message.content.as_deref())
        .unwrap_or("");
    ```
    f. If `llm_text.is_empty()`: return (no response to post)

12. **Post response:** Use `send_message` (not deprecated `.say()`):
    ```rust
    if let Err(why) = msg.channel_id
        .send_message(&ctx.http, CreateMessage::new().content(llm_text.trim()))
        .await
    {
        eprintln!("Failed to post LLM response: {}", why);
    }
    ```

13. **Update cooldown:** Fire and forget — log error only:
    ```rust
    if let Err(e) = update_is_this_real_usage(&pool, usage.id) {
        eprintln!("Failed to update cooldown: {}", e);
    }
    ```

**Wiring in mod.rs:**
- Add `is_this_real::IsThisReal` to imports
- Add `pub mod is_this_real;` to module declarations
- Add `IsThisReal::handler(&ctx, &msg).await;` to the `message()` event handler

**Steps:**
- [ ] Create `src/handlers/is_this_real.rs` with all imports, structs, constants, and handler logic
- [ ] Add `pub mod is_this_real;` to `src/handlers/mod.rs` module declarations
- [ ] Import `IsThisReal` in `src/handlers/mod.rs`
- [ ] Add `IsThisReal::handler(&ctx, &msg).await;` to the `message()` event handler
- [ ] Run `cargo build`
  - Did it succeed? If not, fix compilation errors and re-run.
- [ ] Run `cargo test`
  - Did all tests pass? If not, fix and re-run.
- [ ] Commit with message: "feat: implement is_this_real handler with LLM, cooldown, and special user gulag"

**Acceptance criteria:**
- [ ] `cargo build` succeeds
- [ ] Handler triggers only when bot is mentioned AND message is a reply
- [ ] Special user (163055057254875136) gets gulaged for 300s with irony message
- [ ] `Gulag::add_to_gulag` failure is handled (error logged, no false success message)
- [ ] Normal users get daily cooldown (24h) with "Come back tomorrow" message
- [ ] Empty question (just `@tugbot` with no text) posts a help message
- [ ] LLM response is posted to channel via `send_message`
- [ ] Cooldown is updated after successful response
- [ ] Exa search failure gracefully degrades (still calls LLM without search context)
- [ ] LLM failure logs error but doesn't post to Discord
- [ ] All HTTP calls use timeouts (10s for Exa, 30s for Ollama)

---

### Task 5: Verification, formatting, and clippy

**Context:**
Final verification that everything compiles cleanly, passes clippy, and is ready for deployment.

**Files:**
- No new files

**Steps:**
- [ ] Run `cargo fmt`
- [ ] Run `cargo clippy` — fix any warnings
- [ ] Run `cargo build --release`
- [ ] Run `cargo test`
- [ ] Verify `.env` has `EXA_API_KEY` and `TAMA_TOKEN` set
- [ ] Verify production server (`root@tugbot`) has both in `/opt/tugbot/.env`
- [ ] Commit with message: "chore: format, clippy fixes, and final verification for is_this_real"

**Acceptance criteria:**
- [ ] `cargo fmt` applied with no formatting changes needed
- [ ] `cargo clippy` has no warnings
- [ ] `cargo build --release` succeeds with no errors
- [ ] `cargo test` passes all tests
- [ ] Both local and production `.env` files have `EXA_API_KEY` and `TAMA_TOKEN`
