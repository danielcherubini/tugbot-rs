# AGENTS.md

Guidance for AI agents working on the tugbot-rs codebase.

## Project Overview

tugbot-rs is a Discord bot written in Rust using the Serenity framework. Features include:
- **LLM-powered mention handler** — responds to bot mentions with snarky, concise answers routed through five modes (research, meme-knowledge, image-analysis, assassination, casual)
- **Gulag system** — temporary role-based punishment with voting
- **Social media link rewrites** — Twitter, TikTok, Bsky, Instagram embed suppression
- **Feature flags** — database-backed toggles for features
- **Misc handlers** — AI Slop, Goku Poll, horny/phony prefix toggles, "teh" reactions, derpies, elon auto-gulag

## Quick Start

```bash
cargo build    # Build
cargo run      # Run locally (requires .env with DISCORD_TOKEN, APPLICATION_ID, DATABASE_URL, ADMIN_USER_ID, SLOW_USER_IDS)
cargo test     # Run tests
cargo clippy --all-targets   # Lint (must be warning-free)
```

### Required Environment Variables

| Var | Purpose | Required |
|-----|---------|----------|
| `DISCORD_TOKEN` | Bot token | Yes |
| `APPLICATION_ID` | Discord application ID | Yes |
| `DATABASE_URL` | PostgreSQL connection string | Yes |
| `ADMIN_USER_ID` | Discord user ID that bypasses mention cooldowns | No (default: 0 = disabled) |
| `SLOW_USER_IDS` | Comma-separated user IDs with slower cooldown + auto-gulag on mention | No (default: empty) |
| `TUGBOT_SKILLS_DIR` | Override the skills dir path (for prod deployments) | No (default: project root) |
| `RUST_LOG` / `RUST_BACKTRACE` | Standard Rust env vars | No |

## Architecture

### Mention Handler (`src/handlers/mention.rs`)

Main entry point for bot interactions. When a user mentions the bot:

1. **Feature flag check** — DB key is `"is_this_real"` (backward compat)
2. **Special user gulag** — users in `SLOW_USER_IDS` get auto-gulagged on ANY mention
3. **Bot mention parsing** — tokenize-and-filter on `msg.mentions` semantics (no string-replace)
4. **Cooldown check** — 30m normal (`COOLDOWN_SECS = 1800`), 2h for slow users (`SLOW_COOLDOWN_SECS = 7200`), admin bypass
5. **Image download** — fetches images from referenced message attachments and embeds (10s timeout, validated via `is_safe_url`, MIME detected via `mime_for_url`)
6. **Prompt build** — includes author name, referenced message context, and the question
7. **pi RPC call** — sends prompt to `pi --mode rpc` subprocess via `ask_with_images()`
8. **Empty-response guard** — if the LLM returns empty, skip posting AND skip cooldown update
9. **Post response** — replies to the original message with the LLM's answer

The handler reacts with 👀 and 🤔 on receive, removes 🤔 on completion.

### pi RPC (`src/pi_rpc.rs`)

Spawns `pi --mode rpc` as a long-lived subprocess with a **supervisor task + mpsc channel** architecture:

- **Supervisor task** owns the subprocess (child, stdin, stdout) for its entire lifetime
- **Request channel** — `ask()` sends a `Request { req_id, prompt, images, oneshot_response_tx }` via `mpsc::unbounded_channel`
- **Response** — supervisor writes the request to pi's stdin, reads from stdout, sends the result back via the oneshot
- **Concurrent `ask()` calls** queue through the channel — no mutex held across I/O
- **System prompt** loaded from `skills/tugbot-system-prompt.md` (via `--append-system-prompt`)
- **Allowed tools:** `web_search,fetch_content` (no bash/read/write)
- **No session** — each ask is independent (`--no-session`)
- **No skill discovery** — skills are inlined in the system prompt, not loaded via `--skill` flag
- **No context files** — `--no-context-files` prevents pi from loading repo context
- **Timeout** — 300s per ask via `TIMEOUT_SECS` constant (timeout fires on the await of the oneshot)
- **Auto-restart** — if the pi subprocess dies (EOF on stdout), it's restarted before the next request
- **Logging** — prompts and responses are logged via `eprintln!` (visible in `journalctl -u tugbot`)

The `PiRpc` handle is stored in Serenity's `Data` under `PiRpcKey` and accessed via `get_pi_rpc(ctx)`. Bot config is similarly stored under `ConfigKey` and accessed via `get_config(ctx)`.

### System Prompt (`skills/tugbot-system-prompt.md`)

All routing logic and skill instructions are inlined here. Five modes:

| Mode | Use When | Length | Tools |
|------|----------|--------|-------|
| **Research** | Fact-checking, "is this real", general knowledge | 1-2 sentences | `web_search`, `fetch_content` |
| **Meme-knowledge** | SA lore, 4chan, old memes, internet history | 2-4 sentences | None (training data) |
| **Image-analysis** | User shared an image | 1-2 sentences | `web_search` for verification |
| **Assassination** | "how do I kill X" — internal joke, absurd impractical methods | 1-3 sentences | None |
| **Casual** | Banter, greetings, personal questions, default | 1-3 sentences | None |

SA lore is baked in (Fartcar, ADTRW, Lowtax, 4chan origins). Code questions are refused with attitude. Anti-injection guardrails are always appended.

### Skills Directory (`skills/`)

Contains standalone SKILL.md files for each mode. These are **not loaded by pi** (no `--skill` flag). They exist as reference/documentation for the inlined system prompt content. When editing behavior, update both the SKILL.md and the system prompt.

### Gulag System (`src/handlers/gulag/`)

Multi-module punishment system:
- `run_gulag_check()` — background task, releases users when sentence expires
- `run_gulag_vote_check()` — background task, processes votes that reach threshold (5)
- Vote tracking via reactions and slash commands
- **Pagination** — `gulag_reaction.rs` paginates Discord's `get_reaction_users` API to fetch all voters (Discord caps at 100 per request)
- **Discord 404 detection** — `Gulag::is_discord_not_found()` walks the error chain looking for `serenity::Error::Http` with status 404, then cleans up stale DB rows (Unknown Guild / Unknown Message). Replaces fragile string matching on error messages.
- Uses Discord roles, not native timeouts

### Database (`src/db/`)

Diesel ORM with PostgreSQL. Schema in `src/db/schema.rs`, models in `src/db/models.rs`. Connection established per-operation via the r2d2 pool (`max_size=15`, `connection_timeout=30s`). The pool is established once in `main()` and stored in Serenity's `Data` under `DbPoolKey`.

### Feature Flags (`src/features/mod.rs`)

Database-backed toggles. Two APIs:
- `Features::is_enabled(&pool, "key") -> bool` — silent, returns `false` on error (background tasks)
- `Features::check_enabled(&pool, "key") -> Result<bool>` — propagates error (user-facing commands)

`Features::all(&pool)` lists all features. `Features::update(&pool, "key", bool)` toggles.

### Handlers — shared patterns

- **Prefix handler** (`src/handlers/prefix_handler.rs`) — generic `/<prefix>` command for the "horny" and "phony" nick prefixes. Reads `command.data.name` to determine the prefix, looks up the matching feature flag. Single source for both commands.
- **Highly Regarded permission check** — duplicated in `gulag_handler.rs`, `gulag_remove_handler.rs`, and `ai_slop.rs` via `Gulag::member_has_any_role(http, guild_id, &member, &["Highly Regarded", "admin"])`. Not yet extracted to a shared helper.
- **Gulag release duration** — exponential, `Gulag::get_gulag_duration_for_offense(count)` = `1800 * 2^count` seconds, capped at u64::MAX. Used by `ai_slop` and `goku_poll` for repeat offenders.

## Development

### Adding a New Handler

1. Create module in `src/handlers/`
2. Implement `handler()` method
3. Add to `mod.rs` exports and event dispatch
4. Add feature flag check if needed (use `Features::is_enabled` for background, `Features::check_enabled` for user-facing)

### Modifying Bot Behavior

Edit `skills/tugbot-system-prompt.md` — this is the single source of truth for how the bot responds. The file is loaded at startup via `--append-system-prompt`.

### Adding a New Slash Command

1. Create a handler in `src/handlers/<name>.rs` with `setup_command()` and `setup_interaction()`
2. Add the module to `src/handlers/mod.rs` (mod declaration + use)
3. Register the command in `ready()` via `server.guild_id.set_commands(...)`
4. Dispatch in `interaction_create()` (the `match command.data.name.as_str()` block)

### Debugging

```bash
# View bot logs (includes [pi_rpc] prompt/response logging)
journalctl -u tugbot -f

# Check what prompt was sent and what the LLM replied
journalctl -u tugbot | grep pi_rpc

# Check gulag vote processing
journalctl -u tugbot | grep gulag_reaction
```

## Deployment

```bash
ssh root@tugbot
update-tugbot   # pulls, migrates, builds, restarts
```

Builds use `--locked` (Cargo.lock) to prevent dependency drift. All dependencies in `Cargo.toml` are pinned (no wildcards).

## Conventions

- **Logging:** Use `eprintln!("[module] message")` for runtime logging (goes to journalctl). Never use `println!`.
- **Error handling:** Use `context()` / `with_context()` from anyhow for error chains. For Discord API errors that should trigger cleanup (404s on guild/message), check via `Gulag::is_discord_not_found` — never match on `e.to_string()`.
- **Discord:** Use `CreateMessage::new().content().reference_message()` for replies. Use `ctx.http.get_member(guild, user).await?` rather than `unwrap()`.
- **Feature flags:** DB key `"is_this_real"` is used for the mention handler (rename via migration later). The same name is the prefix for the "phony" and "horny" commands.
- **ID conversions:** Discord IDs are `u64` in code, `i64` in DB. Use `i64::try_from(u64).with_context(...)` at the boundary — never `as i64` (silent truncation).
- **Discord ID type:** `Member::edit` requires `&mut self` even for read-then-edit flows. Keep `let mut mem` in those cases.
- **HTTP:** Build a `reqwest::Client` with a 10s timeout for any external HTTP call. Don't use the default `reqwest::get` shortcut (no timeout = potential indefinite hang).
