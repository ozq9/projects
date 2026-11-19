//! The one place that talks to Claude.
//!
//! Every call goes through [`ClaudeClient::send`], which adds auth headers, timeouts, retries with
//! backoff (honouring `retry-after` and `x-should-retry`), server-side refusal fallback where the
//! model supports it, and records token usage + estimated cost. Anything the app parses goes
//! through [`ClaudeClient::structured`]: JSON-schema constrained output, deserialised and
//! validated, with one automatic repair round-trip if the output doesn't validate.

mod error;
pub mod models;
pub mod types;

use std::sync::RwLock;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub use error::AiError;
use types::*;

use crate::db::usage::{self, UsageRecord};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::secrets::ApiKey;

pub const API_BASE: &str = "https://api.anthropic.com";
pub const API_VERSION: &str = "2026-10-21";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

/// Per-call settings. `purpose` labels the call in the usage counter.
#[derive(Debug, Clone)]
pub struct CallOptions {
    pub purpose: &'static str,
    pub item_id: Option<i64>,
    pub timeout: Duration,
    pub max_retries: u32,
}

impl CallOptions {
    pub fn new(purpose: &'static str) -> Self {
        Self { purpose, item_id: None, timeout: Duration::from_secs(120), max_retries: 3 }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn item(mut self, item_id: i64) -> Self {
        self.item_id = Some(item_id);
        self
    }
}

/// Semantic checks on parsed AI output, beyond what the JSON schema can express.
pub trait Validate {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Input for [`ClaudeClient::structured`].
pub struct StructuredCall<'a> {
    pub model: &'a str,
    pub system: &'a str,
    pub content: Vec<ContentBlock>,
    pub schema: &'a Value,
    pub effort: Option<Effort>,
    pub max_tokens: u32,
}

/// A validated structured result plus what it cost.
#[derive(Debug)]
pub struct Structured<T> {
    pub value: T,
    pub model: String,
    pub usage: Usage,
    pub cost_usd: Option<f64>,
    pub latency_ms: u64,
}

pub struct ClaudeClient {
    http: reqwest::Client,
    base_url: String,
    key: RwLock<Option<ApiKey>>,
    db: Db,
}

struct Failure {
    error: AiError,
    retryable: bool,
    retry_after: Option<Duration>,
}

impl Failure {
    fn from_reqwest(e: reqwest::Error) -> Self {
        let error = if e.is_timeout() { AiError::Timeout } else { AiError::Network(e.to_string()) };
        Failure { error, retryable: true, retry_after: None }
    }
}

impl ClaudeClient {
    pub fn new(db: Db, key: Option<ApiKey>) -> AppResult<Self> {
        Self::with_base_url(db, key, API_BASE)
    }

    pub fn with_base_url(db: Db, key: Option<ApiKey>, base_url: &str) -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .user_agent(concat!("Satchel/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| AppError::internal(format!("Couldn't create the HTTP client: {e}")))?;
        Ok(Self { http, base_url: base_url.trim_end_matches('/').to_string(), key: RwLock::new(key), db })
    }

    pub fn set_key(&self, key: Option<ApiKey>) {
        *self.key.write().unwrap_or_else(|p| p.into_inner()) = key;
    }

    pub fn key_hint(&self) -> Option<String> {
        self.key.read().unwrap_or_else(|p| p.into_inner()).as_ref().map(ApiKey::hint)
    }

    fn current_key(&self) -> Result<ApiKey, AiError> {
        self.key.read().unwrap_or_else(|p| p.into_inner()).clone().ok_or(AiError::MissingKey)
    }

    /// Sends one Messages API request (with retries). Returns `Refused` for refusals; any other
    /// stop reason (including `max_tokens`) is left for the caller to judge.
    pub async fn send(&self, mut req: MessagesRequest, opts: &CallOptions) -> Result<MessagesResponse, AiError> {
        let key = self.current_key()?;
        let caps = models::capabilities(&req.model);
        if caps.server_fallback && req.fallbacks.is_none() {
            // On a safety-classifier refusal, the API re-runs the request on a fallback model.
            req.fallbacks = Some(json!("default"));
            req.betas.push(FALLBACK_BETA);
        }
        if !caps.effort {
            req.output_config.effort = None;
        }

        let mut attempt = 0;
        loop {
            match self.attempt(&key, &req, opts).await {
                Ok(resp) => {
                    self.record_usage(&resp, opts).await;
                    if resp.stop_reason.as_deref() == Some("refusal") {
                        let details = resp.stop_details.clone();
                        return Err(AiError::Refused {
                            category: details.as_ref().and_then(|d| d.category.clone()),
                            explanation: details.and_then(|d| d.explanation),
                        });
                    }
                    return Ok(resp);
                }
                Err(failure) if failure.retryable && attempt < opts.max_retries => {
                    let delay = failure.retry_after.unwrap_or_else(|| backoff(attempt));
                    tracing::warn!(purpose = opts.purpose, attempt, ?delay, error = %failure.error, "Claude call failed; retrying");
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
                Err(failure) => return Err(failure.error),
            }
        }
    }

    async fn attempt(&self, key: &ApiKey, req: &MessagesRequest, opts: &CallOptions) -> Result<MessagesResponse, Failure> {
        let mut builder = self
            .http
            .post(format!("{}/v1/messages", self.base_url))
            .timeout(opts.timeout)
            .header("x-api-key", key.expose())
            .header("anthropic-version", API_VERSION)
            .json(req);
        if !req.betas.is_empty() {
            builder = builder.header("anthropic-beta", req.betas.join(","));
        }

        let started = Instant::now();
        let resp = builder.send().await.map_err(Failure::from_reqwest)?;
        let status = resp.status().as_u16();
        let header = |name: &str| resp.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_owned);
        let should_retry = header("x-should-retry").map(|v| v == "true");
        let retry_after = header("retry-after")
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|s| s.is_finite() && *s >= 0.0)
            .map(|s| Duration::from_secs_f64(s.min(60.0)));
        let body = resp.text().await.map_err(Failure::from_reqwest)?;

        if (200..300).contains(&status) {
            tracing::debug!(purpose = opts.purpose, ms = started.elapsed().as_millis() as u64, "Claude call ok");
            return serde_json::from_str(&body).map_err(|e| Failure {
                error: AiError::Server { status, message: format!("unreadable response ({e})") },
                retryable: false,
                retry_after: None,
            });
        }

        let message = serde_json::from_str::<ApiErrorBody>(&body)
            .map(|b| b.error.message)
            .unwrap_or_else(|_| body.chars().take(300).collect());
        let error = match status {
            401 | 403 => AiError::Auth(message),
            404 => AiError::ModelNotFound(req.model.clone()),
            408 => AiError::Timeout,
            429 => AiError::RateLimited,
            529 => AiError::Overloaded,
            400 | 413 | 422 => AiError::InvalidRequest(message),
            _ => AiError::Server { status, message },
        };
        let retryable_by_default = matches!(status, 408 | 409 | 429) || status >= 500;
        Err(Failure { error, retryable: should_retry.unwrap_or(retryable_by_default), retry_after })
    }

    async fn record_usage(&self, resp: &MessagesResponse, opts: &CallOptions) {
        let u = resp.usage;
        let model = resp.model.clone();
        let purpose = opts.purpose;
        let item_id = opts.item_id;
        let cache_write = u.cache_creation_input_tokens.unwrap_or(0);
        let cache_read = u.cache_read_input_tokens.unwrap_or(0);
        let cost = models::cost_usd(&model, u.input_tokens, u.output_tokens, cache_write, cache_read);
        let result = self
            .db
            .call(move |conn| {
                usage::record(
                    conn,
                    &UsageRecord {
                        purpose,
                        model: &model,
                        item_id,
                        input_tokens: u.input_tokens,
                        output_tokens: u.output_tokens,
                        cache_creation_tokens: cache_write,
                        cache_read_tokens: cache_read,
                        cost_usd: cost,
                    },
                )
            })
            .await;
        if let Err(e) = result {
            tracing::warn!(error = %e, "couldn't record AI usage");
        }
    }

    /// Asks for JSON matching `schema`, then deserialises and validates it into `T`.
    /// If the reply doesn't validate, Claude gets one chance to correct it.
    pub async fn structured<T>(&self, call: StructuredCall<'_>, opts: &CallOptions) -> Result<Structured<T>, AiError>
    where
        T: DeserializeOwned + Validate,
    {
        let caps = models::capabilities(call.model);
        let system = if caps.structured_outputs {
            call.system.to_string()
        } else {
            // Older models without constrained decoding: ask for the schema in the prompt.
            format!(
                "{}\n\nRespond with only a JSON object, no prose and no code fences, matching this JSON schema:\n{}",
                call.system, call.schema
            )
        };

        let started = Instant::now();
        let mut messages = vec![Message::user(call.content)];
        let mut total = Usage::default();
        let mut problem = String::from("no reply");

        for attempt in 0..2 {
            let mut req = MessagesRequest::new(call.model, call.max_tokens, messages.clone());
            req.system = Some(system.clone());
            req.output_config.effort = call.effort;
            if caps.structured_outputs {
                req.output_config.format = Some(OutputFormat::json_schema(call.schema.clone()));
            }

            let resp = self.send(req, opts).await?;
            total.add(&resp.usage);
            if resp.stop_reason.as_deref() == Some("max_tokens") {
                return Err(AiError::Truncated);
            }

            let text = resp.text();
            match parse_json::<T>(&text) {
                Ok(value) => {
                    let cost_usd = models::cost_usd(
                        &resp.model,
                        total.input_tokens,
                        total.output_tokens,
                        total.cache_creation_input_tokens.unwrap_or(0),
                        total.cache_read_input_tokens.unwrap_or(0),
                    );
                    return Ok(Structured {
                        value,
                        model: resp.model,
                        usage: total,
                        cost_usd,
                        latency_ms: started.elapsed().as_millis() as u64,
                    });
                }
                Err(p) => {
                    tracing::warn!(purpose = opts.purpose, attempt, problem = %p, "structured output rejected");
                    problem = p;
                    if !text.trim().is_empty() {
                        messages.push(Message::assistant_text(text));
                        messages.push(Message::user_text(format!(
                            "That reply can't be used: {problem}\nReply again with only the corrected JSON object."
                        )));
                    }
                }
            }
        }
        Err(AiError::BadOutput(problem))
    }

    /// Settings → "Test connection": a tiny structured call that exercises the whole pipeline.
    pub async fn test_connection(&self, model: &str) -> Result<ConnectionTest, AiError> {
        let schema = json!({
            "type": "object",
            "properties": {
                "ok": { "type": "boolean" },
                "greeting": { "type": "string", "description": "One short, friendly sentence." }
            },
            "required": ["ok", "greeting"],
            "additionalProperties": false
        });
        let result: Structured<Ping> = self
            .structured(
                StructuredCall {
                    model,
                    system: "You are the connection check of Satchel, a study app for students.",
                    content: vec![ContentBlock::text(
                        "Confirm the connection works: set ok to true and write a one-sentence greeting for a student who just set up their study app.",
                    )],
                    schema: &schema,
                    effort: Some(Effort::Low),
                    max_tokens: 2048,
                },
                &CallOptions::new("connection_test").timeout(Duration::from_secs(60)),
            )
            .await?;
        Ok(ConnectionTest {
            model_requested: model.to_string(),
            model_served: result.model,
            greeting: result.value.greeting,
            latency_ms: result.latency_ms,
            input_tokens: result.usage.input_tokens,
            output_tokens: result.usage.output_tokens,
            cost_usd: result.cost_usd,
        })
    }
}

#[derive(Debug, Deserialize)]
struct Ping {
    ok: bool,
    greeting: String,
}

impl Validate for Ping {
    fn validate(&self) -> Result<(), String> {
        if !self.ok {
            return Err("`ok` must be true".into());
        }
        if self.greeting.trim().is_empty() {
            return Err("`greeting` is empty".into());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTest {
    pub model_requested: String,
    pub model_served: String,
    pub greeting: String,
    pub latency_ms: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
}

/// Exponential backoff with jitter: ~0.5s, 1s, 2s, 4s … capped at 8s.
fn backoff(attempt: u32) -> Duration {
    let base = 0.5 * 2f64.powi(attempt.min(4) as i32);
    Duration::from_secs_f64(base.min(8.0) * (0.75 + 0.25 * fastrand::f64()))
}

/// Parses a JSON reply into `T` and runs its semantic validation.
pub fn parse_json<T: DeserializeOwned + Validate>(text: &str) -> Result<T, String> {
    let body = strip_code_fence(text.trim());
    if body.is_empty() {
        return Err("the reply was empty".into());
    }
    let value: T = serde_json::from_str(body).map_err(|e| format!("not valid JSON for this schema ({e})"))?;
    value.validate()?;
    Ok(value)
}

fn strip_code_fence(s: &str) -> &str {
    match s.strip_prefix("```") {
        Some(rest) => {
            let rest = rest.trim_start_matches(|c: char| c.is_ascii_alphanumeric());
            rest.strip_suffix("```").unwrap_or(rest).trim()
        }
        None => s,
    }
}

#[cfg(test)]
mod tests;
