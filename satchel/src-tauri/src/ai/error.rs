/// Errors from Claude calls, worded for the person using the app.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("Add your Claude API key in Settings first.")]
    MissingKey,
    #[error("Claude didn't accept the API key ({}).", .0.trim_end_matches('.'))]
    Auth(String),
    #[error("Claude's rate limit was reached. Wait a minute and try again.")]
    RateLimited,
    #[error("Claude is overloaded right now. Try again in a few minutes.")]
    Overloaded,
    #[error("Claude couldn't process the request: {}", sentence(.0))]
    InvalidRequest(String),
    #[error("Claude doesn't know the model “{0}”.")]
    ModelNotFound(String),
    #[error("Claude had a server error ({status}): {}", sentence(.message))]
    Server { status: u16, message: String },
    #[error("Couldn't reach Claude ({}).", .0.trim_end_matches('.'))]
    Network(String),
    #[error("Claude took too long to answer.")]
    Timeout,
    #[error("Claude declined this request{}.", refusal_suffix(.category, .explanation))]
    Refused { category: Option<String>, explanation: Option<String> },
    #[error("Claude's answer was cut off before it finished.")]
    Truncated,
    #[error("Claude's answer wasn't in the expected format: {0}")]
    BadOutput(String),
}

/// API messages often lack a final period; add one so a UI hint can follow cleanly.
fn sentence(s: &str) -> String {
    let s = s.trim();
    if s.ends_with(['.', '!', '?']) {
        s.to_string()
    } else {
        format!("{s}.")
    }
}

fn refusal_suffix(category: &Option<String>, explanation: &Option<String>) -> String {
    match (category, explanation) {
        (_, Some(e)) if !e.is_empty() => format!(": {e}"),
        (Some(c), _) => format!(" (category: {c})"),
        _ => String::new(),
    }
}

impl AiError {
    pub fn kind(&self) -> &'static str {
        match self {
            AiError::MissingKey => "missing_key",
            AiError::Auth(_) => "auth",
            AiError::RateLimited => "rate_limited",
            AiError::Overloaded => "overloaded",
            AiError::InvalidRequest(_) => "invalid_request",
            AiError::ModelNotFound(_) => "model_not_found",
            AiError::Server { .. } => "server",
            AiError::Network(_) => "network",
            AiError::Timeout => "timeout",
            AiError::Refused { .. } => "refused",
            AiError::Truncated => "truncated",
            AiError::BadOutput(_) => "bad_output",
        }
    }
}
