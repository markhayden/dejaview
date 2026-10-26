//! The fake MCP server (stdio, revision 2025-11-25) the tests drive: it serves `fake_regions` with
//! scripted envelopes and `probe_state`, which reports the calls since start or reset, its pid,
//! and which test variables reached its environment. Flags make it misbehave on purpose.

use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::time::Duration;

use serde_json::{Value, json};

struct Options {
    protocol_version: String,
    tools_capability: bool,
    reset: bool,
    hang_reset: bool,
    slow_call: Duration,
    ignore_stdin_close: bool,
    envelopes: VecDeque<Value>,
}

fn options() -> Options {
    let mut o = Options {
        protocol_version: "2025-11-25".into(),
        tools_capability: true,
        reset: true,
        hang_reset: false,
        slow_call: Duration::ZERO,
        ignore_stdin_close: false,
        envelopes: VecDeque::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--protocol-version" => o.protocol_version = args.next().unwrap_or_default(),
            "--no-tools-capability" => o.tools_capability = false,
            "--no-reset" => o.reset = false,
            "--hang-reset" => o.hang_reset = true,
            "--slow-call-ms" => {
                o.slow_call =
                    Duration::from_millis(args.next().and_then(|s| s.parse().ok()).unwrap_or(0));
            }
            "--ignore-stdin-close" => o.ignore_stdin_close = true,
            "--envelopes" => {
                let path = args.next().unwrap_or_default();
                let text = std::fs::read_to_string(&path).unwrap_or_else(|_| "[]".into());
                let list: Vec<Value> = serde_json::from_str(&text).unwrap_or_default();
                o.envelopes = list.into();
            }
            other => eprintln!("fake mcp: unknown flag {other}"),
        }
    }
    o
}

fn manifest(toml_text: &str) -> Value {
    let parsed: toml::Value = toml::from_str(toml_text).expect("manifest toml");
    serde_json::to_value(&parsed["tool"]).expect("manifest json")
}

/// A dejaview manifest as an MCP tool: native name, description, schemas; the rest annotated.
fn mcp_tool(manifest: &Value) -> Value {
    let mut annotation = manifest.clone();
    if let Some(obj) = annotation.as_object_mut() {
        obj.remove("name");
        obj.remove("input_schema");
        obj.remove("output_schema");
    }
    json!({
        "name": manifest["name"],
        "description": manifest["purpose"],
        "inputSchema": manifest["input_schema"],
        "outputSchema": manifest["output_schema"],
        "annotations": { "dejaview": annotation },
    })
}

fn provenance(tool: &str) -> Value {
    json!({ "tool": tool, "version": "0.0.1", "duration_ms": 1, "lock_digest": null })
}

fn invalid_args(tool: &str, message: &str) -> Value {
    json!({
        "status": "invalid_args", "summary": message, "measurements": [], "regions": [], "images": [], "transforms": [],
        "provenance": provenance(tool),
    })
}

fn main() {
    let mut o = options();
    let fake_regions = manifest(include_str!("../../fixtures/fake_regions.toml"));
    let probe_state = manifest(include_str!("../../fixtures/probe_state.toml"));
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut calls_since_reset: u64 = 0;
    let respond = |value: Value| {
        let mut out = stdout.lock();
        let _ = writeln!(out, "{value}");
        let _ = out.flush();
    };
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            eprintln!("fake mcp: malformed line");
            continue;
        };
        let method = msg["method"].as_str().unwrap_or("");
        let id = msg.get("id").cloned();
        match method {
            "initialize" => {
                let mut capabilities = json!({});
                if o.tools_capability {
                    capabilities["tools"] = json!({ "listChanged": false });
                }
                respond(json!({ "jsonrpc": "2.0", "id": id, "result": {
                    "protocolVersion": o.protocol_version,
                    "capabilities": capabilities,
                    "serverInfo": { "name": "dejaview-testkit-mcp", "version": "0.0.1" },
                } }));
            }
            "notifications/initialized" => {}
            "notifications/cancelled" => {
                eprintln!(
                    "fake mcp: cancelled requestId {}",
                    msg["params"]["requestId"]
                );
            }
            "tools/list" => {
                respond(
                    json!({ "jsonrpc": "2.0", "id": id, "result": { "tools": [mcp_tool(&fake_regions), mcp_tool(&probe_state)] } }),
                );
            }
            "tools/call" => {
                calls_since_reset += 1;
                let name = msg["params"]["name"].as_str().unwrap_or("");
                let args = &msg["params"]["arguments"];
                // Only the scripted tool is slow, so a probe can still observe the server.
                if name == "fake_regions" && !o.slow_call.is_zero() {
                    std::thread::sleep(o.slow_call);
                }
                let (envelope, is_error) = if args.get("_dejaview").is_none() {
                    (invalid_args(name, "_dejaview is required"), true)
                } else {
                    match name {
                        "fake_regions" => match o.envelopes.pop_front() {
                            Some(e) => (e, false),
                            None => (invalid_args(name, "script exhausted"), true),
                        },
                        "probe_state" => (
                            json!({
                                "status": "ok",
                                "summary": format!(
                                    "calls_since_reset={calls_since_reset} plain={} secret={}",
                                    u8::from(std::env::var_os("DEJAVIEW_TEST_PLAIN").is_some()),
                                    u8::from(std::env::var_os("DEJAVIEW_TEST_SECRET_KEY").is_some())
                                ),
                                "measurements": [
                                    { "name": "calls_since_reset", "value": calls_since_reset, "unit": "count", "frame": null },
                                    { "name": "pid", "value": std::process::id(), "unit": "count", "frame": null },
                                ],
                                "regions": [], "images": [], "transforms": [],
                                "provenance": provenance(name),
                            }),
                            false,
                        ),
                        _ => {
                            respond(
                                json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32602, "message": format!("unknown tool {name}") } }),
                            );
                            continue;
                        }
                    }
                };
                let content: Vec<Value> = envelope["images"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|img| json!({ "type": "image", "data": img["base64"].as_str().unwrap_or(""), "mimeType": img["media_type"] }))
                    .collect();
                respond(
                    json!({ "jsonrpc": "2.0", "id": id, "result": { "content": content, "structuredContent": envelope, "isError": is_error } }),
                );
            }
            "dejaview/reset" => {
                if o.hang_reset {
                    eprintln!("fake mcp: hanging on reset");
                    continue;
                }
                if !o.reset {
                    respond(
                        json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": "Method not found" } }),
                    );
                    continue;
                }
                calls_since_reset = 0;
                respond(json!({ "jsonrpc": "2.0", "id": id, "result": {} }));
            }
            _ => {
                if id.is_some() {
                    respond(
                        json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": "Method not found" } }),
                    );
                }
            }
        }
    }
    if o.ignore_stdin_close {
        eprintln!("fake mcp: ignoring stdin close");
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}
