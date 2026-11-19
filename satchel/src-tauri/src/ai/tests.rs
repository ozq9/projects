//! Client tests against a tiny local HTTP server that replays canned responses.

use std::sync::{Arc, Mutex};

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;

struct Canned {
    status: u16,
    headers: Vec<(&'static str, &'static str)>,
    body: String,
}

fn ok_message(model: &str, text: &str, stop_reason: &str) -> Canned {
    Canned {
        status: 200,
        headers: vec![],
        body: json!({
            "id": "msg_test", "type": "message", "role": "assistant", "model": model,
            "content": [{"type": "text", "text": text}],
            "stop_reason": stop_reason, "stop_details": null,
            "usage": {"input_tokens": 100, "output_tokens": 20}
        })
        .to_string(),
    }
}

fn api_error(status: u16, kind: &str, message: &str, headers: Vec<(&'static str, &'static str)>) -> Canned {
    Canned { status, headers, body: json!({"type": "error", "error": {"type": kind, "message": message}}).to_string() }
}

/// Serves `responses` in order, one per connection; returns the base URL and captured requests.
async fn serve(responses: Vec<Canned>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_clone = seen.clone();
    tokio::spawn(async move {
        for canned in responses {
            let (mut sock, _) = listener.accept().await.unwrap();
            let request = read_request(&mut sock).await;
            seen_clone.lock().unwrap().push(request);
            let mut out = format!(
                "HTTP/1.1 {} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n",
                canned.status,
                canned.body.len()
            );
            for (k, v) in canned.headers {
                out.push_str(&format!("{k}: {v}\r\n"));
            }
            out.push_str("\r\n");
            out.push_str(&canned.body);
            sock.write_all(out.as_bytes()).await.unwrap();
            sock.shutdown().await.ok();
        }
    });
    (format!("http://{addr}"), seen)
}

async fn read_request(sock: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = sock.read(&mut chunk).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        let text = String::from_utf8_lossy(&buf);
        if let Some(header_end) = text.find("\r\n\r\n") {
            let content_length = text[..header_end]
                .lines()
                .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap()))
                .unwrap_or(0);
            if buf.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}

fn client(base: &str) -> (ClaudeClient, Db) {
    let db = Db::open_in_memory().unwrap();
    let key = ApiKey::parse("sk-ant-test-0000000000000000000000").unwrap();
    (ClaudeClient::with_base_url(db.clone(), Some(key), base).unwrap(), db)
}

async fn usage_calls(db: &Db) -> i64 {
    db.call(|c| Ok(c.query_row("SELECT COUNT(*) FROM ai_usage", [], |r| r.get::<_, i64>(0))?)).await.unwrap()
}

#[tokio::test]
async fn connection_test_retries_overload_then_succeeds() {
    let (base, seen) = serve(vec![
        api_error(529, "overloaded_error", "Overloaded", vec![("retry-after", "0")]),
        ok_message("claude-opus-5-5", r#"{"ok": true, "greeting": "Welcome to Satchel!"}"#, "end_turn"),
    ])
    .await;
    let (claude, db) = client(&base);

    let result = claude.test_connection("claude-opus-5-5").await.unwrap();
    assert_eq!(result.greeting, "Welcome to Satchel!");
    assert_eq!(result.model_served, "claude-opus-5-5");
    assert!(result.cost_usd.unwrap() > 0.0);

    let requests = seen.lock().unwrap().clone();
    assert_eq!(requests.len(), 2);
    let req = requests[1].to_ascii_lowercase();
    assert!(req.contains("x-api-key: sk-ant-test-"));
    assert!(req.contains("anthropic-version: 2026-10-21"));
    assert!(req.contains("anthropic-beta: server-side-fallback-2026-07-01"));
    assert!(req.contains(r#""fallbacks":"default""#));
    assert!(req.contains(r#""format":{"type":"json_schema""#));
    assert!(req.contains(r#""effort":"low""#));
    // Only the successful attempt produced a billable message.
    assert_eq!(usage_calls(&db).await, 1);
}

#[tokio::test]
async fn invalid_output_gets_one_repair_round_trip() {
    let (base, seen) = serve(vec![
        ok_message("claude-opus-5-5", r#"{"ok": false, "greeting": ""}"#, "end_turn"),
        ok_message("claude-opus-5-5", r#"{"ok": true, "greeting": "Hi!"}"#, "end_turn"),
    ])
    .await;
    let (claude, db) = client(&base);

    let result = claude.test_connection("claude-opus-5-5").await.unwrap();
    assert_eq!(result.greeting, "Hi!");
    assert_eq!(result.input_tokens, 200); // usage summed across both calls
    assert!(seen.lock().unwrap()[1].contains("can't be used"));
    assert_eq!(usage_calls(&db).await, 2);
}

#[tokio::test]
async fn bad_output_twice_is_a_clean_error() {
    let (base, _) = serve(vec![
        ok_message("claude-opus-5-5", "not json", "end_turn"),
        ok_message("claude-opus-5-5", "still not json", "end_turn"),
    ])
    .await;
    let (claude, _) = client(&base);
    let err = claude.test_connection("claude-opus-5-5").await.unwrap_err();
    assert_eq!(err.kind(), "bad_output");
}

#[tokio::test]
async fn auth_errors_are_not_retried() {
    let (base, seen) = serve(vec![api_error(401, "authentication_error", "invalid x-api-key", vec![])]).await;
    let (claude, _) = client(&base);
    let err = claude.test_connection("claude-opus-5-5").await.unwrap_err();
    assert_eq!(err.kind(), "auth");
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn refusal_and_truncation_are_reported() {
    let (base, _) = serve(vec![
        Canned {
            status: 200,
            headers: vec![],
            body: json!({
                "id": "msg_r", "type": "message", "role": "assistant", "model": "claude-opus-5-5",
                "content": [], "stop_reason": "refusal",
                "stop_details": {"type": "refusal", "category": "cyber", "explanation": null},
                "usage": {"input_tokens": 10, "output_tokens": 0}
            })
            .to_string(),
        },
        ok_message("claude-opus-5-5", r#"{"ok": tr"#, "max_tokens"),
    ])
    .await;
    let (claude, _) = client(&base);
    assert_eq!(claude.test_connection("claude-opus-5-5").await.unwrap_err().kind(), "refused");
    assert_eq!(claude.test_connection("claude-opus-5-5").await.unwrap_err().kind(), "truncated");
}

#[tokio::test]
async fn older_models_get_schema_in_prompt_and_no_fallback() {
    let (base, seen) =
        serve(vec![ok_message("claude-sonnet-4-6", "```json\n{\"ok\": true, \"greeting\": \"Hey\"}\n```", "end_turn")]).await;
    let (claude, _) = client(&base);
    let result = claude.test_connection("claude-sonnet-4-6").await.unwrap();
    assert_eq!(result.greeting, "Hey");
    let req = &seen.lock().unwrap()[0];
    assert!(!req.contains("fallbacks"));
    assert!(!req.contains("json_schema"));
    assert!(req.contains("matching this JSON schema"));
}

#[tokio::test]
async fn missing_key_fails_fast() {
    let db = Db::open_in_memory().unwrap();
    let claude = ClaudeClient::with_base_url(db, None, "http://127.0.0.1:9").unwrap();
    assert_eq!(claude.test_connection("claude-opus-5-5").await.unwrap_err().kind(), "missing_key");
}

#[test]
fn code_fences_are_tolerated() {
    #[derive(serde::Deserialize)]
    struct V {
        a: i32,
    }
    impl Validate for V {}
    assert_eq!(parse_json::<V>("```json\n{\"a\": 1}\n```").unwrap().a, 1);
    assert_eq!(parse_json::<V>("{\"a\": 2}").unwrap().a, 2);
    assert!(parse_json::<V>("").is_err());
}
