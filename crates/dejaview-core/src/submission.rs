//! The model's structured submission (`submit_verdict`): the canonical schema generated from these
//! types, JSON Schema validation, and the semantic rules that tie a submission to this run's
//! checklist, instances, exemptions, frames and tool calls (SPEC "Submission schema").

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::aggregate::{Confidence, Disposition};
use crate::checklist::Checklist;
use crate::frames::{Location, LocationKind, Registry};
use crate::obligations::Resolved;
use crate::provider::ToolDefinition;
use crate::{Error, Result};

pub const SUBMISSION_SCHEMA_VERSION: u32 = 1;

/// The canonical submission, schema version 1. Every property is required; optional values are
/// nullable, so the shape survives every provider's strict mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Submission {
    pub schema_version: u32,
    pub items: Vec<ItemDisposition>,
    pub findings: Vec<Finding>,
    pub opinion: Opinion,
}

/// What the model said about one checklist item. Items carry no obligation ids: the checklist
/// owns them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ItemDisposition {
    /// The checklist item id exactly as listed, for example `chk:ob:SF1@candidate`.
    pub id: String,
    pub disposition: Disposition,
    pub confidence: Confidence,
    /// One to a few sentences; at most 600 characters.
    pub reason: String,
    /// Exemption id; required when `disposition = allowed_change`, null otherwise.
    pub allowed_by: Option<String>,
    pub location: Option<Location>,
    /// Tool call ids from this run used as evidence.
    pub evidence: Vec<String>,
}

/// Something the model noticed beyond the checklist: a violation or allowed change not covered by
/// an item, or an observation under an `observe` rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    /// One to eight rule ids as numbered under Obligations, for example `SF4`; never checklist item ids.
    pub obligation_ids: Vec<String>,
    pub disposition: FindingDisposition,
    pub confidence: Confidence,
    /// One to a few sentences; at most 600 characters.
    pub reason: String,
    /// Exemption id when `disposition = allowed_change`, null otherwise.
    pub allowed_by: Option<String>,
    /// Where it is; findings always have a location.
    pub location: Location,
    /// Tool call ids from this run used as evidence.
    pub evidence: Vec<String>,
}

/// Dispositions a finding may carry. `held` and `hidden` belong to checklist items only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FindingDisposition {
    Violation,
    AllowedChange,
    CantTell,
    Observed,
}

impl From<FindingDisposition> for Disposition {
    fn from(d: FindingDisposition) -> Disposition {
        match d {
            FindingDisposition::Violation => Disposition::Violation,
            FindingDisposition::AllowedChange => Disposition::AllowedChange,
            FindingDisposition::CantTell => Disposition::CantTell,
            FindingDisposition::Observed => Disposition::Observed,
        }
    }
}

/// Recorded, never read by aggregation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Opinion {
    /// Your own overall call: `pass`, `fail` or `unsure`.
    pub verdict: String,
    /// At most 600 characters.
    pub reason: String,
}

/// The canonical JSON Schema, generated from the types.
pub fn canonical_schema() -> serde_json::Value {
    let mut schema =
        serde_json::to_value(schemars::schema_for!(Submission)).unwrap_or(serde_json::Value::Null);
    close_objects(&mut schema);
    schema
}

/// Makes the generated schema canonical: every object schema is closed and every property is
/// required (schemars leaves `Option` fields optional; the canonical rule is required-but-nullable,
/// which is what every provider's strict mode wants).
fn close_objects(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(o) => {
            let is_object = o.get("type") == Some(&serde_json::Value::String("object".into()));
            if is_object && let Some(props) = o.get("properties").and_then(|p| p.as_object()) {
                let all: Vec<serde_json::Value> = props
                    .keys()
                    .map(|k| serde_json::Value::String(k.clone()))
                    .collect();
                o.insert("required".into(), serde_json::Value::Array(all));
                o.insert(
                    "additionalProperties".into(),
                    serde_json::Value::Bool(false),
                );
            }
            for value in o.values_mut() {
                close_objects(value);
            }
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(close_objects),
        _ => {}
    }
}

/// The `submit_verdict` tool as offered to the provider: the canonical schema as the model schema.
/// Adapters derive their wire schema from it (M2).
pub fn submit_verdict_tool() -> ToolDefinition {
    ToolDefinition {
        name: "submit_verdict".into(),
        description: "Submit the final structured verdict: exactly one disposition per checklist \
                      item, any further findings, and your own opinion (recorded only)."
            .into(),
        input_schema: canonical_schema(),
    }
}

/// Everything a submission is validated against.
pub struct ValidationContext<'a> {
    pub checklist: &'a Checklist,
    pub resolved: &'a Resolved,
    pub registry: &'a Registry,
    /// Tool call ids that completed with `ok` or `measured_nothing` in this run.
    pub completed_calls: &'a [String],
}

/// JSON Schema validation against the canonical schema.
pub fn validate_schema(arguments: &serde_json::Value) -> Result<()> {
    let schema = canonical_schema();
    let validator = jsonschema::validator_for(&schema)
        .map_err(|e| Error::Submission(format!("internal: canonical schema invalid: {e}")))?;
    let errors: Vec<String> = validator
        .iter_errors(arguments)
        .map(|e| format!("{} at {}", e, e.instance_path()))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Submission(format!(
            "does not match the submission schema: {}",
            errors.join("; ")
        )))
    }
}

/// Schema validation, then every semantic rule, in order.
pub fn parse_and_validate(
    arguments: &serde_json::Value,
    ctx: &ValidationContext<'_>,
) -> Result<Submission> {
    validate_schema(arguments)?;
    let sub: Submission = serde_json::from_value(arguments.clone())
        .map_err(|e| Error::Submission(format!("does not match the submission types: {e}")))?;
    if sub.schema_version != SUBMISSION_SCHEMA_VERSION {
        return Err(Error::Submission(format!(
            "schema_version must be {SUBMISSION_SCHEMA_VERSION}, got {}",
            sub.schema_version
        )));
    }
    validate_items(&sub, ctx)?;
    validate_findings(&sub, ctx)?;
    Ok(sub)
}

fn validate_items(sub: &Submission, ctx: &ValidationContext<'_>) -> Result<()> {
    let mut counts: HashMap<&str, usize> = ctx.checklist.ids().map(|id| (id, 0)).collect();
    for item in &sub.items {
        let Some(n) = counts.get_mut(item.id.as_str()) else {
            return Err(Error::Submission(format!(
                "unknown checklist item {:?}",
                item.id
            )));
        };
        *n += 1;
        let entry = ctx
            .checklist
            .get(&item.id)
            .ok_or_else(|| Error::Submission(format!("unknown checklist item {:?}", item.id)))?;
        if item.reason.trim().is_empty() {
            return Err(Error::Submission(format!(
                "{}: reason must not be empty",
                item.id
            )));
        }
        if let Some(loc) = &item.location {
            check_location(loc, ctx, &item.id)?;
        }
        match item.disposition {
            Disposition::Violation if item.location.is_none() => {
                return Err(Error::Submission(format!(
                    "{}: a violation needs a location",
                    item.id
                )));
            }
            Disposition::Observed => {
                return Err(Error::Submission(format!(
                    "{}: `observed` is a finding disposition, not an item one",
                    item.id
                )));
            }
            Disposition::AllowedChange => {
                let allowed_by = item.allowed_by.as_deref().ok_or_else(|| {
                    Error::Submission(format!("{}: allowed_change needs allowed_by", item.id))
                })?;
                check_exemption_applies(allowed_by, &entry.instance_ids, ctx, &item.id)?;
            }
            _ => {
                if item.allowed_by.is_some() {
                    return Err(Error::Submission(format!(
                        "{}: allowed_by is only valid with disposition allowed_change",
                        item.id
                    )));
                }
            }
        }
        check_evidence(&item.evidence, ctx, &item.id)?;
    }
    for (id, n) in counts {
        if n != 1 {
            return Err(Error::Submission(format!(
                "checklist item {id:?} must appear exactly once, appeared {n} times"
            )));
        }
    }
    Ok(())
}

fn validate_findings(sub: &Submission, ctx: &ValidationContext<'_>) -> Result<()> {
    for (n, f) in sub.findings.iter().enumerate() {
        let tag = format!("finding {n}");
        if f.reason.trim().is_empty() {
            return Err(Error::Submission(format!(
                "{tag}: reason must not be empty"
            )));
        }
        if f.obligation_ids.is_empty() {
            return Err(Error::Submission(format!("{tag}: cites no obligation")));
        }
        let mapped = check_location(&f.location, ctx, &tag)?;
        let root = root_image(&mapped, ctx);
        let mut mandatory_instances: Vec<String> = Vec::new();
        let mut cited_observe = 0usize;
        for id in &f.obligation_ids {
            let instances: Vec<_> = ctx
                .resolved
                .instances
                .iter()
                .filter(|i| &i.obligation_id == id)
                .collect();
            if !instances.is_empty() {
                let on_root = instances.iter().find(|i| Some(&i.target) == root.as_ref());
                match on_root {
                    Some(i) => mandatory_instances.push(i.id()),
                    None => {
                        return Err(Error::Submission(format!(
                            "{tag}: {id} does not judge image {root:?} where the finding is located"
                        )));
                    }
                }
            } else if let Some(ob) = ctx.resolved.observations.iter().find(|o| &o.id == id) {
                if !ob.targets.iter().any(|t| Some(t) == root.as_ref()) {
                    return Err(Error::Submission(format!(
                        "{tag}: observation {id} is not on image {}",
                        root.as_deref().unwrap_or("?")
                    )));
                }
                cited_observe += 1;
            } else if ctx.resolved.exemptions.iter().any(|e| &e.id == id) {
                return Err(Error::Submission(format!(
                    "{tag}: {id} is an exemption; cite the obligation it waives and name it in allowed_by"
                )));
            } else {
                return Err(Error::Submission(format!(
                    "{tag}: unknown obligation {id:?}; obligation_ids take rule ids as numbered under Obligations (for example SF4), not checklist item ids"
                )));
            }
        }
        match f.disposition {
            FindingDisposition::Observed => {
                if !mandatory_instances.is_empty() || cited_observe == 0 {
                    return Err(Error::Submission(format!(
                        "{tag}: `observed` findings cite observe rules only (such as SF4); a mandatory rule takes violation, allowed_change or cant_tell"
                    )));
                }
                if f.allowed_by.is_some() {
                    return Err(Error::Submission(format!(
                        "{tag}: observed findings take no allowed_by"
                    )));
                }
            }
            FindingDisposition::Violation | FindingDisposition::CantTell => {
                if mandatory_instances.is_empty() {
                    return Err(Error::Submission(format!(
                        "{tag}: a {:?} finding must cite at least one mandatory obligation",
                        f.disposition
                    )));
                }
                if f.allowed_by.is_some() {
                    return Err(Error::Submission(format!(
                        "{tag}: allowed_by is only valid with allowed_change"
                    )));
                }
            }
            FindingDisposition::AllowedChange => {
                if mandatory_instances.is_empty() {
                    return Err(Error::Submission(format!(
                        "{tag}: an allowed_change finding must cite the mandatory obligations waived"
                    )));
                }
                let allowed_by = f.allowed_by.as_deref().ok_or_else(|| {
                    Error::Submission(format!("{tag}: allowed_change needs allowed_by"))
                })?;
                check_exemption_applies(allowed_by, &mandatory_instances, ctx, &tag)?;
            }
        }
        check_evidence(&f.evidence, ctx, &tag)?;
    }
    Ok(())
}

/// Validity table, projection rules and mapping to the root frame (not wholly outside).
fn check_location(loc: &Location, ctx: &ValidationContext<'_>, who: &str) -> Result<Location> {
    let mapped = ctx
        .registry
        .to_root(loc)
        .map_err(|e| Error::Submission(format!("{who}: location invalid: {e}")))?;
    if loc.kind == LocationKind::Absent && loc.bbox.is_some() {
        return Err(Error::Submission(format!(
            "{who}: absent carries no coordinates"
        )));
    }
    Ok(mapped.location)
}

fn root_image(mapped: &Location, ctx: &ValidationContext<'_>) -> Option<String> {
    ctx.registry
        .get(&mapped.frame)
        .map(|f| f.root().to_string())
}

/// The exemption exists, waives every mandatory instance's obligation, and covers each target.
fn check_exemption_applies(
    allowed_by: &str,
    instance_ids: &[String],
    ctx: &ValidationContext<'_>,
    who: &str,
) -> Result<()> {
    let ex = ctx
        .resolved
        .exemptions
        .iter()
        .find(|e| e.id == allowed_by)
        .ok_or_else(|| {
            Error::Submission(format!(
                "{who}: allowed_by {allowed_by:?} is not an enabled exemption"
            ))
        })?;
    for inst_id in instance_ids {
        let inst = ctx.resolved.instance(inst_id).ok_or_else(|| {
            Error::Submission(format!("{who}: internal: unknown instance {inst_id}"))
        })?;
        if !ex.applies_to(&inst.obligation_id, &inst.target) {
            return Err(Error::Submission(format!(
                "{who}: exemption {allowed_by} does not waive {} on {}",
                inst.obligation_id, inst.target
            )));
        }
    }
    Ok(())
}

fn check_evidence(evidence: &[String], ctx: &ValidationContext<'_>, who: &str) -> Result<()> {
    for call in evidence {
        if !ctx.completed_calls.iter().any(|c| c == call) {
            return Err(Error::Submission(format!(
                "{who}: evidence {call:?} is not a tool call that completed in this run"
            )));
        }
    }
    Ok(())
}
