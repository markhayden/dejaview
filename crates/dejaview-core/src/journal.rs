//! The append-only run journal: one JSON object per line, fsynced per event, written from the
//! first event so a failure at any step leaves a durable record. Artifacts an event references are
//! written first (see `artifacts`).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::Result;

/// A run journal under `<run dir>/journal.jsonl`.
pub struct Journal {
    file: File,
    path: PathBuf,
    seq: u64,
    last_event: Option<String>,
    fail_at: Option<u64>,
}

#[derive(Serialize)]
struct Line<'a> {
    seq: u64,
    unix_ms: u128,
    event: &'a str,
    data: serde_json::Value,
}

impl Journal {
    /// Creates `journal.jsonl` in `run_dir` (which must exist). Fails if it already exists.
    pub fn create(run_dir: &Path) -> Result<Journal> {
        let path = run_dir.join("journal.jsonl");
        let file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&path)?;
        Ok(Journal {
            file,
            path,
            seq: 0,
            last_event: None,
            fail_at: None,
        })
    }

    /// Test knob: the append at this sequence number fails as if the process had died between an
    /// artifact write and its event.
    pub fn fail_at(&mut self, seq: u64) {
        self.fail_at = Some(seq);
    }

    /// The last event appended: the state a run reached.
    pub fn last_event(&self) -> Option<&str> {
        self.last_event.as_deref()
    }

    /// Appends one event and flushes it to disk.
    pub fn append(&mut self, event: &str, data: serde_json::Value) -> Result<()> {
        if self.fail_at == Some(self.seq) {
            return Err(std::io::Error::other(format!(
                "injected journal fault at seq {}",
                self.seq
            ))
            .into());
        }
        let unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let line = Line {
            seq: self.seq,
            unix_ms,
            event,
            data,
        };
        let mut text = serde_json::to_string(&line)?;
        text.push('\n');
        self.file.write_all(text.as_bytes())?;
        self.file.sync_data()?;
        self.seq += 1;
        self.last_event = Some(event.to_string());
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Number of events appended so far.
    pub fn len(&self) -> u64 {
        self.seq
    }

    pub fn is_empty(&self) -> bool {
        self.seq == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_numbered_lines_in_order() {
        let dir = std::env::temp_dir().join(format!("dejaview-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("tempdir");
        let mut j = Journal::create(&dir).expect("create");
        j.append("resolved", serde_json::json!({"images": 2}))
            .expect("append");
        j.append("verdict", serde_json::json!({"verdict": "pass"}))
            .expect("append");
        let text = std::fs::read_to_string(j.path()).expect("read");
        let lines: Vec<serde_json::Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).expect("json"))
            .collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["seq"], 0);
        assert_eq!(lines[0]["event"], "resolved");
        assert_eq!(lines[1]["seq"], 1);
        assert_eq!(lines[1]["data"]["verdict"], "pass");
        assert!(
            Journal::create(&dir).is_err(),
            "a second journal in the same run dir is refused"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
