# Tugbot Refactoring Guide

Migrating tugbot-rs from manual Serenity handlers to a Poise-based architecture.

## What's Done (Phase 0)

- Added `poise 0.6`, `thiserror 1.0`, pinned serde/regex versions
- `src/error.rs` -- `BotError` enum with `#[from]` conversions for Diesel, Serenity, Anyhow
- `src/data.rs` -- Centralized `Data` struct (replaces TypeMap pattern)
- `src/utils/nickname.rs` -- Shared nickname logic (single source of truth)
- `src/db/queries/` -- Separated DB operations: `GulagQueries`, `FeatureQueries`, `ServerQueries`
- `src/commands/utility.rs` -- `/phony`, `/horny`, `/feature` migrated to Poise (ephemeral responses preserved)
- `src/lib.rs` -- Type aliases: `Context<'a>`, `Error`

Old handlers remain functional. Nothing is wired up to `main.rs` yet.

## Remaining Phases

### Phase 1: Gulag System

- Create `commands/moderation.rs` with Poise gulag commands
- Extract gulag business logic into a service module
- Move background tasks (`run_gulag_check`, `run_gulag_vote_check`) into a `tasks/` module
- Implement reaction-based gulag voting via Poise's `event_handler`

### Phase 2: Link Rewriters

- Move twitter/tiktok/bsky/instagram rewrite logic into Poise's `event_handler`
- Extract pure URL rewrite functions into a shared module
- Delete old handler files

### Phase 3: Wire Up main.rs

- Replace `Client::builder` + `Handler` with `poise::Framework::builder`
- Register commands, event handler, error handler, and background tasks in `setup()`

### Phase 4: Cleanup

- Remove old `handlers/` code (keep until everything is tested)
- Remove `HandlerResponse`, `DbPoolKey`, `get_pool`
- Update CLAUDE.md with new patterns
