# Cull Command Plan

**Goal:** Admin-only `/cull` slash command to identify and kick inactive guild members (never posted or inactive X+ days), with persistent activity tracking and dry-run preview.

**Architecture:** A `user_activity` table tracks `last_message_at` per user per guild. Passive tracking via `MESSAGE_CREATE` upserts on every human message. An initial scan on startup seeds the table from recent message history. The `/cull` command queries inactive users, optionally previews (dry-run), or kicks them — all output posted to `cat-herding` (channel `1224402885786472659`).

**Tech Stack:** Rust, Serenity (Discord), Diesel ORM, PostgreSQL, tokio async runtime.

---

### Task 1: Database migration, schema, models, and DB functions

**Context:**
The foundation of the cull feature is the `user_activity` table and its associated Diesel models, schema entry, and DB helper functions. This task creates the migration, updates the Diesel schema, adds the model structs, and implements the DB functions used by both passive tracking and the cull command. The upsert must use `GREATEST` semantics to prevent the startup scan from regressing timestamps written by live passive tracking.

**Files:**
- Create: `migrations/2026-06-25-000000_create_user_activity/up.sql`
- Create: `migrations/2026-06-25-000000_create_user_activity/down.sql`
- Modify: `src/db/schema.rs`
- Modify: `src/db/models.rs`
- Modify: `src/db/mod.rs`

**What to implement:**

1. **Migration `up.sql`:**
```sql
CREATE TABLE user_activity (
    user_id         BIGINT      NOT NULL,
    guild_id        BIGINT      NOT NULL,
    last_message_at TIMESTAMP   NOT NULL,
    created_at      TIMESTAMP   NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, guild_id)
);

CREATE INDEX idx_user_activity_guild_last_message
    ON user_activity (guild_id, last_message_at);
```

2. **Migration `down.sql`:**
```sql
DROP INDEX IF EXISTS idx_user_activity_guild_last_message;
DROP TABLE IF EXISTS user_activity;
```

3. **Schema update (`src/db/schema.rs`):** Add a new `diesel::table!` block for `user_activity`:
```rust
diesel::table! {
    user_activity (user_id, guild_id) {
        user_id -> Int8,
        guild_id -> Int8,
        last_message_at -> Timestamp,
        created_at -> Timestamp,
    }
}
```
Add `user_activity` to the `diesel::allow_tables_to_appear_in_same_query!` macro at the bottom of the file.

4. **Models (`src/db/models.rs`):** Add two structs:
```rust
#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = user_activity)]
pub struct UserActivity {
    pub user_id: i64,
    pub guild_id: i64,
    pub last_message_at: SystemTime,
    pub created_at: SystemTime,
}

#[derive(Insertable)]
#[diesel(table_name = user_activity)]
pub struct NewUserActivity {
    pub user_id: i64,
    pub guild_id: i64,
    pub last_message_at: SystemTime,
    pub created_at: SystemTime,
}
```

5. **GREATEST helper (`src/db/mod.rs`):** Diesel 2.3.6 does NOT have a built-in `diesel::dsl::greatest()`. Define a custom `sql_function!` macro at the TOP of `src/db/mod.rs` (before any function that uses it):

```rust
diesel::sql_function! {
    /// PostgreSQL GREATEST function — returns the larger of two timestamps.
    /// Used by upsert_user_activity and bulk_upsert_activity to prevent timestamp regression.
    fn greatest(a: diesel::sql_types::Timestamp, b: diesel::sql_types::Timestamp) -> diesel::sql_types::Timestamp;
}
```

6. **DB functions (`src/db/mod.rs`):** Add four functions:

   a. `upsert_user_activity(pool, user_id: i64, guild_id: i64) -> Result<(), diesel::result::Error>` — Upserts a single row. On INSERT, sets both `last_message_at` AND `created_at` to `SystemTime::now()`. On CONFLICT (user exists), updates ONLY `last_message_at = GREATEST(existing, new)` — `created_at` is NEVER modified on update. Implementation using Diesel's on_conflict API:

```rust
pub fn upsert_user_activity(pool: &DbPool, user_id: i64, guild_id: i64) -> Result<(), diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use crate::db::schema::user_activity::dsl::*;
    use crate::db::models::NewUserActivity;
    use diesel::prelude::*;

    let time_now = SystemTime::now();
    let new_record = NewUserActivity {
        user_id,
        guild_id,
        last_message_at: time_now,
        created_at: time_now,
    };

    diesel::insert_into(user_activity)
        .values(&new_record)
        .on_conflict((user_id, guild_id))
        .do_update()
        .set(last_message_at.eq(greatest(last_message_at, diesel::dsl::excluded(last_message_at))))
        .execute(&mut conn)?;

    Ok(())
}
```

   b. `bulk_upsert_activity(pool, records: Vec<(i64, i64)>) -> Result<usize, diesel::result::Error>` — Bulk upserts multiple `(user_id, guild_id)` pairs. Same `GREATEST` semantics as the single upsert. `created_at` is set on INSERT only, never modified on UPDATE. Implementation:

```rust
pub fn bulk_upsert_activity(pool: &DbPool, records: Vec<(i64, i64)>) -> Result<usize, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use crate::db::schema::user_activity::dsl::*;
    use crate::db::models::NewUserActivity;
    use diesel::prelude::*;

    let time_now = SystemTime::now();
    let new_records: Vec<NewUserActivity> = records
        .into_iter()
        .map(|(uid, gid)| NewUserActivity {
            user_id: uid,
            guild_id: gid,
            last_message_at: time_now,
            created_at: time_now,
        })
        .collect();

    let rows = diesel::insert_into(user_activity)
        .values(&new_records)
        .on_conflict((user_id, guild_id))
        .do_update()
        .set(last_message_at.eq(greatest(last_message_at, diesel::dsl::excluded(last_message_at))))
        .execute(&mut conn)?;

    Ok(rows)
}
```

   c. `query_inactive_users(pool, guild_id: i64, days: i32) -> Result<Vec<i64>, diesel::result::Error>` — Returns a `Vec<i64>` of inactive user IDs (users in `user_activity` whose `last_message_at` is older than `days` ago). Computes the cutoff in Rust and filters by `guild_id` + `last_message_at < cutoff`:

```rust
pub fn query_inactive_users(pool: &DbPool, guild_id: i64, days: i32) -> Result<Vec<i64>, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use crate::db::schema::user_activity::dsl::*;
    use diesel::prelude::*;

    let cutoff = SystemTime::now() - Duration::from_secs((days as u64) * 86400);
    let inactive_ids: Vec<i64> = user_activity
        .filter(guild_id.eq(guild_id))
        .filter(last_message_at.lt(cutoff))
        .select(user_id)
        .load(&mut conn)?;

    Ok(inactive_ids)
}
```

   d. `query_all_tracked_user_ids_for_guild(pool, guild_id: i64) -> Result<Vec<i64>, diesel::result::Error>` — Returns ALL user IDs that have ANY row in `user_activity` for the given guild. Used by the cull handler to determine "never posted" users (members NOT in this list):

   e. `query_user_activity_for_ids(pool, guild_id: i64, user_ids: Vec<i64>) -> Result<Vec<UserActivity>, diesel::result::Error>` — Returns `UserActivity` rows for specific user IDs in a single query. Used by the cull handler's dry-run mode to fetch last_message_at for all candidates in one roundtrip (avoids N+1 queries). Uses PostgreSQL `ANY` array:

```rust
pub fn query_user_activity_for_ids(pool: &DbPool, guild_id: i64, user_ids: Vec<i64>) -> Result<Vec<UserActivity>, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use crate::db::schema::user_activity::dsl::*;
    use crate::db::models::UserActivity;
    use diesel::prelude::*;

    let results: Vec<UserActivity> = user_activity
        .filter(guild_id.eq(guild_id))
        .filter(user_id.eq_any(user_ids))
        .load(&mut conn)?;

    Ok(results)
}
```

```rust
pub fn query_all_tracked_user_ids_for_guild(pool: &DbPool, guild_id: i64) -> Result<Vec<i64>, diesel::result::Error> {
    let mut conn = pool.get().map_err(pool_error_to_diesel)?;
    use crate::db::schema::user_activity::dsl::*;
    use diesel::prelude::*;

    let tracked_ids: Vec<i64> = user_activity
        .filter(guild_id.eq(guild_id))
        .select(user_id)
        .load(&mut conn)?;

    Ok(tracked_ids)
}
```

6. **Add `user_activity` to the imports in `src/db/mod.rs`:** Add `user_activity` to the `use self::schema::...` block and `UserActivity, NewUserActivity` to the `use self::models::...` block.

**Steps:**
- [ ] Create migration directory and `up.sql` / `down.sql` files
- [ ] Add `user_activity` table block to `src/db/schema.rs`
- [ ] Add `user_activity` to `allow_tables_to_appear_in_same_query!` in `src/db/schema.rs`
- [ ] Add `UserActivity` and `NewUserActivity` structs to `src/db/models.rs`
- [ ] Add `upsert_user_activity`, `bulk_upsert_activity`, `query_inactive_users` to `src/db/mod.rs`
- [ ] Run `cargo build`
  - Did it succeed? If not, fix compilation errors and re-run before continuing.
- [ ] Run `cargo test`
  - Did all tests pass? If not, fix failures and re-run before continuing.
- [ ] Run `cargo clippy --all-targets`
  - Did it succeed with zero warnings? If not, fix and re-run before continuing.
- [ ] Run `cargo fmt`
- [ ] Commit with message: "feat: add user_activity table, models, and DB functions for cull tracking"

**Acceptance criteria:**
- [ ] Migration files exist with correct SQL (table + index in up.sql, drop in down.sql)
- [ ] `user_activity` table block in schema.rs with composite PK `(user_id, guild_id)`
- [ ] `UserActivity` (Queryable) and `NewUserActivity` (Insertable) structs in models.rs
- [ ] `upsert_user_activity` uses `GREATEST` in ON CONFLICT DO UPDATE
- [ ] `bulk_upsert_activity` accepts `Vec<(i64, i64)>` and bulk upserts with `GREATEST`
- [ ] `query_inactive_users` computes cutoff in Rust and filters by `guild_id` + `last_message_at < cutoff`
- [ ] `cargo build` succeeds, `cargo test` passes, `cargo clippy --all-targets` is warning-free

---

### Task 2: Passive tracking hook + startup initial scan

**Context:**
Activity tracking happens in two modes: (1) passive — every `MESSAGE_CREATE` from a human triggers an upsert, and (2) initial scan — on bot startup, recent message history is scanned to seed the table. The passive hook is added to the existing `message()` event handler in `mod.rs`. The initial scan runs as a background `tokio::spawn` task (not blocking `ready()`), spawned from `ready()`.

**Files:**
- Modify: `src/handlers/mod.rs`

**What to implement:**

1. **Passive tracking hook in `message()` handler:** Add at the TOP of the existing `async fn message(&self, ctx: Context, msg: Message)` function (before any existing handler calls):

```rust
async fn message(&self, ctx: Context, msg: Message) {
    // Passive activity tracking for cull command
    // Skip: bots, webhooks, DMs (no guild)
    if !msg.author.bot && msg.webhook_id.is_none() {
        if let Some(guild_id) = msg.guild_id {
            let pool = get_pool(&ctx).await;
            let user_id = match i64::try_from(msg.author.id.get()) {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("[cull] User ID conversion error: {}", e);
                    return; // skip tracking for this message
                }
            };
            let guild_id_i64 = match i64::try_from(guild_id.get()) {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("[cull] Guild ID conversion error: {}", e);
                    return;
                }
            };
            if let Err(e) = crate::db::upsert_user_activity(&pool, user_id, guild_id_i64) {
                eprintln!("[cull] Failed to upsert user activity: {}", e);
            }
        }
    }

    // ... existing handler calls below ...
    Teh::handler(&ctx, &msg).await;
    // etc.
}
```

Key details:
- Check `!msg.author.bot` (skip bot messages including the bot itself)
- Check `msg.webhook_id.is_none()` (skip webhook messages)
- Check `msg.guild_id.is_some()` (skip DMs — `guild_id` is None for DMs)
- Use `i64::try_from()` for ID conversions (codebase convention)
- Use `eprintln!("[cull] ...")` for logging (codebase convention)
- Errors are logged but NOT propagated (passive tracking must never crash the message handler)
- This is fire-and-forget — no await on a heavy operation (the upsert is a single DB call, fast)

2. **Startup initial scan in `ready()` handler:** Add after `Gulag::run_gulag_vote_check(...)` in the existing `ready()` function. Spawn a background task that:

```rust
// Spawn initial activity scan (seeds user_activity table from recent messages)
let http_clone = ctx.http.clone();
let pool_clone = pool.clone();
let servers_clone = servers.clone(); // from existing `let servers = Servers::get_servers(...)`

tokio::spawn(async move {
    for server in &servers_clone {
        let guild_id = server.guild_id.get();
        
        // Fetch all channels for this guild
        let channels = match http_clone.get_channels(guild_id.into()).await {
            Ok(chs) => chs,
            Err(e) => {
                eprintln!("[cull] Failed to get channels for guild {}: {}", guild_id, e);
                continue;
            }
        };

        // Filter to text channels only (skip voice, category, DM, etc.)
        // Use serenity::all::ChannelType to match codebase conventions
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
            // Fetch last 200 messages from each channel
            let builder = serenity::all::GetMessages::new(channel.id).most(200);
            let messages = match http_clone.get_messages(channel.id, builder).await {
                Ok(msgs) => msgs,
                Err(e) => {
                    eprintln!("[cull] Failed to get messages from channel {}: {}", channel.name, e);
                    continue;
                }
            };

            for msg in &messages {
                if !msg.author.bot && msg.webhook_id.is_none() {
                    let user_id = match i64::try_from(msg.author.id.get()) {
                        Ok(id) => id,
                        Err(_) => continue,
                    };
                    all_user_pairs.push((user_id, guild_id_i64));
                }
            }

            eprintln!("[cull] Scanned {}/{} channels, found {} unique users so far", 
                i + 1, text_channels.len(), all_user_pairs.len());
        }

        // Deduplicate
        let mut seen = std::collections::HashSet::new();
        all_user_pairs.retain(|pair| seen.insert(*pair));

        if !all_user_pairs.is_empty() {
            match crate::db::bulk_upsert_activity(&pool_clone, all_user_pairs) {
                Ok(rows) => {
                    eprintln!("[cull] Upserted {} user activity records for guild {}", rows, guild_id);
                }
                Err(e) => {
                    eprintln!("[cull] Failed to bulk upsert activity: {}", e);
                }
            }
        }
    }

    eprintln!("[cull] Initial activity scan complete");
});
```

Key details:
- Uses `tokio::spawn` — does NOT block `ready()`
- Iterates over existing `servers` (already fetched in `ready()`)
- Filters to `ChannelType::Text` only (skip voice, category, announcement, etc.)
- Fetches 200 most recent messages per channel via `GetMessages::new(channel.id).most(200)`
- Collects `(user_id, guild_id)` pairs, deduplicates with `HashSet`
- Bulk upserts with `GREATEST` semantics (same as Task 1's single upsert)
- Uses `SystemTime::now()` as `last_message_at` for scanned users (since we know they were active recently, the exact timestamp from old messages is less useful than "active within scan window")
- Logs progress per-channel and final count with `eprintln!("[cull] ...")`
- All errors are logged but NOT propagated (startup must succeed even if scan fails)

**Steps:**
- [ ] Add passive tracking hook to `message()` in `src/handlers/mod.rs` (at the top, before existing handlers)
- [ ] Add startup initial scan as `tokio::spawn` in `ready()` in `src/handlers/mod.rs` (after `Gulag::run_gulag_vote_check`)
- [ ] Run `cargo build`
  - Did it succeed? If not, fix compilation errors and re-run before continuing.
- [ ] Run `cargo clippy --all-targets`
  - Did it succeed with zero warnings? If not, fix and re-run before continuing.
- [ ] Run `cargo fmt`
- [ ] Commit with message: "feat: add passive activity tracking and startup initial scan for cull"

**Acceptance criteria:**
- [ ] `message()` handler has passive tracking at the top (skips bots, webhooks, DMs)
- [ ] `ready()` spawns a background scan task (does not block ready)
- [ ] Scan iterates text channels, fetches 200 messages each, bulk upserts with GREATEST
- [ ] All errors logged via `eprintln!("[cull] ...")` but never propagated
- [ ] `cargo build` succeeds, `cargo clippy --all-targets` is warning-free

---

### Task 3: Cull handler (slash command + interaction logic)

**Context:**
The core `/cull` slash command implementation. It checks permissions, queries inactive users, and either previews (dry-run) or kicks them. All output is posted to `cat-herding` (channel `1224402885786472659`). The command requires the `cull` feature flag to be enabled.

**Files:**
- Create: `src/handlers/cull.rs`

**What to implement:**

Create `src/handlers/cull.rs` with the following structure:

```rust
use crate::db::{query_all_tracked_user_ids_for_guild, query_inactive_users, query_user_activity_for_ids, DbPool};
use crate::features::Features;
use crate::handlers::{get_pool, gulag::Gulag, HandlerResponse};
use serenity::{
    all::{
        CommandDataOptionValue, CommandInteraction, CommandOptionType, 
        CreateMessage, GetMembers, Permissions,
    },
    builder::{CreateCommand, CreateCommandOption},
    client::Context,
    model::id::ChannelId,
};
use std::collections::HashSet;

pub struct CullHandler;

// Channel ID for cat-herding (moderator-only output channel)
const CAT_HERDING_CHANNEL_ID: u64 = 1224402885786472659;
// Hard cap on kicks per invocation
const MAX_KICKS: usize = 50;
// Sleep between kicks to respect rate limits (1.5s)
const KICK_DELAY_MS: u64 = 1500;
// Whitelist roles — users with these roles are never culled
const WHITELIST_ROLES: &[&str] = &["Highly Regarded", "admin"];
```

1. **`setup_command() -> CreateCommand`:**
```rust
pub fn setup_command() -> CreateCommand {
    CreateCommand::new("cull")
        .description("Cull inactive members from the server")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "days",
                "Inactivity threshold in days (default: 30)",
            )
            .required(false),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "dry-run",
                "Preview candidates without kicking (default: false)",
            )
            .required(false),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "include-never-posted",
                "Include users who have never posted (default: false)",
            )
            .required(false),
        )
}
```

2. **`setup_interaction(ctx, command) -> HandlerResponse`:** Full implementation:

   a. **Feature flag check:** Use `Features::check_enabled(&pool, "cull")`. If disabled, return ephemeral "Cull feature is currently disabled".

   b. **Guild check:** `command.guild_id` must be `Some`. If `None`, return ephemeral "This command can only be used in a guild".

   c. **Permission check:** Same pattern as gulag handlers. Get member via `ctx.http.get_member(guild_id, command.user.id)`, then `Gulag::member_has_any_role(&ctx.http, guild_id, &member, &["Highly Regarded", "admin"])`. If not authorized, return ephemeral "Error: You need Highly Regarded or admin role to use this command".

   d. **Bot KICK_MEMBERS check:** Verify the bot has `KICK_MEMBERS` permission. `Guild::member_permissions` takes `&Member` (not `UserId`), so fetch the bot's Member first:
   ```rust
   let current_user = match ctx.http.get_current_user().await {
       Ok(u) => u,
       Err(e) => return HandlerResponse { content: format!("Failed to get bot info: {}", e), components: None, ephemeral: true },
   };
   let bot_member = match ctx.http.get_member(guild_id.into(), current_user.id).await {
       Ok(m) => m,
       Err(e) => return HandlerResponse { content: format!("Failed to get bot member: {}", e), components: None, ephemeral: true },
   };
   let guild = match ctx.http.get_guild(guild_id.into()).await {
       Ok(g) => g,
       Err(e) => return HandlerResponse { content: format!("Failed to get guild info: {}", e), components: None, ephemeral: true },
   };
   let bot_permissions = guild.member_permissions(&bot_member);
   if !bot_permissions.contains(Permissions::KICK_MEMBERS) {
       return HandlerResponse {
           content: "I don't have KICK_MEMBERS permission on this server.".to_string(),
           components: None,
           ephemeral: true,
       };
   }
   ```

   e. **Parse options:** Extract `days` (default 30), `dry_run` (default false), `include_never_posted` (default false). Validate `days > 0` and `days <= 365`.

   f. **Fetch member list:** Use REST pagination with `GetMembers`. Keep `Vec<Member>` objects throughout (NOT just a `HashSet<u64>`), because `member.user.bot` is available directly on the fetched Member — no additional API calls needed.
   ```rust
   let mut all_members: Vec<serenity::all::Member> = Vec::new();
   let mut after_id: Option<serenity::all::UserId> = None;
   loop {
       let mut builder = GetMembers::new(guild_id.into()).limit(1000);
       if let Some(user_id) = after_id {
           builder = builder.after(user_id);
       }
       let members: Vec<serenity::all::Member> = match ctx.http.get_members(builder).await {
           Ok(ms) => ms,
           Err(e) => {
               post_to_cat_herding(&ctx.http, &format!("Error fetching members: {}", e)).await;
               break;
           }
       };
       // Break when we get fewer than limit (last page) or empty
       all_members.extend(members);
       if members.len() < 1000 {
           break;
       }
       after_id = all_members.last().map(|m| m.user.id);
   }
   ```

   g. **Filter member list:** Remove bots and whitelisted users. Use `member.user.bot` directly (no additional API call):
   ```rust
   // Fetch whitelist role IDs once
   let whitelist_role_ids = get_whitelist_role_ids(&ctx.http, guild_id).await;
   
   // Filter: remove bots, remove whitelisted roles
   let filtered_members: Vec<_> = all_members.into_iter().filter(|member| {
       // Skip bots (member.user.bot is available directly from GetMembers)
       if member.user.bot {
           return false;
       }
       // Skip whitelisted roles
       if member_has_any_role_ids(member, &whitelist_role_ids) {
           return false;
       }
       true
   }).collect();
   ```
   
   Helper function `get_whitelist_role_ids(http, guild_id) -> HashSet<u64>`: Fetch all guild roles via `http.get_guild_roles(guild_id.into())`, find those matching `WHITELIST_ROLES` ("Highly Regarded", "admin"), return their IDs as `HashSet<u64>`.
   
   Helper function `member_has_any_role_ids(member: &serenity::all::Member, role_ids: &HashSet<u64>) -> bool`: Check if any `member.roles` (which is `Vec<RoleId>`) intersects with `role_ids`. Implementation: `member.roles.iter().any(|r| role_ids.contains(&r.get()))`.

   h. **Filter out gulaged users:** For each remaining member, check `Gulag::is_user_in_gulag(&pool, uid)`. If `Some`, skip (they're already punished). Note: `is_user_in_gulag` only filters by `user_id` (not `guild_id`), so a user gulaged in ANY guild is skipped. This is acceptable behavior — a user already being punished elsewhere shouldn't be culled.

   i. **Query inactive users:** Call `query_inactive_users(&pool, guild_id_i64, days)` to get inactive user IDs from the DB. Convert to `HashSet<u64>`.

   j. **Build candidate list:**
   - Inactive candidates: members whose `user.id.get()` is in `inactive_user_ids`
   - If `include_never_posted`: call `query_all_tracked_user_ids_for_guild(&pool, guild_id_i64)` to get all tracked user IDs, convert to `HashSet<u64>`. Add members whose `user.id.get()` is NOT in `tracked_user_ids` (never posted).
   - Cap at `MAX_KICKS` (50). If more than 50, take the first 50 (sorted by user ID for determinism).
   - Store candidates as `Vec<u64>` (user IDs).

   k. **Post to cat-herding:** Helper function `post_to_cat_herding(http, content: &str) -> bool`. Use `ChannelId::from(ID).send_message()` directly — no need to call `get_channel()` first:
   ```rust
   async fn post_to_cat_herding(http: &serenity::http::Http, content: &str) -> bool {
       let channel_id = serenity::all::ChannelId::from(CAT_HERDING_CHANNEL_ID);
       match channel_id.send_message(http, CreateMessage::new().content(content)).await {
           Ok(_) => true,
           Err(e) => {
               eprintln!("[cull] Failed to post to cat-herding: {}", e);
               false
           }
       }
   }
   ```
   If posting fails, fall back to including the content in the ephemeral response.

   l. **Dry-run mode (`dry_run = true`):**
   - For "last active" dates, query ALL candidate timestamps in a SINGLE roundtrip. Add a DB function in Task 1: `query_user_activity_for_ids(pool, guild_id: i64, user_ids: Vec<i64>) -> Result<Vec<UserActivity>, diesel::result::Error>` — uses `WHERE guild_id = $1 AND user_id = ANY($2)` (PostgreSQL array). Build a `HashMap<i64, SystemTime>` from the results for O(1) lookup.
   - Build a message listing candidates (max 25 lines to stay within Discord's 2000-char message limit):
   ```
   **Cull Dry-Run** (inactive {}+ days, never posted: {})
   
   {}
   
   Total candidates: {} (capped at {})
   Run `/cull --days {}` to execute.
   ```
   - Each candidate line: `<@USER_ID> (last active: YYYY-MM-DD)` or `<@USER_ID> (never posted)`
   - For "last active" date, look up in the `HashMap`. For "never posted", just say "never posted".
   - If >25 candidates, show first 25 + "and N more..."
   - Post to cat-herding, ephemeral "Dry-run posted to <#1224402885786472659>".

   m. **Execute mode (`dry_run = false`):**
   - Post "Starting cull: {} candidates (inactive {}+ days)..." to cat-herding
   - For each candidate:
     ```rust
     match ctx.http.kick(guild_id.into(), uid.into(), Some(&format!("Inactive {} days — /cull by {}", days, command.user.name))).await {
         Ok(_) => {
             success_count += 1;
             // Log to cat-herding (batch every 5 to avoid too many messages)
         }
         Err(e) => {
             skip_count += 1;
             eprintln!("[cull] Failed to kick {}: {}", uid, e);
         }
     }
     tokio::time::sleep(std::time::Duration::from_millis(KICK_DELAY_MS)).await;
     ```
   - After all kicks, post summary: `Cull complete: {} kicked, {} skipped (errors).` to cat-herding.
   - Ephemeral: `Cull complete. Results posted to <#1224402885786472659>`.

   n. **Error handling for all paths:** If any step fails (DB error, API error), post the error to cat-herding AND return an ephemeral error to the invoker.

**Steps:**
- [ ] Create `src/handlers/cull.rs` with `CullHandler` struct
- [ ] Implement `setup_command()` with days, dry-run, include-never-posted options
- [ ] Implement `setup_interaction()` with all steps (feature flag, permissions, bot check, member fetch, filter, query, dry-run/execute)
- [ ] Implement helper functions: `post_to_cat_herding`, `get_whitelist_role_ids`, `member_has_any_role_ids`
- [ ] Add `pub mod cull;` to `src/handlers/mod.rs`
- [ ] Run `cargo build`
  - Did it succeed? If not, fix compilation errors and re-run before continuing.
- [ ] Run `cargo clippy --all-targets`
  - Did it succeed with zero warnings? If not, fix and re-run before continuing.
- [ ] Run `cargo fmt`
- [ ] Commit with message: "feat: implement /cull slash command with dry-run and kick logic"

**Acceptance criteria:**
- [ ] `setup_command()` registers `/cull` with `days` (int), `dry-run` (bool), `include-never-posted` (bool) options
- [ ] Feature flag check uses `Features::check_enabled(&pool, "cull")`
- [ ] Permission check requires "Highly Regarded" or "admin" role
- [ ] Bot `KICK_MEMBERS` permission verified before proceeding
- [ ] Member list fetched via REST pagination (`GetMembers` with `limit(1000)` + `after`)
- [ ] Bots, whitelisted roles, and gulaged users filtered from candidates
- [ ] Inactive users queried from `user_activity` table
- [ ] "Never posted" users derived from member list minus tracked users
- [ ] Hard cap of 50 candidates enforced
- [ ] Dry-run posts candidate list to cat-herding (channel 1224402885786472659)
- [ ] Execute mode kicks with 1.5s delay between kicks, posts results to cat-herding
- [ ] All errors posted to cat-herding with fallback to ephemeral
- [ ] `cargo build` succeeds, `cargo clippy --all-targets` is warning-free

---

### Task 4: Wire cull handler into event dispatch + add feature flag migration

**Context:**
The cull handler needs to be registered as a slash command and wired into the interaction dispatch. The `cull` feature flag needs to be added to the database (disabled by default).

**Files:**
- Modify: `src/handlers/mod.rs`
- Create: `migrations/2026-06-25-000001_add_cull_feature/up.sql`
- Create: `migrations/2026-06-25-000001_add_cull_feature/down.sql`

**What to implement:**

1. **`src/handlers/mod.rs` changes:**

   a. Add module declaration (with existing modules):
   ```rust
   pub mod cull;
   ```

   b. Add import in the `use crate::handlers::{...}` block:
   ```rust
   use crate::handlers::{
       // ... existing imports ...
       cull::CullHandler,
   };
   ```

   c. Add to `interaction_create` match block (before the `_ =>` fallback):
   ```rust
   "cull" => CullHandler::setup_interaction(&ctx, &command).await,
   ```

   d. Add to `set_commands` vec in `ready()` (with existing commands):
   ```rust
   CullHandler::setup_command(),
   ```

2. **Feature flag migration:**

   `migrations/2026-06-25-000001_add_cull_feature/up.sql`:
   ```sql
   -- Add the cull feature flag (disabled by default).
   -- When enabled, admins can use /cull to kick inactive members.
   INSERT INTO features (name, enabled) VALUES ('cull', false)
   ON CONFLICT (name) DO NOTHING;
   ```

   `migrations/2026-06-25-000001_add_cull_feature/down.sql`:
   ```sql
   DELETE FROM features WHERE name = 'cull';
   ```

**Steps:**
- [ ] Add `pub mod cull;` to `src/handlers/mod.rs`
- [ ] Add `cull::CullHandler` to the imports in `src/handlers/mod.rs`
- [ ] Add `"cull" => CullHandler::setup_interaction(&ctx, &command).await,` to the `interaction_create` match
- [ ] Add `CullHandler::setup_command(),` to the `set_commands` vec in `ready()`
- [ ] Create feature flag migration files (up.sql + down.sql)
- [ ] Run `cargo build`
  - Did it succeed? If not, fix compilation errors and re-run before continuing.
- [ ] Run `cargo test`
  - Did all tests pass? If not, fix failures and re-run before continuing.
- [ ] Run `cargo clippy --all-targets`
  - Did it succeed with zero warnings? If not, fix and re-run before continuing.
- [ ] Run `cargo fmt`
- [ ] Commit with message: "feat: wire /cull command into dispatch and add cull feature flag"

**Acceptance criteria:**
- [ ] `pub mod cull;` declared in `mod.rs`
- [ ] `CullHandler` imported in `mod.rs`
- [ ] `"cull"` case added to `interaction_create` match (before `_ =>`)
- [ ] `CullHandler::setup_command()` added to `set_commands` vec
- [ ] Feature flag migration inserts `('cull', false)` with `ON CONFLICT DO NOTHING`
- [ ] `cargo build` succeeds, `cargo test` passes, `cargo clippy --all-targets` is warning-free

---

### Task 5: Run migrations, verify end-to-end

**Context:**
Final verification that all pieces work together. Run migrations against the database, verify the bot compiles and starts, and confirm the feature flag and command are registered.

**Files:**
- No new files. Verification only.

**What to implement:**

1. **Run migrations:**
   ```bash
   diesel migration run
   ```
   Verify both migrations (`2026-06-25-000000_create_user_activity` and `2026-06-25-000001_add_cull_feature`) apply successfully.

2. **Verify database schema:**
   ```sql
   -- Check user_activity table exists with correct columns
   \d user_activity
   -- Check index exists
   \di idx_user_activity_guild_last_message
   -- Check feature flag exists
   SELECT * FROM features WHERE name = 'cull';
   ```
   Expected: `user_activity` table with `(user_id, guild_id)` PK, `last_message_at TIMESTAMP`, `created_at TIMESTAMP`. Index on `(guild_id, last_message_at)`. Feature row `('cull', false)`.

3. **Build and verify:**
   ```bash
   cargo build --release
   cargo clippy --all-targets  # must be warning-free
   cargo test
   ```

4. **Enable feature flag (for testing):**
   ```sql
   UPDATE features SET enabled = true WHERE name = 'cull';
   ```

5. **Start bot and verify:**
   - Bot logs should show `[cull] Scanned X/Y channels...` and `[cull] Initial activity scan complete`
   - Bot should register `/cull` command (check logs for "I now have the following guild slash commands" including "cull")
   - In Discord: `/cull --dry-run --days 30` should return "Cull feature is currently disabled" if flag is off, or a dry-run preview if flag is on

**Steps:**
- [ ] Run `diesel migration run` — both migrations apply successfully
- [ ] Verify `user_activity` table schema and index exist
- [ ] Verify `cull` feature flag exists in `features` table (enabled = false)
- [ ] Run `cargo build --release` — succeeds
- [ ] Run `cargo clippy --all-targets` — warning-free
- [ ] Run `cargo test` — all pass
- [ ] Commit with message: "ci: verify cull command migrations and build"

**Acceptance criteria:**
- [ ] Both migrations applied successfully
- [ ] `user_activity` table exists with correct schema and index
- [ ] `cull` feature flag exists (enabled = false by default)
- [ ] `cargo build --release` succeeds
- [ ] `cargo clippy --all-targets` is warning-free
- [ ] `cargo test` passes

---

## Reviewer Notes (from brainstorming + plan review)

The following issues from both reviews were addressed in this plan:

| Issue | Resolution |
|-------|-----------|
| Upsert GREATEST semantics | `diesel::sql_function!` macro defines `greatest()`, used in all upserts |
| `created_at` preservation | `ON CONFLICT DO UPDATE SET` updates ONLY `last_message_at`, never `created_at` |
| `diesel::dsl::greatest` doesn't exist | Replaced with `diesel::sql_function!` macro at top of `db/mod.rs` |
| `include_never_posted` needs undeclared function | Added `query_all_tracked_user_ids_for_guild()` to Task 1 |
| Wrong `member_permissions` signature | Fetch bot's `Member` first, then call `guild.member_permissions(&bot_member)` |
| N API calls for bot filtering | Keep `Vec<Member>` throughout; use `member.user.bot` directly (no extra API calls) |
| `channel.id` vs `channel.id()` | Use `ChannelId::from(ID).send_message()` directly (no `get_channel` needed) |
| REST pagination for member list | `GetMembers` with `limit(1000)` + `after` cursor loop, break when `len() < 1000` |
| Kick rate-limit handling | 1.5s `tokio::time::sleep` between kicks |
| Bot KICK_MEMBERS check | Fetch bot Member, verify `guild.member_permissions(&bot_member).contains(KICK_MEMBERS)` |
| Missing index | `idx_user_activity_guild_last_message ON user_activity (guild_id, last_message_at)` |
| DMs edge case | `msg.guild_id.is_some()` check in passive tracking |
| Voice users | Not filtered — accepted behavior (voice-only users are inactive by text standards) |
| Gulaged users | Filtered out via `Gulag::is_user_in_gulag` check (cross-guild skip is acceptable) |
| Startup scan blocking | `tokio::spawn` background task, not inline in `ready()` |
| Feature flag | `cull` feature flag (default false), checked via `Features::check_enabled` |
| Webhook filter | `msg.webhook_id.is_none()` check (not author-based) |
| Audit reason | `kick(..., Some(&format!("Inactive {} days — /cull by {}", days, command.user.name)))` |
| Whitelist roles | Uses `["Highly Regarded", "admin"]` (matches existing codebase) |
| Output channel | All output to `cat-herding` (1224402885786472659), fallback to ephemeral |
| N+1 query in dry-run | Added `query_user_activity_for_ids()` — single query with `eq_any(user_ids)` |
| `bulk_upsert_activity` unused | Task 2's startup scan now calls `bulk_upsert_activity()` instead of inlining |
| `ChannelType` import path | Uses `serenity::all::ChannelType` to match codebase conventions |
