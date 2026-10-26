//! The checklist: what the model must answer, item by item. Task 1.1 builds one item per
//! mandatory instance; reference items, evidence items, drops and the budget arrive in 1.5b.

use serde::{Deserialize, Serialize};

use crate::obligations::Instance;

/// One checklist item with its code-owned dependencies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChecklistItem {
    /// `chk:ob:<obligation>@<target>` for static items.
    pub id: String,
    /// Instance ids this item depends on; every one is mandatory.
    pub instance_ids: Vec<String>,
}

/// Builds the static checklist from the mandatory instances.
pub fn build(instances: &[Instance]) -> Vec<ChecklistItem> {
    instances
        .iter()
        .filter(|i| i.is_mandatory())
        .map(|i| ChecklistItem {
            id: format!("chk:ob:{}", i.id()),
            instance_ids: vec![i.id()],
        })
        .collect()
}

/// Renders the checklist as the model sees it.
pub fn render(items: &[ChecklistItem], instances: &[Instance]) -> String {
    let mut out = String::from("Checklist. Give every item below exactly one disposition.\n");
    for item in items {
        let texts: Vec<&str> = instances
            .iter()
            .filter(|i| item.instance_ids.contains(&i.id()))
            .map(|i| i.text.as_str())
            .collect();
        out.push_str(&format!("- {}: {}\n", item.id, texts.join(" ")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obligations::Kind;

    fn inst(id: &str, kind: Kind) -> Instance {
        Instance {
            obligation_id: id.into(),
            target: "candidate".into(),
            kind,
            matters: true,
            hidden_ok: false,
            text: format!("text of {id}"),
            evidence: vec![],
            evidence_kinds: vec![],
            provenance: "test".into(),
        }
    }

    #[test]
    fn one_item_per_mandatory_instance_and_none_for_others() {
        let items = build(&[
            inst("SF1", Kind::MustHold),
            inst("SF3", Kind::Allowed),
            inst("SF4", Kind::Observe),
        ]);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "chk:ob:SF1@candidate");
        assert_eq!(items[0].instance_ids, vec!["SF1@candidate"]);
    }

    #[test]
    fn rendering_lists_each_item_with_its_text() {
        let instances = [inst("SF2", Kind::MustHold)];
        let text = render(&build(&instances), &instances);
        assert!(text.contains("- chk:ob:SF2@candidate: text of SF2"));
    }
}
