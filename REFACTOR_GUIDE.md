# Tugbot Refactoring Guide

Migrating tugbot-rs from manual Serenity handlers to a Poise-based architecture.

## Phase 0: Foundation (Done)

- Added `poise 0.6`, `thiserror 1.0`, pinned serde/regex versions
- `src/error.rs` -- `BotError` enum with `#[from]` conversions for Diesel, Serenity, Anyhow
- `src/data.rs` -- Centralized `Data` struct (replaces TypeMap pattern)
- `src/utils/nickname.rs` -- Shared nickname logic (single source of truth)
- `src/db/queries/` -- Separated DB operations: `GulagQueries`, `FeatureQueries`, `ServerQueries`
- `src/commands/utility.rs` -- `/phony`, `/horny`, `/feature` written as Poise commands
- `src/lib.rs` -- Type aliases: `Context<'a>`, `Error`

**Current state:** Old handlers serve all traffic. Poise commands exist but are dead code
(not wired into main.rs). Everything compiles, 37 tests pass, zero warnings.

### Issues fixed (from Phase 0 review)

1. **Blocking Diesel on async runtime** -- Poise commands in `utility.rs` now wrap all
   sync DB calls in `tokio::task::spawn_blocking`.
2. **Broken row locking** -- `find_expired()`, `find_votes_ready_for_processing()`,
   `mark_released()`, `update_vote_status()`, and `mark_vote_done()` now take
   `&mut PgConnection` instead of `&DbPool`. Callers wrap find+process in a single
   `conn.transaction()` to hold `FOR UPDATE` locks until commit.
3. **Silent error swallowing** -- `find_by_user_id()` now logs pool and query errors
   via `eprintln!` before returning `None`, matching `ServerQueries::find_by_guild_id`.

## Phase 1: Wire up Poise in main.rs

Get the framework running end-to-end before migrating more code into it.

- Replace `Client::builder` + `Handler` with `poise::Framework::builder` in `main.rs`
- Pass `Data { db_pool, http }` via Poise's `setup()` callback
- Register the 3 existing Poise commands (`phony`, `horny`, `feature`)
- Keep old `EventHandler` for everything else (messages, reactions, guild_member_addition)
  using Poise's `event_handler` closure to delegate to the existing `Handler` methods
- Verify the 3 commands actually work in Discord (they're untested right now)
- Remove `DbPoolKey` / `get_pool` -- use `ctx.data().db_pool` everywhere

## Phase 2: Migrate commands

Once Poise is wired up and verified, migrate the remaining commands one at a time.

### Gulag commands
- Create `commands/moderation.rs` with Poise gulag subcommands:
  `/gulag`, `/gulag-release`, `/gulag-list`, `Add Gulag Vote` (message command)
- Extract gulag business logic out of `handlers/gulag/` into reusable functions
- Migrate reaction-based voting into Poise's `event_handler` closure
- Move background tasks (`run_gulag_check`, `run_gulag_vote_check`) -- spawn them
  in Poise's `setup()` callback

### Other commands
- `AiSlopHandler` (message command) -> Poise message command
- `teh.rs`, `elon.rs`, `derpies.rs` -- decide: keep as message event handlers or drop

## Phase 3: Migrate event handlers

- Move Twitter/TikTok/Bsky/Instagram link rewriters into Poise's `event_handler` closure
- Extract pure URL rewrite functions into `src/utils/` (they're already mostly pure)
- Delete old handler files as each one is migrated and tested

## Phase 4: Cleanup

- Remove `src/handlers/` entirely
- Remove `HandlerResponse` struct
- Remove old `#[deprecated]` functions in `src/db/mod.rs`
- Update CLAUDE.md with new patterns and architecture
