//! The model's structured submission (`submit_verdict`) and its validation. Task 1.1 ships the
//! canonical types and the two checks the walking slice needs (every checklist item exactly once;
//! a violation carries a location). The JSON Schema generation and the full semantic rules arrive
//! in task 1.6a.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::aggregate::{Confidence, Disposition};
use crate::checklist::ChecklistItem;
use crate::provider::ToolDefinition;
use crate::{Error, Result};

/// The canonical submission, schema version 1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Submission {
    pub schema_version: u32,
    pub items: Vec<ItemDisposition>,
    pub findings: Vec<Finding>,
    pub opinion: Opinion,
}

/// What the model said about one checklist item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemDisposition {
    pub id: String,
    pub disposition: Disposition,
    pub confidence: Confidence,
    pub reason: String,
    pub allowed_by: Option<String>,
    pub location: Option<Location>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

/// Something the model noticed beyond the checklist.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub obligation_ids: Vec<String>,
    pub disposition: Disposition,
    pub confidence: Confidence,
    pub reason: String,
    pub allowed_by: Option<String>,
    pub location: Location,
    #[serde(default)]
    pub evidence: Vec<String>,
}

/// Recorded, never read by aggregation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Opinion {
    pub verdict: String,
    pub reason: String,
}

pub use crate::frames::{Location, LocationKind};

/// The `submit_verdict` tool as offered to the provider. Hand-written for the walking slice;
/// generated from the types with `schemars` in task 1.6a.
pub fn submit_verdict_tool() -> ToolDefinition {
    ToolDefinition {
        name: "submit_verdict".into(),
        description:
            "Submit the final structured verdict: one disposition per checklist item, any \
                      further findings, and your own opinion (recorded only)."
                .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["schema_version", "items", "findings", "opinion"],
            "properties": {
                "schema_version": { "const": 1 },
                "items": { "type": "array" },
                "findings": { "type": "array" },
                "opinion": { "type": "object" }
            }
        }),
    }
}

/// Parses and validates a submission against the current checklist.
pub fn parse_and_validate(
    arguments: &serde_json::Value,
    checklist: &[ChecklistItem],
) -> Result<Submission> {
    let sub: Submission = serde_json::from_value(arguments.clone())
        .map_err(|e| Error::Submission(format!("does not match the submission schema: {e}")))?;
    if sub.schema_version != 1 {
        return Err(Error::Submission(format!(
            "schema_version must be 1, got {}",
            sub.schema_version
        )));
    }
    let mut counts: HashMap<&str, usize> = checklist.iter().map(|c| (c.id.as_str(), 0)).collect();
    for item in &sub.items {
        match counts.get_mut(item.id.as_str()) {
            Some(n) => *n += 1,
            None => {
                return Err(Error::Submission(format!(
                    "unknown checklist item {:?}",
                    item.id
                )));
            }
        }
        if item.disposition == Disposition::Violation && item.location.is_none() {
            return Err(Error::Submission(format!(
                "{}: a violation needs a location",
                item.id
            )));
        }
    }
    for (id, n) in counts {
        if n != 1 {
            return Err(Error::Submission(format!(
                "checklist item {id:?} must appear exactly once, appeared {n} times"
            )));
        }
    }
    Ok(sub)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checklist() -> Vec<ChecklistItem> {
        vec![ChecklistItem {
            id: "chk:ob:SF2@candidate".into(),
            instance_ids: vec!["SF2@candidate".into()],
        }]
    }

    fn held() -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1,
            "items": [{ "id": "chk:ob:SF2@candidate", "disposition": "held", "confidence": "confident",
                        "reason": "same scene", "allowed_by": null, "location": null, "evidence": [] }],
            "findings": [],
            "opinion": { "verdict": "pass", "reason": "x" }
        })
    }

    #[test]
    fn a_complete_submission_validates() {
        assert!(parse_and_validate(&held(), &checklist()).is_ok());
    }

    #[test]
    fn a_missing_item_is_rejected() {
        let mut v = held();
        v["items"] = serde_json::json!([]);
        assert!(
            matches!(parse_and_validate(&v, &checklist()), Err(Error::Submission(m)) if m.contains("exactly once"))
        );
    }

    #[test]
    fn an_unknown_item_is_rejected() {
        let mut v = held();
        v["items"][0]["id"] = serde_json::json!("chk:ob:XX9@candidate");
        assert!(
            matches!(parse_and_validate(&v, &checklist()), Err(Error::Submission(m)) if m.contains("unknown"))
        );
    }

    #[test]
    fn a_violation_without_location_is_rejected() {
        let mut v = held();
        v["items"][0]["disposition"] = serde_json::json!("violation");
        assert!(
            matches!(parse_and_validate(&v, &checklist()), Err(Error::Submission(m)) if m.contains("location"))
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut v = held();
        v["extra"] = serde_json::json!(true);
        assert!(parse_and_validate(&v, &checklist()).is_err());
    }
}
