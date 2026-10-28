//! Arm membership (SPEC "Runner and arms"): A has no tools; C is the visual-access control,
//! `view` and `align` with `align` under its `visual_only` projection; D_X is C plus exactly the
//! tool under test, `align` staying projected unless it is the tool under test; D is C plus every
//! tool the admission record holds as `confirmed` for the use case.

use std::collections::BTreeMap;

use crate::admissions::Admissions;
use crate::matrix::Arm;
use crate::{Error, Result};

pub const VIEW: &str = "view";
pub const ALIGN: &str = "align";
pub const VISUAL_ONLY: &str = "visual_only";

/// The tools a cell enables, in prompt order, and the projection each one runs under.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ToolSelection {
    pub enabled: Vec<String>,
    pub projections: BTreeMap<String, String>,
}

pub fn tools_for(arm: &Arm, admissions: &Admissions, use_case: &str) -> Result<ToolSelection> {
    let mut sel = ToolSelection::default();
    match arm {
        Arm::A => {}
        Arm::C => {
            sel.enabled = vec![VIEW.into(), ALIGN.into()];
            sel.projections.insert(ALIGN.into(), VISUAL_ONLY.into());
        }
        Arm::DX(tool) => {
            if tool == VIEW {
                return Err(Error::Runner(format!(
                    "arm D_{tool}: {tool} is part of the control and cannot be the tool under test"
                )));
            }
            sel.enabled = vec![VIEW.into(), ALIGN.into()];
            if tool != ALIGN {
                sel.enabled.push(tool.clone());
                sel.projections.insert(ALIGN.into(), VISUAL_ONLY.into());
            }
        }
        Arm::D => {
            sel.enabled = vec![VIEW.into(), ALIGN.into()];
            let admitted = admissions.admitted(use_case);
            for tool in &admitted {
                if !sel.enabled.contains(tool) {
                    sel.enabled.push(tool.clone());
                }
            }
            // `align` stays the projected control unless it was itself admitted in full.
            if !admitted.iter().any(|t| t == ALIGN) {
                sel.projections.insert(ALIGN.into(), VISUAL_ONLY.into());
            }
        }
    }
    Ok(sel)
}
