# AGENTS.md

Guidance for AI agents working on the tugbot-rs codebase.

## Project Overview

tugbot-rs is a Discord bot written in Rust using the Serenity framework. Features include:
- **LLM-powered mention handler** — responds to bot mentions with snarky, concise answers routed through four modes (research, meme-knowledge, image-analysis, casual)
- **Gulag system** — temporary role-based punishment with voting
- **Social media link rewrites** — Twitter, TikTok, Bsky, Instagram embed suppression
- **Feature flags** — database-backed toggles for features

## Quick Start

```bash
cargo build    # Build
cargo run      # Run locally (requires .env with DISCORD_TOKEN, APPLICATION_ID, DATABASE_URL)
cargo test     # Run tests
```

## Architecture

### Mention Handler (`src/handlers/mention.rs`)

Main entry point for bot interactions. When a user mentions the bot:

1. **Feature flag check** — DB key is `"is_this_real"` (backward compat)
2. **Special user gulag** — user ID `163055057254875136` gets gulagged on ANY mention
3. **Cooldown check** — 2h per user (24h for restricted user `776223452758540308`, admin `212879017257205760` bypasses)
4. **Image download** — fetches images from referenced message attachments and embeds (validated via `is_safe_url`)
5. **Prompt build** — includes author name, referenced message context, and the question
6. **pi RPC call** — sends prompt to `pi --mode rpc` subprocess via `ask_with_images()`
7. **Post response** — replies to the original message with the LLM's answer

The handler reacts with 👀 and 🤔 on receive, removes 🤔 on completion.

### pi RPC (`src/pi_rpc.rs`)

Spawns `pi --mode rpc` as a subprocess. Key details:

- **System prompt** loaded from `skills/tugbot-system-prompt.md` — contains all skill instructions inlined (research, meme-knowledge, image-analysis, casual, refusals)
- **Allowed tools:** `web_search,fetch_content` (no bash/read/write)
- **No session** — each ask is independent (`--no-session`)
- **No skill discovery** — skills are inlined in the system prompt, not loaded via `--skill` flag
- **No context files** — `--no-context-files` prevents pi from loading repo context
- **Mutex-serialized** — concurrent requests are serialized (not a concern given 2h cooldown)
- **Timeout** — configurable via `TIMEOUT_SECS` constant
- **Auto-restart** — if the pi subprocess dies, it's restarted on next ask
- **Logging** — prompts and responses are logged via `eprintln!` (visible in `journalctl -u tugbot`)

### System Prompt (`skills/tugbot-system-prompt.md`)

All routing logic and skill instructions are inlined here. Four modes:

| Mode | Use When | Length | Tools |
|------|----------|--------|-------|
| **Research** | Fact-checking, "is this real", general knowledge | 1-2 sentences | `web_search`, `fetch_content` |
| **Meme-knowledge** | SA lore, 4chan, old memes, internet history | 2-4 sentences | None (training data) |
| **Image-analysis** | User shared an image | 1-2 sentences | `web_search` for verification |
| **Casual** | Banter, greetings, personal questions, default | 1-3 sentences | None |

SA lore is baked in (Fartcar, ADTRW, Lowtax, 4chan origins). Code questions are refused with attitude. Anti-injection guardrails are always appended.

### Skills Directory (`skills/`)

Contains standalone SKILL.md files for each mode. These are **not loaded by pi** (no `--skill` flag). They exist as reference/documentation for the inlined system prompt content. When editing behavior, update both the SKILL.md and the system prompt.

### Gulag System (`src/handlers/gulag/`)

Multi-module punishment system:
- `run_gulag_check()` — background task, releases users when sentence expires
- `run_gulag_vote_check()` — background task, processes votes that reach threshold (5)
- Vote tracking via reactions and slash commands
- Uses Discord roles, not native timeouts

### Database (`src/db/`)

Diesel ORM with PostgreSQL. Schema in `src/db/schema.rs`, models in `src/db/models.rs`. Connection established per-operation.

### Feature Flags (`src/features/mod.rs`)

Database-backed toggles. Check with `Features::is_enabled(&pool, "key")`.

## Development

### Adding a New Handler

1. Create module in `src/handlers/`
2. Implement `handler()` method
3. Add to `mod.rs` exports and event dispatch
4. Add feature flag check if needed

### Modifying Bot Behavior

Edit `skills/tugbot-system-prompt.md` — this is the single source of truth for how the bot responds. The file is loaded at startup via `--append-system-prompt`.

### Debugging

```bash
# View bot logs (includes [pi_rpc] prompt/response logging)
journalctl -u tugbot -f

# Check what prompt was sent and what the LLM replied
journalctl -u tugbot | grep pi_rpc
```

## Deployment

```bash
ssh root@tugbot
update-tugbot   # pulls, migrates, builds, restarts
```

Builds use `--locked` (Cargo.lock) to prevent dependency drift.

## Conventions

- **Logging:** Use `eprintln!("[module] message")` for runtime logging (goes to journalctl)
- **Error handling:** Use `context()` from anyhow for error chains
- **Discord:** Use `CreateMessage::new().content().reference_message()` for replies
- **Feature flags:** DB key `"is_this_real"` is used for the mention handler (rename via migration later)
