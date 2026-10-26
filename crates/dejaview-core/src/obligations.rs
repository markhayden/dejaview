//! Obligations and their instances. Task 1.1 ships only the instance type and the walking slice's
//! single hard-coded obligation; the full model (rules files, merge, scope and evidence
//! resolution, exemptions) arrives in tasks 1.4a and 1.4b.

use serde::{Deserialize, Serialize};

use crate::job::Job;

/// What an obligation does in aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    MustHold,
    MustNot,
    Allowed,
    Observe,
}

impl Kind {
    pub fn is_mandatory(self) -> bool {
        matches!(self, Kind::MustHold | Kind::MustNot)
    }
}

/// One obligation resolved to one target image: `SF2@candidate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub obligation_id: String,
    pub target: String,
    pub kind: Kind,
    pub matters: bool,
    pub hidden_ok: bool,
    pub text: String,
}

impl Instance {
    /// The instance id, `<obligation>@<target>`.
    pub fn id(&self) -> String {
        format!("{}@{}", self.obligation_id, self.target)
    }

    pub fn is_mandatory(&self) -> bool {
        self.kind.is_mandatory()
    }
}

/// The walking slice's instances: SF2 (same scene, same camera) on the candidate. Replaced by
/// rules-file resolution in task 1.4b.
pub fn walking_slice_instances(job: &Job) -> Vec<Instance> {
    let target = job
        .candidate()
        .map(|c| c.id.clone())
        .unwrap_or_else(|| "candidate".into());
    vec![Instance {
        obligation_id: "SF2".into(),
        target,
        kind: Kind::MustHold,
        matters: true,
        hidden_ok: false,
        text:
            "The candidate is derived from the source: same scene, same camera, same projection; \
               nothing rotated, stretched or cropped away."
                .into(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_id_joins_obligation_and_target() {
        let inst = Instance {
            obligation_id: "SF2".into(),
            target: "candidate".into(),
            kind: Kind::MustHold,
            matters: true,
            hidden_ok: false,
            text: String::new(),
        };
        assert_eq!(inst.id(), "SF2@candidate");
        assert!(inst.is_mandatory());
    }

    #[test]
    fn only_must_hold_and_must_not_are_mandatory() {
        assert!(Kind::MustHold.is_mandatory());
        assert!(Kind::MustNot.is_mandatory());
        assert!(!Kind::Allowed.is_mandatory());
        assert!(!Kind::Observe.is_mandatory());
    }
}
