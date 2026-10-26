//! The tool registry for one run: the enabled tools in prompt order, with lookup by name.

use super::{Manifest, Tool};
use crate::{Error, Result};

pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Keeps the tools named in `enabled`, ordered by `order` where given, then by cost tier
    /// (cheap first) with `requires` before dependants.
    pub fn new(
        mut available: Vec<Box<dyn Tool>>,
        enabled: &[String],
        order: &[String],
    ) -> Result<ToolRegistry> {
        let mut tools: Vec<Box<dyn Tool>> = Vec::new();
        for name in enabled {
            let pos = available
                .iter()
                .position(|t| &t.manifest().name == name)
                .ok_or_else(|| {
                    Error::Config(format!(
                        "tool {name:?} is enabled but not available in this build"
                    ))
                })?;
            tools.push(available.remove(pos));
        }
        for t in &tools {
            for req in &t.manifest().requires {
                if !tools.iter().any(|o| &o.manifest().name == req) {
                    return Err(Error::Config(format!(
                        "tool {:?} requires {req:?}, which is not enabled",
                        t.manifest().name
                    )));
                }
            }
        }
        tools.sort_by_key(|t| {
            let m = t.manifest();
            let explicit = order.iter().position(|o| o == &m.name);
            let tier = m.cost.tier as u8;
            (
                explicit.is_none(),
                explicit.unwrap_or(0),
                tier,
                m.name.clone(),
            )
        });
        // Requires before dependants: stable pass moving prerequisites ahead.
        let mut i = 0;
        while i < tools.len() {
            let reqs = tools[i].manifest().requires.clone();
            if let Some(j) = reqs
                .iter()
                .filter_map(|r| tools.iter().position(|t| &t.manifest().name == r))
                .find(|&j| j > i)
            {
                let dep = tools.remove(j);
                tools.insert(i, dep);
                continue;
            }
            i += 1;
        }
        Ok(ToolRegistry { tools })
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn manifests(&self) -> impl Iterator<Item = &Manifest> {
        self.tools.iter().map(|t| t.manifest())
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.manifest().name == name)
            .map(|t| t.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{DejaviewContext, Envelope};

    struct Named(Manifest);
    impl Tool for Named {
        fn manifest(&self) -> &Manifest {
            &self.0
        }
        fn call(&self, _: &serde_json::Value, _: &DejaviewContext) -> Result<Envelope> {
            Err(Error::Tool("unused".into()))
        }
    }

    fn tool(name: &str, tier: &str, requires: &[&str]) -> Box<dyn Tool> {
        let text = include_str!("../../../../fixtures/tools/regions.toml")
            .replace("name = \"regions\"", &format!("name = \"{name}\""))
            .replace("tier = \"expensive\"", &format!("tier = \"{tier}\""))
            .replace(
                "requires = [\"align\"]",
                &format!(
                    "requires = [{}]",
                    requires
                        .iter()
                        .map(|r| format!("\"{r}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        Box::new(Named(Manifest::from_toml(&text).expect("manifest")))
    }

    #[test]
    fn default_order_is_cheap_first_with_requirements_ahead() {
        let reg = ToolRegistry::new(
            vec![
                tool("regions", "expensive", &["align"]),
                tool("view", "cheap", &[]),
                tool("align", "moderate", &[]),
            ],
            &["regions".into(), "view".into(), "align".into()],
            &[],
        )
        .expect("registry");
        let names: Vec<&str> = reg.manifests().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["view", "align", "regions"]);
    }

    #[test]
    fn explicit_order_wins_but_requirements_still_come_first() {
        let reg = ToolRegistry::new(
            vec![
                tool("regions", "expensive", &["align"]),
                tool("view", "cheap", &[]),
                tool("align", "moderate", &[]),
            ],
            &["regions".into(), "view".into(), "align".into()],
            &["regions".into(), "view".into()],
        )
        .expect("registry");
        let names: Vec<&str> = reg.manifests().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["align", "regions", "view"]);
    }

    #[test]
    fn missing_requirement_and_unknown_tool_are_errors() {
        assert!(
            ToolRegistry::new(
                vec![tool("regions", "expensive", &["align"])],
                &["regions".into()],
                &[]
            )
            .is_err()
        );
        assert!(ToolRegistry::new(vec![], &["nope".into()], &[]).is_err());
    }
}
