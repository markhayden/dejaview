//! The HTTPS client every adapter sends through: rustls with the ring provider and the platform's
//! trust roots, a request timeout, and a redirect policy that follows same-origin redirects only;
//! a redirect to another host, scheme or port is refused as `error: provider`. Failures become
//! `Error::Transport` with the status and `retry-after`, so the run loop decides about retries;
//! messages never carry the URL or headers.

use std::time::Duration;

use dejaview_core::retry::TransportError;
use dejaview_core::{Error, Result};

/// Same-origin redirects followed before giving up.
pub const MAX_REDIRECTS: usize = 5;
/// Body bytes kept in a transport error message.
const ERROR_BODY_BYTES: usize = 2048;

#[derive(Debug, Clone)]
pub struct Client {
    inner: reqwest::Client,
}

/// A successful (2xx) response.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub body: String,
    pub request_id: Option<String>,
}

impl Client {
    pub fn new(timeout: Duration) -> Result<Client> {
        // The provider is process-wide; a second installation (tests, embedders) is harmless.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let inner = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::custom(same_origin))
            .user_agent(concat!("dejaview/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| Error::Provider(format!("cannot build the HTTP client: {e}")))?;
        Ok(Client { inner })
    }

    /// POST a JSON body with the given headers. 2xx is `Ok`; anything else is a transport error
    /// carrying the status, the server's `retry-after` and the start of the body.
    pub async fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &serde_json::Value,
    ) -> Result<Response> {
        let mut req = self.inner.post(url).json(body);
        for (name, value) in headers {
            req = req.header(*name, *value);
        }
        let response = req.send().await.map_err(map_send_error)?;
        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(Duration::from_secs);
        let request_id = ["request-id", "x-request-id"]
            .iter()
            .find_map(|h| response.headers().get(*h))
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let body = response.text().await.map_err(|e| {
            Error::Transport(TransportError {
                status: Some(status),
                retry_after,
                message: format!("reading the response body: {}", strip_url(e)),
            })
        })?;
        if (200..300).contains(&status) {
            return Ok(Response {
                status,
                body,
                request_id,
            });
        }
        let excerpt: String = body.chars().take(ERROR_BODY_BYTES).collect();
        Err(Error::Transport(TransportError {
            status: Some(status),
            retry_after,
            message: format!("HTTP {status}: {excerpt}"),
        }))
    }
}

/// Follow a redirect only to the same scheme, host and port, at most `MAX_REDIRECTS` times.
fn same_origin(attempt: reqwest::redirect::Attempt) -> reqwest::redirect::Action {
    let Some(first) = attempt.previous().first() else {
        return attempt.stop();
    };
    let next = attempt.url();
    let same = next.scheme() == first.scheme()
        && next.host_str() == first.host_str()
        && next.port_or_known_default() == first.port_or_known_default();
    if !same {
        let host = next.host_str().unwrap_or("?").to_string();
        return attempt.error(format!("redirect to another host refused: {host}"));
    }
    if attempt.previous().len() > MAX_REDIRECTS {
        return attempt.error("too many redirects");
    }
    attempt.follow()
}

fn map_send_error(e: reqwest::Error) -> Error {
    let status = e.status().map(|s| s.as_u16());
    let (redirect, timeout, connect) = (e.is_redirect(), e.is_timeout(), e.is_connect());
    let detail = strip_url(e);
    if redirect {
        return Error::Provider(format!("redirect refused: {detail}"));
    }
    let message = if timeout {
        "timed out".to_string()
    } else if connect {
        format!("connection failed: {detail}")
    } else {
        detail
    };
    Error::Transport(TransportError {
        status,
        retry_after: None,
        message,
    })
}

/// reqwest's messages name the URL, which may carry a key in its query for some providers; the
/// chain of sources without the URL is enough.
fn strip_url(e: reqwest::Error) -> String {
    let e = e.without_url();
    let mut parts = vec![e.to_string()];
    let mut source: Option<&(dyn std::error::Error + 'static)> = std::error::Error::source(&e);
    while let Some(s) = source {
        parts.push(s.to_string());
        source = s.source();
    }
    parts.join(": ")
}
