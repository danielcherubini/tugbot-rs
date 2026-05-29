use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

static NEXT_REQ_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> String {
    format!("req-{}", NEXT_REQ_ID.fetch_add(1, Ordering::Relaxed))
}

const TIMEOUT_SECS: u64 = 300;
const PI_BINARY: &str = "pi";

pub struct PiRpc {
    inner: Mutex<PiRpcInner>,
}

struct PiRpcInner {
    child: Option<Child>,
    stdin: Option<BufWriter<tokio::process::ChildStdin>>,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl PiRpc {
    /// Spawn the `pi --mode rpc --no-session` subprocess and return an `Arc<Self>`.
    pub async fn spawn() -> Result<Arc<Self>> {
        let mut child = Command::new(PI_BINARY)
            .args(["--mode", "rpc", "--no-session"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn pi RPC subprocess")?;

        // Give pi a moment to initialize its RPC server
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let stdin = child.stdin.take().context("Failed to get pi stdin")?;
        let stdout = child.stdout.take().context("Failed to get pi stdout")?;

        let inner = PiRpcInner {
            child: Some(child),
            stdin: Some(BufWriter::new(stdin)),
            stdout: BufReader::new(stdout),
        };

        Ok(Arc::new(PiRpc {
            inner: Mutex::new(inner),
        }))
    }

    /// Send a prompt to the pi RPC subprocess and wait for the agent_end event.
    /// Returns the text of the last assistant message.
    pub async fn ask(&self, prompt: &str) -> Result<String> {
        self.ask_with_images(prompt, &[]).await
    }

    /// Send a prompt with optional base64-encoded images.
    /// Each image is a `(mime_type, base64_data)` tuple.
    pub async fn ask_with_images(
        &self,
        prompt: &str,
        images: &[(String, String)],
    ) -> Result<String> {
        let mut inner = self.inner.lock().await;

        // Restart if process is dead or previous ask crashed mid-stream
        if inner.child.is_none() || inner.stdin.is_none() {
            Self::restart_inner(&mut inner).await?;
        }

        // Note: Mutex is held for the entire ask() operation including the timeout.
        // This serializes concurrent requests, but for this bot's workload
        // (8h cooldown per user on is_this_real), concurrency is not a concern.
        // A channel-based design would be needed for true concurrent access.
        Self::do_ask(&mut inner, prompt, images).await
    }

    async fn do_ask(
        inner: &mut PiRpcInner,
        prompt: &str,
        images: &[(String, String)],
    ) -> Result<String> {
        let req_id = next_id();

        // Build the JSONL command
        let mut cmd = serde_json::Map::new();
        cmd.insert("id".into(), req_id.as_str().into());
        cmd.insert("type".into(), "prompt".into());
        cmd.insert("message".into(), prompt.into());
        if !images.is_empty() {
            let images_json: Vec<serde_json::Value> = images
                .iter()
                .map(|(mime, b64)| {
                    serde_json::json!({ "type": "image", "data": b64, "mimeType": mime })
                })
                .collect();
            cmd.insert("images".into(), images_json.into());
        }
        let command = serde_json::Value::Object(cmd);
        let command_str = serde_json::to_string(&command).context("Failed to serialize command")?;

        // Write to stdin
        let stdin = inner.stdin.as_mut().context("Stdin not available")?;
        stdin
            .write_all(format!("{}\n", command_str).as_bytes())
            .await
            .context("Failed to write to pi stdin")?;
        stdin.flush().await.context("Failed to flush pi stdin")?;

        // Read responses with timeout
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(TIMEOUT_SECS),
            Self::read_response(&mut inner.stdout, &req_id),
        )
        .await;

        match result {
            Ok(Ok(text)) => Ok(text),
            Ok(Err(e)) => {
                // Process died — mark as dead
                inner.child = None;
                inner.stdin = None;
                Err(e)
            }
            Err(_elapsed) => {
                // Timeout — mark as dead since we can't reliably recover mid-stream
                inner.child = None;
                inner.stdin = None;
                Err(anyhow::anyhow!(
                    "pi RPC ask timed out after {} seconds",
                    TIMEOUT_SECS
                ))
            }
        }
    }

    async fn read_response(
        stdout: &mut BufReader<tokio::process::ChildStdout>,
        req_id: &str,
    ) -> Result<String> {
        use tokio::io::AsyncBufReadExt;

        let mut prompt_accepted = false;
        let mut line = String::new();

        loop {
            line.clear();
            match stdout.read_line(&mut line).await {
                Ok(0) => {
                    // EOF — process died
                    return Err(anyhow::anyhow!("pi subprocess exited (EOF on stdout)"));
                }
                Ok(_) => {}
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to read from pi stdout: {}", e));
                }
            }

            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }

            let json: Value = match serde_json::from_str(line_trimmed) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!(
                        "[pi_rpc] Failed to parse JSON line: {} — {}",
                        e, line_trimmed
                    );
                    continue;
                }
            };

            // Check if this is a response (has "id" field)
            if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
                if id == req_id {
                    if let Some(success) = json.get("success").and_then(|v| v.as_bool()) {
                        if success {
                            prompt_accepted = true;
                            continue;
                        } else {
                            // Command rejected
                            let err_msg = json
                                .get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown error");
                            return Err(anyhow::anyhow!("pi RPC rejected prompt: {}", err_msg));
                        }
                    }
                }
                // Other responses with different IDs — skip
                continue;
            }

            // No "id" field — this is an event
            if let Some(event_type) = json.get("type").and_then(|v| v.as_str()) {
                if event_type == "agent_end" {
                    if !prompt_accepted {
                        return Err(anyhow::anyhow!(
                            "Received agent_end before prompt was accepted (req_id={})",
                            req_id
                        ));
                    }

                    // Check for error in agent_end
                    if let Some(error) = json.get("error") {
                        let err_msg = error
                            .as_str()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| error.to_string());
                        return Err(anyhow::anyhow!("pi RPC agent_end error: {}", err_msg));
                    }

                    // Extract text from the last assistant message
                    return extract_assistant_text(&json);
                }
            }
            // Other events (agent_start, turn_start, message_update, etc.) — skip
        }
    }

    /// Restart the pi subprocess. Must be called with inner lock held.
    async fn restart_inner(inner: &mut PiRpcInner) -> Result<()> {
        eprintln!("[pi_rpc] Restarting pi subprocess...");

        // Kill existing child if still running
        if let Some(mut child) = inner.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        inner.stdin = None;

        // Spawn new subprocess
        let mut new_child = Command::new(PI_BINARY)
            .args(["--mode", "rpc", "--no-session"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn new pi RPC subprocess during restart")?;

        // Give pi a moment to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let stdin = new_child
            .stdin
            .take()
            .context("Failed to get new pi stdin")?;
        let stdout = new_child
            .stdout
            .take()
            .context("Failed to get new pi stdout")?;

        inner.child = Some(new_child);
        inner.stdin = Some(BufWriter::new(stdin));
        inner.stdout = BufReader::new(stdout);

        eprintln!("[pi_rpc] pi subprocess restarted successfully");
        Ok(())
    }
}

/// Extract the text of the last assistant message from an agent_end event.
fn extract_assistant_text(json: &Value) -> Result<String> {
    let messages = json
        .get("messages")
        .and_then(|v| v.as_array())
        .context("agent_end missing 'messages' array")?;

    // Find the last message with role == "assistant"
    let assistant_msg = messages
        .iter()
        .rev()
        .find(|m| m.get("role").and_then(|v| v.as_str()) == Some("assistant"))
        .context("No assistant message found in agent_end")?;

    let content = assistant_msg
        .get("content")
        .context("Assistant message missing 'content'")?;

    // Content can be a string or an array of content blocks
    if let Some(text) = content.as_str() {
        return Ok(text.to_string());
    }

    if let Some(blocks) = content.as_array() {
        let mut text_parts = Vec::new();
        for block in blocks {
            if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                    text_parts.push(text);
                }
            }
        }
        return Ok(text_parts.join(""));
    }

    Err(anyhow::anyhow!(
        "Assistant content is neither a string nor an array: {}",
        content
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_assistant_text_string_content() {
        let json = serde_json::json!({
            "messages": [
                {"role": "user", "content": "Hello"},
                {"role": "assistant", "content": "Hi there!"}
            ]
        });
        let text = extract_assistant_text(&json).unwrap();
        assert_eq!(text, "Hi there!");
    }

    #[test]
    fn test_extract_assistant_text_array_content() {
        let json = serde_json::json!({
            "messages": [
                {"role": "user", "content": "Hello"},
                {"role": "assistant", "content": [
                    {"type": "text", "text": "Hi "},
                    {"type": "text", "text": "there!"}
                ]}
            ]
        });
        let text = extract_assistant_text(&json).unwrap();
        assert_eq!(text, "Hi there!");
    }

    #[test]
    fn test_extract_assistant_text_mixed_blocks() {
        let json = serde_json::json!({
            "messages": [
                {"role": "user", "content": "Hello"},
                {"role": "assistant", "content": [
                    {"type": "text", "text": "Answer: "},
                    {"type": "tool_use", "name": "search"},
                    {"type": "text", "text": "42"}
                ]}
            ]
        });
        let text = extract_assistant_text(&json).unwrap();
        assert_eq!(text, "Answer: 42");
    }

    #[test]
    fn test_extract_assistant_text_last_assistant() {
        let json = serde_json::json!({
            "messages": [
                {"role": "user", "content": "Hello"},
                {"role": "assistant", "content": "First response"},
                {"role": "user", "content": "Again?"},
                {"role": "assistant", "content": "Second response"}
            ]
        });
        let text = extract_assistant_text(&json).unwrap();
        assert_eq!(text, "Second response");
    }

    #[test]
    fn test_extract_assistant_text_no_assistant() {
        let json = serde_json::json!({
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });
        let result = extract_assistant_text(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_next_id_uniqueness() {
        let id1 = next_id();
        let id2 = next_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("req-"));
        assert!(id2.starts_with("req-"));
    }

    /// Smoke test — requires `pi` binary installed locally.
    #[tokio::test]
    #[ignore] // Requires pi binary
    async fn smoke_test_pi_rpc() {
        let pi_rpc = PiRpc::spawn().await.expect("Failed to spawn pi RPC");
        let result = pi_rpc.ask("Say hello in three words.").await;
        assert!(result.is_ok(), "pi RPC ask failed: {:?}", result.err());
        let text = result.unwrap();
        assert!(!text.is_empty(), "pi RPC returned empty response");
    }
}
