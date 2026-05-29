# pi RPC Integration Plan

**Goal:** Replace the direct Ollama + Exa integration in `is_this_real` with a persistent pi RPC subprocess that handles LLM calls, tool use (web_search), and skills.

**Architecture:** A new `pi_rpc` module spawns `pi --mode rpc --no-session` as a persistent subprocess on bot startup. The subprocess is stored in Serenity's `Data` alongside `DbPool`. The `is_this_real` handler sends prompts via `/skill:is-this-real` and receives the final response. A `tokio::sync::Mutex` serializes concurrent requests. On subprocess crash, the next `ask()` call auto-restarts pi.

**Tech Stack:** Rust, tokio (process + async I/O), serde_json (JSONL parsing)

---

### Task 1: Create `pi_rpc` module

**Context:**
This is the core new module — it manages the persistent pi subprocess and provides a simple `ask()` API. It spawns `pi --mode rpc --no-session`, communicates via JSONL over stdin/stdout, and handles subprocess lifecycle (spawn, restart on crash).

**Dependencies:** None (first task)

**Files:**
- Create: `src/pi_rpc.rs`
- Modify: `src/lib.rs` — add `pub mod pi_rpc;`
- Modify: `Cargo.toml` — add `process` and `io-util` to tokio features

**What to implement:**

In `src/lib.rs`, add `pub mod pi_rpc;`.

In `Cargo.toml`, change:
```toml
tokio = { version = "1.15.0", features = ["time", "macros", "rt-multi-thread"] }
```
to:
```toml
tokio = { version = "1.15.0", features = ["time", "macros", "rt-multi-thread", "process", "io-util"] }
```

In `src/pi_rpc.rs`:

**Imports:**
```rust
use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, BufReader, AsyncWriteExt, BufWriter};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
```

> **Important:** Use `tokio::process::Child` (NOT `std::process::Child`). Use `tokio::process::Command` (NOT `std::process::Command`). Only `Stdio` comes from `std::process`.

> **Request IDs:** Use a simple atomic counter — NO `uuid` crate needed:
> ```rust
> static NEXT_REQ_ID: AtomicU64 = AtomicU64::new(1);
> fn next_id() -> String { format!("req-{}", NEXT_REQ_ID.fetch_add(1, Ordering::Relaxed)) }
> ```

**Struct:**
```rust
pub struct PiRpc {
    inner: Mutex<PiRpcInner>,
}

struct PiRpcInner {
    child: Option<Child>,
    stdin: Option<BufWriter<tokio::io::WriteHalf<tokio::process::ChildStdio>>>,
    stdout: BufReader<tokio::io::ReadHalf<tokio::process::ChildStdio>>,
}
```

The `Mutex` wraps all state so `ask()` calls are serialized. The `Option<Child>` and `Option<stdin>` allow detecting and recovering from dead subprocesses.

**Constants:**
```rust
const TIMEOUT_SECS: u64 = 60;
const PI_BINARY: &str = "pi";
```

**Methods:**

1. **`pub async fn spawn() -> Result<Arc<Self>>`**
   - Spawn `pi --mode rpc --no-session` with piped stdin/stdout
   - Split stdin into write half, stdout into read half
   - Wrap in BufWriter/BufReader
   - Return `Arc<Self>`
   - If spawn fails, return `Err` with context

2. **`pub async fn ask(&self, prompt: &str) -> Result<String>`**
   - Lock `self.inner`
   - Check if child/stdin is `None` (dead process) — if so, call `restart()` internally
   - Generate unique request ID: `format!("req-{}", uuid::Uuid::now_v7())` or use a simple atomic counter
   - Write JSONL command to stdin: `{"id":"<id>","type":"prompt","message":"<escaped prompt>"}`
   - Read stdout line by line with timeout (60s):
     - Parse each line as JSON
     - Skip lines without `"id"` field (these are events)
     - When `response.id` matches our id and `success == true` → prompt accepted, continue reading
     - When `response.id` matches our id and `success == false` → return `Err`
     - When event `"type" == "agent_end"` → extract text from messages array (find last message with `"role" == "assistant"`, get its text content), return the text
     - When event `"type" == "agent_end"` with error → return `Err`
   - If stdout returns EOF (process died) → set child/stdin to `None`, return `Err`
   - If timeout expires → return `Err`
   - Unlock mutex (via Drop when guard goes out of scope)

3. **`async fn restart(&mut PiRpcInner) -> Result<()>`** (private)
   - Kill existing child if still running
   - Spawn new `pi --mode rpc --no-session`
   - Replace child, stdin, stdout
   - eprintln! with restart notice

**JSON escaping for prompt:** Use `serde_json::json!()` or manually escape — the prompt must be valid JSON string value. Use `serde_json::Value::String(prompt.to_string())` and `serde_json::to_string()` for the full command.

**Extracting assistant text from `agent_end`:** The `agent_end` event has a `messages` array. Find the last message where `role == "assistant"`. Its `content` field may be a string or an array of content blocks. Handle both:
- If string (`content` is a JSON string): return it directly
- If array (`content` is a JSON array): concatenate all `{"type": "text", "text": "..."}` blocks

**JSONL protocol example (for reference):**

Request (stdin):
```json
{"id":"req-1","type":"prompt","message":"/skill:is-this-real Someone said: \"foo\" — The question is: \"is this real?\""}
```

Response (stdout, first matching line — command accepted):
```json
{"id":"req-1","type":"response","command":"prompt","success":true}
```

Events (stdout, no `id` field — stream until agent_end):
```json
{"type":"agent_start"}
{"type":"turn_start"}
{"type":"message_update","message":{...},"assistantMessageEvent":{"type":"text_delta","delta":"According"}}
{"type":"tool_execution_start","toolCallId":"call_abc","toolName":"web_search","args":{"query":"..."}}
{"type":"tool_execution_end","toolCallId":"call_abc","toolName":"web_search","result":{...}}
{"type":"turn_end","message":{...},"toolResults":[...]}
{"type":"agent_end","messages":[{"role":"user","content":"..."},{"role":"assistant","content":"Yes that's real"}]}
```

The `ask()` method should:
1. Write the request to stdin
2. Read lines until it finds a response with matching `id` (command accepted/rejected)
3. Continue reading events until `agent_end` (or error/timeout)
4. Extract text from the last assistant message in `agent_end.messages`

**Testing approach:** The `PiRpc` module spawns a real subprocess and parses live JSONL — unit testing requires mocking the process, which is complex and fragile. Instead:
- Add a `#[cfg(test)]` module with a **smoke test** that spawns pi RPC and sends a trivial prompt (requires pi installed locally)
- The real integration test is the `is_this_real` handler in Discord (manual testing)
- Focus code review on the JSONL parsing logic for correctness

**Steps:**
- [ ] Update `Cargo.toml`: add `"process"` and `"io-util"` to tokio features
- [ ] Add `pub mod pi_rpc;` to `src/lib.rs`
- [ ] Create `src/pi_rpc.rs` with struct definitions, imports, constants
- [ ] Implement `spawn()` — spawn subprocess, pipe stdin/stdout, return Arc<Self>
- [ ] Implement `ask()` — send prompt via JSONL, read events, extract response from agent_end
- [ ] Implement private `restart()` — kill old process, spawn new one
- [ ] Handle JSON parsing errors gracefully (skip unparseable lines with eprintln)
- [ ] Handle EOF on stdout (process death) — set inner state to None, return Err
- [ ] Add 60-second timeout on the entire `ask()` operation using `tokio::time::timeout`
- [ ] Run `cargo build`
  - Did it succeed? If not, fix compilation errors and re-run.
- [ ] Run `cargo test`
  - Did all existing tests pass? If not, fix and re-run.
- [ ] Commit with message: "feat: add pi_rpc module for persistent pi RPC subprocess"

**Acceptance criteria:**
- [ ] `cargo build` succeeds
- [ ] `spawn()` creates a `pi --mode rpc --no-session` subprocess with piped stdin/stdout
- [ ] `ask()` sends a JSONL prompt command and waits for `agent_end` event
- [ ] `ask()` returns the last assistant message text from `agent_end`
- [ ] `ask()` is mutex-serialized — concurrent calls don't interfere
- [ ] `ask()` has a 60-second timeout
- [ ] Process death (EOF on stdout) is detected and returns `Err`
- [ ] `restart()` spawns a fresh subprocess and replaces internal state
- [ ] All existing tests still pass

---

### Task 2: Register `PiRpc` in bot data layer

**Dependencies:** Task 1 (`PiRpc` struct must exist and compile)

**Context:
Make `PiRpc` available to all handlers via Serenity's `Data`, following the same pattern as `DbPool`. Initialize on `ready()`, accessible via a helper function.

**Files:**
- Modify: `src/handlers/mod.rs` — add `PiRpcKey` TypeMapKey, `get_pi_rpc()` helper, init in `ready()`

**What to implement:**

In `src/handlers/mod.rs`:

1. **Add import:**
```rust
use crate::pi_rpc::PiRpc;
```

2. **Add TypeMapKey (next to `DbPoolKey`):**
```rust
pub struct PiRpcKey;

impl serenity::prelude::TypeMapKey for PiRpcKey {
    type Value = std::sync::Arc<PiRpc>;
}
```

3. **Add helper function (next to `get_pool()`):**
```rust
pub async fn get_pi_rpc(ctx: &serenity::client::Context) -> std::sync::Arc<PiRpc> {
    let data = ctx.data.read().await;
    data.get::<PiRpcKey>()
        .expect("Expected PiRpc in TypeMap")
        .clone()
}
```

4. **In `ready()` method, after existing setup (after `Gulag::run_gulag_vote_check`):**
```rust
// Start pi RPC subprocess
match PiRpc::spawn().await {
    Ok(pi_rpc) => {
        let data = ctx.data.write().await;
        data.insert::<PiRpcKey>(pi_rpc);
        println!("pi RPC subprocess started");
    }
    Err(e) => {
        eprintln!("Failed to start pi RPC subprocess: {} — is_this_real feature will not work", e);
    }
}
```

Do NOT make the bot crash if pi fails to start — log error and let other features work. The `is_this_real` handler should gracefully handle a missing PiRpc (check with `data.get::<PiRpcKey>()` instead of `expect`, or keep `expect` in `get_pi_rpc` and do the check in the handler itself).

**Preferred approach:** Keep `get_pi_rpc()` with `expect()`, but in `is_this_real` handler, check availability before calling:
```rust
let pi_rpc = match (ctx.data.read().await).get::<PiRpcKey>() {
    Some(rpc) => rpc.clone(),
    None => {
        eprintln!("[is_this_real] pi RPC not available");
        return;
    }
};
```

**Steps:**
- [ ] Add `PiRpcKey` TypeMapKey to `src/handlers/mod.rs`
- [ ] Add `get_pi_rpc()` helper function to `src/handlers/mod.rs`
- [ ] Add pi RPC initialization in `ready()` with error handling (log, don't crash)
- [ ] Add `use crate::pi_rpc::PiRpc;` import
- [ ] Run `cargo build`
  - Did it succeed? If not, fix compilation errors and re-run.
- [ ] Run `cargo test`
  - Did all tests pass? If not, fix and re-run.
- [ ] Commit with message: "feat: register PiRpc in bot data layer with graceful init"

**Acceptance criteria:**
- [ ] `cargo build` succeeds
- [ ] `PiRpc` is spawned in `ready()` and stored in Serenity Data
- [ ] If spawn fails, bot continues running (other features unaffected)
- [ ] `get_pi_rpc()` helper follows `DbPoolKey`/`get_pool()` pattern
- [ ] All existing tests pass

---

### Task 3: Refactor `is_this_real.rs` to use PiRpc

**Dependencies:** Tasks 1 + 2 (`PiRpc` must exist and be registered in bot data layer)

**Context:
Replace the Ollama + Exa two-pass flow with a single `PiRpc::ask()` call. The handler sends `/skill:is-this-real` with the question context, and pi handles LLM inference, tool use (web_search), and persona through the skill. This removes ~80 lines of Ollama/Exa orchestration code.

**Files:**
- Modify: `src/handlers/is_this_real.rs`

**What to implement:**

**Remove entirely:**
- `use crate::exa;` import
- `use serde::{Deserialize, Serialize};` (no longer needed for Ollama structs)
- `SYSTEM_PROMPT` constant
- `get_ollama_url()` function
- `get_ollama_model()` function
- `OllamaRequest`, `OllamaMessage`, `OllamaResponse`, `OllamaChoice`, `OllamaMessageContent` structs
- `call_ollama()` async function
- Uncertainty detection logic (the `uncertainty_markers` array and `is_uncertain` check)
- Exa search call and second-pass retry logic

**Keep unchanged:**
- Feature flag check
- Bot mention check
- Guild ID check
- Special user gulag handler (`handle_special_user_gulag` method)
- Reply check and referenced message fetch
- Question extraction (strip bot mention)
- Fuzzy trigger matching
- Cooldown check and update
- :eyes: reaction
- Final Discord message posting

**Replace the LLM section (steps 11+ in current handler) with:**

After the :eyes: reaction and before posting the response:

```rust
// Get pi RPC connection
let pi_rpc = match (ctx.data.read().await).get::<crate::handlers::PiRpcKey>() {
    Some(rpc) => rpc.clone(),
    None => {
        eprintln!("[is_this_real] pi RPC not available");
        return;
    }
};

// Build prompt for the skill
let prompt = format!(
    "/skill:is-this-real Someone said: \"{}\" — The question is: \"{}\"",
    referenced_msg.content.replace('"', "\\\""),
    question.replace('"', "\\\"")
);

// Ask pi
let final_text = match pi_rpc.ask(&prompt).await {
    Ok(text) => text.trim().to_string(),
    Err(e) => {
        eprintln!("[is_this_real] pi RPC ask failed: {}", e);
        // Post error message to Discord
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
```

Then the existing "post response to Discord" section uses `final_text` instead of the old `llm_text`/`second pass` variable.

**Also remove:**
- `use std::time::Duration;` — was used for Ollama client timeout only. Keep `SystemTime` (used for cooldown).
- `use serde::{Deserialize, Serialize};` — was used for Ollama structs only. Remove entirely (tests use `rapidfuzz` only, no serde).
- `use crate::exa;` — Exa module removed.

**Keep unchanged:**
- `use std::sync::Arc;` — used in `handle_special_user_gulag` signature (`http: &Arc<Http>`)
- `use std::time::SystemTime;` — used for cooldown
- All `crate::db`, `crate::features`, `crate::handlers::gulag` imports

**Steps:**
- [ ] Remove Ollama-related imports, structs, constants, and `call_ollama()` function from `is_this_real.rs`
- [ ] Remove Exa import and uncertainty detection + second-pass logic
- [ ] Replace LLM call section with `PiRpc::ask()` call as shown above
- [ ] Add pi RPC availability check before `ask()`
- [ ] Add error handling — post friendly message to Discord on failure
- [ ] Update the Discord post section to use `final_text` from pi
- [ ] Remove unused imports (`Duration`, `Serialize`, etc.)
- [ ] Run `cargo build`
  - Did it succeed? If not, fix compilation errors and re-run.
- [ ] Run `cargo test`
  - Did existing tests (fuzzy matching) still pass? If not, fix and re-run.
- [ ] Commit with message: "refactor: replace Ollama/Exa with pi RPC in is_this_real handler"

**Acceptance criteria:**
- [ ] `cargo build` succeeds
- [ ] No Ollama or Exa references remain in `is_this_real.rs`
- [ ] Handler sends `/skill:is-this-real` prompt to pi RPC
- [ ] Handler posts pi's response to Discord
- [ ] Handler posts friendly error message if pi RPC fails
- [ ] Handler gracefully returns if pi RPC not available
- [ ] Special user gulag logic unchanged
- [ ] Cooldown logic unchanged
- [ ] Fuzzy trigger matching tests still pass
- [ ] All existing tests pass

---

### Task 4: Remove `exa.rs` module

**Dependencies:** Task 3 (all references to `crate::exa` must have been removed first)

**Context:
The Exa module is no longer used — pi's built-in `web_search` tool replaces it. Remove the file and its module declaration.

**Files:**
- Delete: `src/exa.rs`
- Modify: `src/lib.rs` — remove `pub mod exa;`

**What to implement:**

1. In `src/lib.rs`, remove the line `pub mod exa;`
2. Delete `src/exa.rs`

Verify no other files reference `crate::exa`:
```bash
grep -r "crate::exa\|use.*exa" src/ --include="*.rs"
```
Should return nothing (the `is_this_real.rs` import was removed in Task 3).

**Steps:**
- [ ] Remove `pub mod exa;` from `src/lib.rs`
- [ ] Delete `src/exa.rs`
- [ ] Run `grep -r "crate::exa\|use.*exa" src/ --include="*.rs"` to confirm no remaining references
- [ ] Run `cargo build`
  - Did it succeed? If not, fix compilation errors and re-run.
- [ ] Run `cargo test`
  - Did all tests pass? If not, fix and re-run.
- [ ] Commit with message: "refactor: remove exa module — replaced by pi RPC web_search"

**Acceptance criteria:**
- [ ] `src/exa.rs` is deleted
- [ ] `pub mod exa;` removed from `src/lib.rs`
- [ ] No references to `exa` module remain in codebase
- [ ] `cargo build` succeeds
- [ ] All tests pass

---

### Task 5: Verification, formatting, and cleanup

**Dependencies:** Tasks 1–4 (all changes must be complete)

**Context:
Final verification — formatting, clippy, release build, and env var cleanup.

**Files:**
- Modify: `Cargo.toml` — potentially remove unused dependencies

**Steps:**
- [ ] Run `cargo fmt`
- [ ] Run `cargo clippy --fix --allow-dirty` — fix any remaining warnings manually
- [ ] Run `cargo build` — verify clean build
- [ ] Run `cargo build --release` — verify release build
- [ ] Run `cargo test` — verify all tests pass
- [ ] Check if `serde` `derive` feature still needed (check all files for `#[derive(Serialize)]` / `#[derive(Deserialize)]`)
- [ ] Check if `rapidfuzz` still needed (yes — fuzzy matching in `is_this_real`)
- [ ] Verify no references to `TAMA_TOKEN`, `OLLAMA_URL`, `OLLAMA_MODEL`, or `EXA_API_KEY` env vars remain in code (all four were used by the removed Ollama/Exa integration)
- [ ] Run `grep -r "TAMA_TOKEN\|OLLAMA_URL\|OLLAMA_MODEL\|EXA_API_KEY" src/ --include="*.rs"` — should return nothing
- [ ] Update `.env` if needed (remove `TAMA_TOKEN`/`OLLAMA_URL`/`OLLAMA_MODEL` if no longer needed, add any pi-related vars)
- [ ] Commit with message: "chore: format, clippy, and cleanup for pi RPC integration"

**Acceptance criteria:**
- [ ] `cargo fmt` applied cleanly
- [ ] `cargo clippy` has no warnings
- [ ] `cargo build --release` succeeds
- [ ] `cargo test` passes all tests
- [ ] No dead code or unused imports
- [ ] No references to removed Ollama/Exa env vars in code
