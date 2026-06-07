use reqwest::{Response, StatusCode};
use std::fmt;

const MAX_BODY_LEN: usize = 500;

/// A non-2xx HTTP response from a provider, carrying both the status code
/// and a truncated body for diagnostics. Attached to `anyhow::Error` chains
/// so the orchestrator can classify it without changing the trait signature.
#[derive(Debug, Clone)]
pub struct HttpStatusError {
    pub status: StatusCode,
    pub body: String,
}

impl fmt::Display for HttpStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HTTP {}", self.status)?;
        if !self.body.is_empty() {
            let body = if self.body.len() > MAX_BODY_LEN {
                format!("{}...(truncated)", &self.body[..MAX_BODY_LEN])
            } else {
                self.body.clone()
            };
            write!(f, ": {}", body)?;
        }
        Ok(())
    }
}

impl std::error::Error for HttpStatusError {}

/// Wrap an HTTP status + body in an `anyhow::Error` so it can be returned
/// from provider code without changing the `AIProvider` trait signature.
pub fn http_error(status: StatusCode, body: String) -> anyhow::Error {
    anyhow::Error::new(HttpStatusError { status, body })
}

/// Inspect a `reqwest::Response` and return an `HttpStatusError`-bearing
/// `anyhow::Error` if the status is not 2xx. On success, returns the
/// response unchanged so the caller can continue consuming the body.
pub async fn check_status(response: Response) -> Result<Response, anyhow::Error> {
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(http_error(status, body));
    }
    Ok(response)
}

/// Whether a given error is worth retrying under QIR, or whether it should
/// be propagated immediately because retrying cannot succeed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    /// The error will not succeed if retried (e.g. 401). Bail out.
    Permanent,
    /// The error may succeed on a subsequent attempt (e.g. 429, 5xx, connect).
    /// Retry.
    Transient,
}

/// Map an HTTP status code to a retry class.
///
/// - 1xx/2xx/3xx: should not normally surface as errors. Treated as transient
///   to be safe (e.g. a 3xx that somehow became an error).
/// - 4xx: most are permanent (client errors that won't fix themselves).
///   Exceptions: 408 Request Timeout, 409 Conflict, 425 Too Early, 429
///   Too Many Requests — these can resolve on their own.
/// - 5xx: all transient (server-side, can clear up).
pub fn classify_status(status: StatusCode) -> RetryClass {
    match status.as_u16() {
        408 | 409 | 425 | 429 => RetryClass::Transient,
        400..=499 => RetryClass::Permanent,
        500..=599 => RetryClass::Transient,
        _ => RetryClass::Transient,
    }
}

/// Classify an `anyhow::Error` by walking its source chain, looking for:
///
/// 1. Our own `HttpStatusError` (preferred — providers attach the status).
/// 2. A raw `reqwest::Error` (carries a status for non-2xx responses, or a
///    transport flag for connect/timeout/etc).
///
/// If no recognized marker is found, defaults to `Permanent` — the safer
/// choice, since retrying a permanent error wastes user quota and may
/// escalate provider-side flags.
pub fn classify_error(err: &anyhow::Error) -> RetryClass {
    let mut current: Option<&dyn std::error::Error> = Some(err.as_ref());
    while let Some(e) = current {
        if let Some(http_err) = e.downcast_ref::<HttpStatusError>() {
            return classify_status(http_err.status);
        }
        if let Some(reqwest_err) = e.downcast_ref::<reqwest::Error>() {
            if let Some(status) = reqwest_err.status() {
                return classify_status(status);
            }
            if reqwest_err.is_connect()
                || reqwest_err.is_timeout()
                || reqwest_err.is_request()
            {
                return RetryClass::Transient;
            }
        }
        current = e.source();
    }
    RetryClass::Permanent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_status_4xx_permanent() {
        assert_eq!(classify_status(StatusCode::BAD_REQUEST), RetryClass::Permanent);
        assert_eq!(classify_status(StatusCode::UNAUTHORIZED), RetryClass::Permanent);
        assert_eq!(classify_status(StatusCode::FORBIDDEN), RetryClass::Permanent);
        assert_eq!(classify_status(StatusCode::NOT_FOUND), RetryClass::Permanent);
        assert_eq!(classify_status(StatusCode::UNPROCESSABLE_ENTITY), RetryClass::Permanent);
    }

    #[test]
    fn classify_status_4xx_transient() {
        assert_eq!(classify_status(StatusCode::REQUEST_TIMEOUT), RetryClass::Transient);
        assert_eq!(classify_status(StatusCode::CONFLICT), RetryClass::Transient);
        assert_eq!(classify_status(StatusCode::TOO_MANY_REQUESTS), RetryClass::Transient);
        assert_eq!(classify_status(StatusCode::from_u16(425).unwrap()), RetryClass::Transient);
    }

    #[test]
    fn classify_status_5xx_transient() {
        assert_eq!(classify_status(StatusCode::INTERNAL_SERVER_ERROR), RetryClass::Transient);
        assert_eq!(classify_status(StatusCode::BAD_GATEWAY), RetryClass::Transient);
        assert_eq!(classify_status(StatusCode::SERVICE_UNAVAILABLE), RetryClass::Transient);
        assert_eq!(classify_status(StatusCode::GATEWAY_TIMEOUT), RetryClass::Transient);
    }

    #[test]
    fn classify_error_with_http_status() {
        let err = http_error(StatusCode::UNAUTHORIZED, "bad key".to_string());
        assert_eq!(classify_error(&err), RetryClass::Permanent);

        let err = http_error(StatusCode::TOO_MANY_REQUESTS, "rate limited".to_string());
        assert_eq!(classify_error(&err), RetryClass::Transient);

        let err = http_error(StatusCode::INTERNAL_SERVER_ERROR, "oops".to_string());
        assert_eq!(classify_error(&err), RetryClass::Transient);
    }

    #[test]
    fn classify_error_unknown_is_permanent() {
        let err = anyhow::anyhow!("something weird");
        assert_eq!(classify_error(&err), RetryClass::Permanent);
    }

    #[test]
    fn classify_error_walks_source_chain() {
        let err = http_error(StatusCode::FORBIDDEN, "x".to_string())
            .context("calling anthropic");
        assert_eq!(classify_error(&err), RetryClass::Permanent);
    }

    #[test]
    fn http_error_display_truncates_long_bodies() {
        let big = "x".repeat(2000);
        let err = http_error(StatusCode::BAD_REQUEST, big);
        let s = format!("{}", err);
        assert!(s.contains("HTTP 400"));
        assert!(s.contains("truncated"));
    }
}
