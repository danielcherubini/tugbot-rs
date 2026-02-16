# Tugbot Refactoring Guide

## Overview

This document outlines the refactoring of tugbot-rs from a manual Serenity-based bot to a modern Poise-based architecture.

## What's Been Completed ✅

### 1. Dependencies Updated
- Added `poise = "0.6"` - Modern command framework
- Added `thiserror = "1.0"` - Structured error handling
- Updated `serde` with derive features
- Specified explicit versions for regex

### 2. New Project Structure Created

```
src/
├── commands/          # NEW: Poise command modules
│   ├── mod.rs
│   ├── moderation.rs  # Gulag commands (TODO)
│   └── utility.rs     # ✅ DONE: phony, horny, feature
├── services/          # NEW: Business logic layer
│   ├── mod.rs
│   ├── gulag.rs       # TODO
│   └── link_rewriter.rs  # TODO
├── tasks/             # NEW: Background tasks
│   ├── mod.rs
│   ├── gulag_checker.rs  # TODO
│   └── vote_processor.rs # TODO
├── event_handlers/    # NEW: Non-command events
│   ├── mod.rs         # ✅ EventHandler implementation
│   ├── message.rs     # TODO: Link rewrites
│   └── reaction.rs    # TODO: Gulag voting
├── db/
│   ├── queries/       # NEW: Separated DB operations
│   │   ├── mod.rs
│   │   ├── gulag.rs   # ✅ DONE
│   │   ├── features.rs # ✅ DONE
│   │   └── servers.rs  # ✅ DONE
│   └── ... (existing)
├── data.rs            # NEW: ✅ Centralized state
├── error.rs           # NEW: ✅ Structured errors
└── ... (existing modules)
```

### 3. Core Infrastructure ✅

**`src/error.rs`** - Structured error types using `thiserror`:
- `BotError` enum with variants for all error types
- Automatic conversion from Diesel, Serenity, Anyhow errors
- Type alias `Result<T>` for convenience

**`src/data.rs`** - Centralized application state:
- `Data` struct containing `db_pool` and `http` client
- Accessible in all commands via `ctx.data()`
- Replaces TypeMap pattern

**`src/lib.rs`** - Updated exports:
- Re-exports `Data`, `BotError`, `Result`
- Type aliases for Poise: `Context<'a>` and `Error`

### 4. Database Query Layer ✅

Separated database operations from business logic:

**`db/queries/gulag.rs`**:
- `GulagQueries::create()` - Create gulag entry
- `GulagQueries::find_by_user_id()` - Check if user in gulag
- `GulagQueries::find_expired()` - Get users ready for release
- `GulagQueries::add_time()` - Extend gulag sentence
- `GulagQueries::mark_released()` - Update status
- `GulagQueries::delete()` - Remove entry
- Vote-related queries

**`db/queries/features.rs`**:
- `FeatureQueries::all()` - Get all features
- `FeatureQueries::is_enabled()` - Check feature status
- `FeatureQueries::update()` - Toggle feature

**`db/queries/servers.rs`**:
- `ServerQueries::create()` - Create server entry
- `ServerQueries::find_by_guild_id()` - Find server
- `ServerQueries::all()` - Get all servers

### 5. Commands Migrated to Poise ✅

**`commands/utility.rs`**:

Migrated from old handler pattern to Poise commands:

```rust
// OLD (manual command registration):
pub struct Phony;
impl Phony {
    pub fn setup_command() -> CreateCommand { ... }
    pub async fn setup_interaction(ctx: &Context, command: &CommandInteraction) -> HandlerResponse { ... }
}

// NEW (Poise):
#[poise::command(slash_command, guild_only, category = "Utility")]
pub async fn phony(ctx: Context<'_>) -> Result<(), Error> { ... }
```

**Benefits**:
- ~70% less boilerplate code
- Automatic argument parsing
- Type-safe context access
- Built-in error handling
- Cleaner, more maintainable code

**Commands migrated**:
- `/phony` - Toggle phony status
- `/horny` - Toggle horny status
- `/feature [name]` - List/toggle features

## What's Next: TODO 🚧

### Phase 1: Migrate Gulag System (CURRENT)

The gulag system is the most complex feature. It needs:

1. **Create `services/gulag.rs`** - Business logic:
   - `GulagService::add_to_gulag()` - Orchestrate adding user
   - `GulagService::release_from_gulag()` - Release user
   - `GulagService::handle_member_rejoin()` - Re-apply on rejoin
   - Helper functions for role/channel finding

2. **Migrate gulag commands to `commands/moderation.rs`**:
   ```rust
   #[poise::command(slash_command, subcommands("add", "remove", "list", "vote"))]
   pub async fn gulag(ctx: Context<'_>) -> Result<(), Error> { ... }
   ```
   - `/gulag add` - Send user to gulag
   - `/gulag remove` - Release user
   - `/gulag list` - List all in gulag
   - Message command: "Add Gulag Vote"

3. **Move background tasks to `tasks/`**:
   - `tasks/gulag_checker.rs` - Check for expired sentences
   - `tasks/vote_processor.rs` - Process votes at threshold
   - Improve error handling
   - Reduce polling frequency (1s → 10s)

4. **Implement reaction handlers in `event_handlers/reaction.rs`**:
   - `handle_reaction_add()` - Track gulag votes
   - `handle_reaction_remove()` - Remove votes

### Phase 2: Migrate Link Rewriters

Move link rewriting to event handlers:

1. **Create `services/link_rewriter.rs`**:
   - Extract rewrite logic from handlers
   - Pure functions for URL transformations

2. **Implement `event_handlers/message.rs`**:
   ```rust
   pub async fn handle_message(ctx: &Context, msg: &Message, data: &Data) -> Result<(), Error> {
       // Check feature flags
       if FeatureQueries::is_enabled(&data.db_pool, "twitter") {
           if let Some(rewritten) = rewrite_twitter_link(&msg.content) {
               // Suppress embed and post rewrite
           }
       }
       // ... same for tiktok, bsky, instagram
   }
   ```

3. **Delete old handler files**:
   - `handlers/twitter.rs`
   - `handlers/tiktok.rs`
   - `handlers/bsky.rs`
   - `handlers/instagram.rs`

### Phase 3: Update main.rs

Replace current handler setup with Poise framework:

```rust
#[tokio::main]
async fn main() {
    let config = Config::get_config();
    let pool = establish_pool();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::commands(), // All commands
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                // Register commands per guild
                let data = Data::new(pool, Arc::clone(&ctx.http));
                
                // Start background tasks
                tokio::spawn(tasks::gulag_checker::start(ctx.clone(), data.clone()));
                tokio::spawn(tasks::vote_processor::start(ctx.clone(), data.clone()));
                
                Ok(data)
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(config.token, config.intents)
        .framework(framework)
        .await
        .expect("Error creating client");

    client.start().await.expect("Client error");
}
```

### Phase 4: Testing & Cleanup

1. **Test all functionality**:
   - Run bot in development
   - Test each command
   - Verify gulag system works
   - Check link rewrites
   - Test feature flags

2. **Remove old code**:
   - Delete `handlers/` directory (except needed utils)
   - Remove `HandlerResponse` struct
   - Clean up unused imports
   - Remove old `Handler` implementation

3. **Update deployment**:
   - Test on staging if available
   - Deploy to production
   - Monitor for issues

## Key Benefits of This Refactor

### Code Reduction
- **~50-70% less boilerplate** for commands
- **Cleaner separation of concerns**
- **Easier to test** (business logic separated)

### Maintainability
- **Industry standard patterns** (Poise is the de facto standard)
- **Type-safe everything** (leverage Rust's type system)
- **Centralized state** (no more TypeMap lookups)

### Developer Experience
- **Faster command development** (Poise macros)
- **Better error messages** (structured errors)
- **Easier onboarding** (familiar patterns)

### Architecture Improvements
- **Services layer** separates business logic
- **Query layer** separates database operations
- **Event handlers** separate non-command logic
- **Background tasks** isolated and testable

## Migration Strategy

### Safety First
1. Keep old handlers working during migration
2. Migrate features incrementally
3. Test each feature thoroughly
4. Deploy gradually if possible

### Rollback Plan
- Git tags at each milestone
- Old code remains until fully tested
- Can revert individual features if needed

## Current Status Summary

**✅ Completed**:
- Dependencies updated
- New directory structure
- Core infrastructure (Data, Error types)
- Database query layer
- Utility commands (phony, horny, feature)
- Event handler framework

**🚧 In Progress**:
- None (ready for Phase 1)

**TODO**:
- Gulag system migration (Phase 1)
- Link rewriters (Phase 2)
- Main.rs update (Phase 3)
- Testing & cleanup (Phase 4)

## Estimated Effort

- **Phase 1** (Gulag): 4-6 hours
- **Phase 2** (Link rewrites): 2-3 hours
- **Phase 3** (main.rs): 1-2 hours
- **Phase 4** (Testing/cleanup): 2-3 hours

**Total**: 9-14 hours of focused work

## Next Steps

To continue the refactor:

1. Start with Phase 1: Migrate gulag commands
2. Create `services/gulag.rs` with business logic
3. Update `commands/moderation.rs` with Poise commands
4. Move background tasks to `tasks/` directory
5. Test gulag functionality thoroughly

Would you like me to continue with Phase 1 now?
