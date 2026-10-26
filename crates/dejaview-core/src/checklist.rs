//! The checklist: what the model must answer, item by item (SPEC "Checklist"). Static items come
//! from the resolved obligations before the first turn; evidence items are added as tool results
//! present regions, bound by evidence kind and root image. Every verdict-bearing item depends on at
//! least one mandatory instance; nothing shown to the model lacks an owner.

use serde::{Deserialize, Serialize};

use crate::frames::{Location, Registry};
use crate::job::Role;
use crate::obligations::{Instance, Resolved};
use crate::tool::envelope::{Envelope, Region};
use crate::tool::manifest::Manifest;
use crate::{Error, Result};

/// One checklist item with its code-owned fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChecklistItem {
    /// `chk:ob:<obligation>@<target>`, `chk:ref:<image>` or `chk:ev:<call>:<n>`.
    pub id: String,
    /// Instance ids this item depends on; every one is mandatory.
    pub instance_ids: Vec<String>,
    /// The image the item is about (the target for static items; the region's root for evidence).
    pub root_image: Option<String>,
    /// For evidence items: the region's location as returned, and mapped to the root frame.
    pub location: Option<Location>,
    pub root_location: Option<Location>,
    /// Evidence items on the same root image whose raw-coordinate IoU exceeds the overlap floor.
    pub overlaps: Vec<String>,
    /// The region's label from the tool, for rendering.
    pub label: Option<String>,
}

/// A region a tool presented that the checklist did not take, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dropped {
    pub call_id: String,
    pub region_id: String,
    pub reason: String,
}

/// What ingesting one envelope did.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ingest {
    pub added: Vec<String>,
    pub dropped: Vec<Dropped>,
}

/// IoU above which two evidence items on the same root image cross-reference each other.
pub const OVERLAP_IOU: f64 = 0.7;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Checklist {
    pub items: Vec<ChecklistItem>,
}

impl Checklist {
    /// Builds the static items: one per mandatory instance and one per reference image.
    pub fn build(resolved: &Resolved, images: &[(String, Role)]) -> Result<Checklist> {
        let mut items = Vec::new();
        for inst in resolved.instances.iter().filter(|i| i.is_mandatory()) {
            items.push(ChecklistItem {
                id: format!("chk:ob:{}", inst.id()),
                instance_ids: vec![inst.id()],
                root_image: Some(inst.target.clone()),
                location: None,
                root_location: None,
                overlaps: Vec::new(),
                label: None,
            });
        }
        for (id, _) in images.iter().filter(|(_, r)| *r == Role::Reference) {
            let deps: Vec<String> = resolved
                .instances
                .iter()
                .filter(|i| i.is_mandatory() && i.evidence.contains(id))
                .map(Instance::id)
                .collect();
            if deps.is_empty() {
                return Err(Error::Config(format!(
                    "checklist: reference {id:?} is consulted by no mandatory instance"
                )));
            }
            items.push(ChecklistItem {
                id: format!("chk:ref:{id}"),
                instance_ids: deps,
                root_image: Some(id.clone()),
                location: None,
                root_location: None,
                overlaps: Vec::new(),
                label: None,
            });
        }
        let list = Checklist { items };
        list.check_invariant()?;
        Ok(list)
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.items.iter().map(|i| i.id.as_str())
    }

    pub fn get(&self, id: &str) -> Option<&ChecklistItem> {
        self.items.iter().find(|i| i.id == id)
    }

    fn evidence_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.id.starts_with("chk:ev:"))
            .count()
    }

    /// Every item depends on at least one mandatory instance.
    pub fn check_invariant(&self) -> Result<()> {
        match self.items.iter().find(|i| i.instance_ids.is_empty()) {
            Some(i) => Err(Error::Config(format!(
                "checklist: item {} has no mandatory dependency",
                i.id
            ))),
            None => Ok(()),
        }
    }

    /// Takes the regions of a tool result into the checklist. Regions with no owner are removed
    /// from the envelope and reported as dropped; exceeding `evidence_budget` is `error: budget`
    /// before anything is presented. Items are appended in region order.
    pub fn ingest(
        &mut self,
        call_id: &str,
        envelope: &mut Envelope,
        manifest: &Manifest,
        resolved: &Resolved,
        registry: &Registry,
        evidence_budget: usize,
    ) -> Result<Ingest> {
        let mut out = Ingest::default();
        let Some(kind) = manifest.evidence_kind.as_deref() else {
            if !envelope.regions.is_empty() {
                for r in envelope.regions.drain(..) {
                    out.dropped.push(Dropped {
                        call_id: call_id.into(),
                        region_id: r.id,
                        reason: format!("tool {} declares no evidence_kind", manifest.name),
                    });
                }
            }
            return Ok(out);
        };

        // Bind first, present second: a result that would exceed the budget is never shown.
        let mut bound: Vec<(Region, Vec<String>, Location)> = Vec::new();
        let mut kept_regions = Vec::new();
        for region in envelope.regions.drain(..) {
            let mapped = match registry.to_root(&region.location) {
                Ok(m) => m,
                Err(e) => {
                    out.dropped.push(Dropped {
                        call_id: call_id.into(),
                        region_id: region.id.clone(),
                        reason: format!("location invalid: {e}"),
                    });
                    continue;
                }
            };
            let root = registry
                .get(&mapped.location.frame)
                .map(|f| f.root().to_string())
                .unwrap_or_default();
            let deps: Vec<String> = resolved
                .instances
                .iter()
                .filter(|i| {
                    i.is_mandatory()
                        && i.target == root
                        && i.evidence_kinds.iter().any(|k| k == kind)
                })
                .map(Instance::id)
                .collect();
            if deps.is_empty() {
                out.dropped.push(Dropped {
                    call_id: call_id.into(),
                    region_id: region.id.clone(),
                    reason: format!(
                        "no mandatory instance on {root:?} accepts evidence kind {kind:?}"
                    ),
                });
                continue;
            }
            bound.push((region.clone(), deps, mapped.location.clone()));
            kept_regions.push(region);
        }
        if self.evidence_count() + bound.len() > evidence_budget {
            return Err(Error::Budget(format!(
                "evidence items would reach {} of a budget of {evidence_budget}; the result of {call_id} is not presented",
                self.evidence_count() + bound.len()
            )));
        }
        envelope.regions = kept_regions;

        for (n, (region, deps, root_location)) in bound.into_iter().enumerate() {
            let id = format!("chk:ev:{}:{n}", call_id.trim_start_matches("call:"));
            let root_image = registry
                .get(&root_location.frame)
                .map(|f| f.root().to_string());
            let overlaps: Vec<String> = self
                .items
                .iter()
                .filter(|other| {
                    other.id.starts_with("chk:ev:")
                        && other.root_image == root_image
                        && other
                            .root_location
                            .as_ref()
                            .zip(Some(&root_location))
                            .is_some_and(|(a, b)| iou(a, b) > OVERLAP_IOU)
                })
                .map(|o| o.id.clone())
                .collect();
            for o in &overlaps {
                if let Some(other) = self.items.iter_mut().find(|i| &i.id == o) {
                    other.overlaps.push(id.clone());
                }
            }
            out.added.push(id.clone());
            self.items.push(ChecklistItem {
                id,
                instance_ids: deps,
                root_image,
                location: Some(region.location.clone()),
                root_location: Some(root_location),
                overlaps,
                label: Some(region.label.clone()),
            });
        }
        self.check_invariant()?;
        Ok(out)
    }

    /// Renders the checklist as the model sees it.
    pub fn render(&self, instances: &[Instance]) -> String {
        let mut out = String::from("Checklist. Give every item below exactly one disposition.\n");
        for item in &self.items {
            let texts: Vec<&str> = instances
                .iter()
                .filter(|i| item.instance_ids.contains(&i.id()))
                .map(|i| i.text.as_str())
                .collect();
            let mut line = format!("- {}", item.id);
            if let Some(label) = &item.label {
                line.push_str(&format!(" [{label}]"));
            }
            if let Some(loc) = &item.location {
                line.push_str(&format!(" at {}", describe(loc)));
            }
            line.push_str(&format!(": {}", texts.join(" ")));
            if !item.overlaps.is_empty() {
                line.push_str(&format!(" (overlaps {})", item.overlaps.join(", ")));
            }
            out.push_str(&line);
            out.push('\n');
        }
        out
    }
}

fn describe(loc: &Location) -> String {
    match (&loc.bbox, &loc.polygon, loc.yaw) {
        (Some(b), _, _) => format!(
            "{} bbox [{:.2}, {:.2}, {:.2}, {:.2}]",
            loc.frame, b[0], b[1], b[2], b[3]
        ),
        (_, Some(p), _) => format!("{} polygon with {} vertices", loc.frame, p.len()),
        (_, _, Some(y)) => format!(
            "{} view yaw {y:.1} pitch {:.1} fov {:.1}",
            loc.frame,
            loc.pitch.unwrap_or(0.0),
            loc.fov.unwrap_or(0.0)
        ),
        _ => format!("{} ({:?})", loc.frame, loc.kind),
    }
}

/// Intersection over union of two root-frame locations' bounding boxes (polygons by their bounds).
pub fn iou(a: &Location, b: &Location) -> f64 {
    let (Some(ba), Some(bb)) = (bounds(a), bounds(b)) else {
        return 0.0;
    };
    let ix = (ba[2].min(bb[2]) - ba[0].max(bb[0])).max(0.0);
    let iy = (ba[3].min(bb[3]) - ba[1].max(bb[1])).max(0.0);
    let inter = ix * iy;
    let union = area(&ba) + area(&bb) - inter;
    if union <= 0.0 { 0.0 } else { inter / union }
}

fn bounds(l: &Location) -> Option<[f64; 4]> {
    if let Some(b) = l.bbox {
        return Some(b);
    }
    let p = l.polygon.as_ref()?;
    let xs = p.iter().map(|v| v[0]);
    let ys = p.iter().map(|v| v[1]);
    Some([
        xs.clone().fold(f64::INFINITY, f64::min),
        ys.clone().fold(f64::INFINITY, f64::min),
        xs.fold(f64::NEG_INFINITY, f64::max),
        ys.fold(f64::NEG_INFINITY, f64::max),
    ])
}

fn area(b: &[f64; 4]) -> f64 {
    (b[2] - b[0]).max(0.0) * (b[3] - b[1]).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::LocationKind;

    fn bbox(frame: &str, b: [f64; 4]) -> Location {
        Location {
            frame: frame.into(),
            kind: LocationKind::Bbox,
            bbox: Some(b),
            polygon: None,
            yaw: None,
            pitch: None,
            fov: None,
        }
    }

    #[test]
    fn iou_of_identical_boxes_is_one_and_of_disjoint_boxes_is_zero() {
        let a = bbox("raw:c", [0.0, 0.0, 0.5, 0.5]);
        assert!((iou(&a, &a) - 1.0).abs() < 1e-9);
        let b = bbox("raw:c", [0.6, 0.6, 1.0, 1.0]);
        assert_eq!(iou(&a, &b), 0.0);
        let c = bbox("raw:c", [0.0, 0.0, 0.5, 0.25]);
        assert!((iou(&a, &c) - 0.5).abs() < 1e-9);
    }
}
