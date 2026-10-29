//! The fixture replay harness shared by every adapter (SPEC "Adapter conformance"). A recorded
//! conversation is a directory of exchanges, `01.json`, `02.json`, …, each holding the request as
//! sent (URL, headers, JSON body) and the response as received (status, headers, JSON body), with
//! secrets stripped. `Player` serves the recorded responses in order and fails the moment a request
//! differs from the recording anywhere except the fields an adapter declares volatile (auth
//! headers, request ids); opaque payloads such as thinking signatures are compared byte for byte,
//! so a dropped or altered block fails the replay. `Recorder` wraps a live transport and writes the
//! same files.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use squint_core::{Error, Result};

use crate::http::{Response, Transport, TransportFuture};

/// What a recording leaves out of the comparison: header names (any case) and JSON pointers into
/// the request body whose values change between otherwise identical requests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Volatile {
    pub headers: Vec<String>,
    pub body_pointers: Vec<String>,
}

impl Volatile {
    /// Credentials and request-tracking headers every adapter treats as volatile.
    pub fn common() -> Volatile {
        Volatile {
            headers: [
                "x-api-key",
                "authorization",
                "x-goog-api-key",
                "request-id",
                "x-request-id",
                "user-agent",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            body_pointers: Vec::new(),
        }
    }

    fn is_volatile_header(&self, name: &str) -> bool {
        self.headers.iter().any(|h| h.eq_ignore_ascii_case(name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedRequest {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
}

/// One round trip as recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exchange {
    pub request: RecordedRequest,
    pub response: RecordedResponse,
}

pub const REDACTED: &str = "<redacted>";

/// Loads the exchanges of one recorded conversation, in file order.
pub fn load(dir: &Path) -> Result<Vec<Exchange>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| Error::Provider(format!("replay: cannot read {}: {e}", dir.display())))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        // Exchanges are numbered files; anything else beside them (`expected.json`) is not one.
        .filter(|p| {
            p.extension().is_some_and(|x| x == "json")
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        })
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(Error::Provider(format!(
            "replay: {} holds no exchanges",
            dir.display()
        )));
    }
    files
        .iter()
        .map(|p| {
            let text = std::fs::read_to_string(p)?;
            serde_json::from_str(&text)
                .map_err(|e| Error::Provider(format!("replay: {}: {e}", p.display())))
        })
        .collect()
}

/// Serves a recorded conversation, checking every request against the recording.
pub struct Player {
    exchanges: Vec<Exchange>,
    volatile: Volatile,
    next: Mutex<usize>,
}

impl Player {
    pub fn new(exchanges: Vec<Exchange>, volatile: Volatile) -> Player {
        Player {
            exchanges,
            volatile,
            next: Mutex::new(0),
        }
    }

    pub fn load(dir: &Path, volatile: Volatile) -> Result<Player> {
        Ok(Player::new(load(dir)?, volatile))
    }

    /// Exchanges not yet served.
    pub fn remaining(&self) -> usize {
        self.exchanges.len() - self.next.lock().map(|n| *n).unwrap_or(0)
    }

    fn check(
        &self,
        n: usize,
        recorded: &RecordedRequest,
        url: &str,
        headers: &[(String, String)],
        body: &Value,
    ) -> Result<()> {
        let fail = |what: String| Error::Provider(format!("replay: exchange {}: {what}", n + 1));
        if recorded.url != url {
            return Err(fail(format!(
                "url differs: recorded {:?}, sent {:?}",
                recorded.url, url
            )));
        }
        let sent: BTreeMap<String, String> = headers
            .iter()
            .filter(|(k, _)| !self.volatile.is_volatile_header(k))
            .map(|(k, v)| (k.to_ascii_lowercase(), v.clone()))
            .collect();
        let want: BTreeMap<String, String> = recorded
            .headers
            .iter()
            .filter(|(k, _)| !self.volatile.is_volatile_header(k))
            .map(|(k, v)| (k.to_ascii_lowercase(), v.clone()))
            .collect();
        if sent != want {
            let names: Vec<&String> = want
                .keys()
                .chain(sent.keys())
                .filter(|k| want.get(*k) != sent.get(*k))
                .collect();
            return Err(fail(format!("headers differ: {names:?}")));
        }
        let mut a = recorded.body.clone();
        let mut b = body.clone();
        for p in &self.volatile.body_pointers {
            remove_pointer(&mut a, p);
            remove_pointer(&mut b, p);
        }
        if let Some(d) = first_difference(&a, &b, "") {
            return Err(fail(format!("request body differs at {d}")));
        }
        Ok(())
    }
}

impl Transport for Player {
    fn post_json<'a>(
        &'a self,
        url: &'a str,
        headers: &'a [(String, String)],
        body: &'a Value,
    ) -> TransportFuture<'a> {
        Box::pin(async move {
            let n = {
                let mut next = self
                    .next
                    .lock()
                    .map_err(|_| Error::Provider("replay: poisoned".into()))?;
                let n = *next;
                *next += 1;
                n
            };
            let Some(exchange) = self.exchanges.get(n) else {
                return Err(Error::Provider(format!(
                    "replay: request {} but the recording holds {} exchanges",
                    n + 1,
                    self.exchanges.len()
                )));
            };
            self.check(n, &exchange.request, url, headers, body)?;
            Ok(Response {
                status: exchange.response.status,
                headers: exchange.response.headers.clone(),
                body: exchange.response.body.to_string(),
            })
        })
    }
}

/// The first JSON pointer at which two values differ, with a short account of the difference.
pub fn first_difference(a: &Value, b: &Value, path: &str) -> Option<String> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
                let p = format!("{path}/{}", k.replace('~', "~0").replace('/', "~1"));
                match (x.get(k), y.get(k)) {
                    (Some(_), None) => return Some(format!("{p}: recorded has it, sent lacks it")),
                    (None, Some(_)) => return Some(format!("{p}: sent has it, recorded lacks it")),
                    (Some(va), Some(vb)) => {
                        if let Some(d) = first_difference(va, vb, &p) {
                            return Some(d);
                        }
                    }
                    (None, None) => {}
                }
            }
            None
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some(format!(
                    "{}: recorded {} items, sent {}",
                    if path.is_empty() { "/" } else { path },
                    x.len(),
                    y.len()
                ));
            }
            x.iter()
                .zip(y)
                .enumerate()
                .find_map(|(i, (va, vb))| first_difference(va, vb, &format!("{path}/{i}")))
        }
        _ if a == b => None,
        _ => Some(format!(
            "{}: recorded {}, sent {}",
            if path.is_empty() { "/" } else { path },
            short(a),
            short(b)
        )),
    }
}

fn short(v: &Value) -> String {
    let s = v.to_string();
    if s.chars().count() > 80 {
        format!("{}…", s.chars().take(80).collect::<String>())
    } else {
        s
    }
}

/// Response headers a recording keeps (lowercase): the request ids the adapters report, the retry
/// hint the run loop reads, and the content type.
pub const KEPT_RESPONSE_HEADERS: &[&str] =
    &["content-type", "request-id", "x-request-id", "retry-after"];

fn remove_pointer(v: &mut Value, pointer: &str) {
    let Some((parent, key)) = pointer.rsplit_once('/') else {
        return;
    };
    let parent = if parent.is_empty() {
        Some(v)
    } else {
        v.pointer_mut(parent)
    };
    if let Some(Value::Object(o)) = parent {
        o.remove(&key.replace("~1", "/").replace("~0", "~"));
    }
}

/// Records every exchange a live transport makes, with secrets stripped: volatile header values
/// become `<redacted>` and every secret string is replaced wherever it appears.
pub struct Recorder {
    inner: Arc<dyn Transport>,
    dir: PathBuf,
    secrets: Vec<String>,
    volatile: Volatile,
    count: Mutex<usize>,
}

impl Recorder {
    pub fn new(
        inner: Arc<dyn Transport>,
        dir: &Path,
        secrets: Vec<String>,
        volatile: Volatile,
    ) -> Result<Recorder> {
        std::fs::create_dir_all(dir)?;
        Ok(Recorder {
            inner,
            dir: dir.to_path_buf(),
            secrets,
            volatile,
            count: Mutex::new(0),
        })
    }

    fn scrub(&self, text: &str) -> String {
        let mut out = text.to_string();
        for s in &self.secrets {
            if !s.is_empty() {
                out = out.replace(s, REDACTED);
            }
        }
        out
    }

    fn scrub_value(&self, v: &Value) -> Value {
        serde_json::from_str(&self.scrub(&v.to_string())).unwrap_or(Value::Null)
    }

    fn record(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &Value,
        response: &Response,
    ) -> Result<()> {
        let n = {
            let mut c = self
                .count
                .lock()
                .map_err(|_| Error::Provider("record: poisoned".into()))?;
            *c += 1;
            *c
        };
        let request_headers: BTreeMap<String, String> = headers
            .iter()
            .map(|(k, v)| {
                let value = if self.volatile.is_volatile_header(k) {
                    REDACTED.to_string()
                } else {
                    self.scrub(v)
                };
                (k.to_ascii_lowercase(), value)
            })
            .collect();
        // Only the response headers the adapters read are kept: the rest carry rate-limit state,
        // organisation and project identifiers, cookies and server details that are not part of
        // the conversation and must not be committed with it.
        let response_headers: BTreeMap<String, String> = response
            .headers
            .iter()
            .filter(|(k, _)| KEPT_RESPONSE_HEADERS.contains(&k.to_ascii_lowercase().as_str()))
            .map(|(k, v)| (k.to_ascii_lowercase(), self.scrub(v)))
            .collect();
        let response_body: Value =
            serde_json::from_str(&response.body).unwrap_or(Value::String(response.body.clone()));
        let exchange = Exchange {
            request: RecordedRequest {
                url: self.scrub(url),
                headers: request_headers,
                body: self.scrub_value(body),
            },
            response: RecordedResponse {
                status: response.status,
                headers: response_headers,
                body: self.scrub_value(&response_body),
            },
        };
        let path = self.dir.join(format!("{n:02}.json"));
        let text = serde_json::to_string_pretty(&exchange)? + "\n";
        squint_core::artifacts::write_private(&path, text.as_bytes())
    }
}

impl Transport for Recorder {
    fn post_json<'a>(
        &'a self,
        url: &'a str,
        headers: &'a [(String, String)],
        body: &'a Value,
    ) -> TransportFuture<'a> {
        Box::pin(async move {
            let response = self.inner.post_json(url, headers, body).await?;
            self.record(url, headers, body, &response)?;
            Ok(response)
        })
    }
}
