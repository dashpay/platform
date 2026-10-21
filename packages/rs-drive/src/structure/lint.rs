use crate::structure::{KeySpec, StructureNode};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Checks that the description is internally consistent and returns one
/// message per problem found.
///
/// `repository_root` is used to check that every `source` file exists and
/// names the constant a fixed key claims to come from; pass `None` to skip
/// those checks.
pub fn lint(root: &StructureNode, repository_root: Option<&Path>) -> Vec<String> {
    let mut problems = vec![];
    let mut ids = BTreeSet::new();

    root.walk(&mut |node| {
        if !ids.insert(node.id.as_str()) {
            problems.push(format!("{}: identifier used twice", node.id));
        }
        if node.segment.is_empty()
            || !node
                .segment
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            problems.push(format!(
                "{}: segment `{}` must be lowercase letters, digits and underscores",
                node.id, node.segment
            ));
        }
        if node.kinds.is_empty() {
            problems.push(format!("{}: no element kind", node.id));
        }
        if node.kinds.len() > 1 && node.kinds_note.is_none() {
            problems.push(format!(
                "{}: several element kinds but nothing says what decides",
                node.id
            ));
        }
        if node.flags.is_empty() {
            problems.push(format!("{}: no element flags kind", node.id));
        }
        if node.flags.len() > 1 && node.flags_note.is_none() {
            problems.push(format!(
                "{}: several kinds of element flags but nothing says what decides",
                node.id
            ));
        }
        if node.description.is_empty() && !matches!(node.key, KeySpec::Root) {
            problems.push(format!("{}: no description", node.id));
        }
        if node.source.is_empty() && !matches!(node.key, KeySpec::Root) {
            problems.push(format!("{}: no source file", node.id));
        }
        if node.until.is_some_and(|until| until < node.since) {
            problems.push(format!("{}: `until` is before `since`", node.id));
        }

        let holds_layer = node.kinds.iter().any(|kind| kind.is_tree());
        let has_below = !node.children.is_empty() || node.recurse.is_some();
        if has_below && !holds_layer {
            problems.push(format!("{}: has children but is never a tree", node.id));
        }
        if has_below && node.opaque.is_some() {
            problems.push(format!("{}: opaque trees cannot have children", node.id));
        }
        let always_opaque = node.kinds.iter().all(|kind| kind.is_opaque());
        if node.opaque.is_some() != (always_opaque && !node.kinds.is_empty()) {
            problems.push(format!(
                "{}: `opaque` must be set exactly when every kind is a non-Merk tree",
                node.id
            ));
        }
        let points_somewhere = node.kinds.iter().any(|kind| kind.is_reference());
        if node.reference.is_some() != points_somewhere {
            problems.push(format!(
                "{}: `reference` must be set exactly when a kind is a reference",
                node.id
            ));
        }

        if node.states.len() == 1 {
            problems.push(format!("{}: a single state says nothing", node.id));
        }
        let mut state_names = BTreeSet::new();
        for state in &node.states {
            if !state_names.insert(state.name.as_str()) {
                problems.push(format!("{}: state `{}` listed twice", node.id, state.name));
            }
            if state.description.is_empty() || state.title.is_empty() {
                problems.push(format!(
                    "{}: state `{}` needs a title and a description",
                    node.id, state.name
                ));
            }
            for key in &state.keys {
                let fixed_child = node.children.iter().any(|child| {
                    child.segment == *key && matches!(child.key, KeySpec::Fixed { .. })
                });
                if !fixed_child {
                    problems.push(format!(
                        "{}: state `{}` names `{key}`, which is not a fixed key of the layer",
                        node.id, state.name
                    ));
                }
            }
        }

        lint_layer(node, &mut problems);
    });

    root.walk(&mut |node| {
        for (what, target) in [("recurse", &node.recurse), ("reference", &node.reference)] {
            if let Some(target) = target {
                if !ids.contains(target.as_str()) {
                    problems.push(format!(
                        "{}: {what} target `{target}` does not exist",
                        node.id
                    ));
                }
            }
        }
    });

    if let Some(repository_root) = repository_root {
        lint_sources(root, repository_root, &mut problems);
    }

    problems
}

/// Within one layer: no fixed key twice, no segment twice, and no two
/// templates that could claim the same element.
fn lint_layer(node: &StructureNode, problems: &mut Vec<String>) {
    let mut fixed_keys = BTreeSet::new();
    let mut segments = BTreeSet::new();
    for child in &node.children {
        if !segments.insert(child.segment.as_str()) {
            problems.push(format!("{}: segment used twice in one layer", child.id));
        }
        if let Some(bytes) = child.fixed_key_bytes() {
            if !fixed_keys.insert(bytes) {
                problems.push(format!("{}: fixed key used twice in one layer", child.id));
            }
        }
    }

    let templates: Vec<_> = node
        .children
        .iter()
        .filter_map(|child| match &child.key {
            KeySpec::Dynamic { matcher, .. } => Some((child, matcher)),
            KeySpec::Root | KeySpec::Fixed { .. } => None,
        })
        .collect();
    for (index, (a, a_matcher)) in templates.iter().enumerate() {
        for (b, b_matcher) in templates.iter().skip(index + 1) {
            // A template for keys of a given length wins over one for any
            // key, so only equally specific templates can clash
            let share_kind = a.kinds.iter().any(|kind| b.kinds.contains(kind));
            let equally_specific = a_matcher.is_any() == b_matcher.is_any();
            if a_matcher.overlaps(b_matcher) && equally_specific && share_kind {
                problems.push(format!(
                    "{} and {}: templates accept the same keys and kinds",
                    a.id, b.id
                ));
            }
        }
    }
}

/// Every source file exists, and names the constant of each fixed key.
fn lint_sources(root: &StructureNode, repository_root: &Path, problems: &mut Vec<String>) {
    root.walk(&mut |node| {
        if node.source.is_empty() {
            return;
        }
        let Ok(contents) = fs::read_to_string(repository_root.join(&node.source)) else {
            problems.push(format!(
                "{}: source file `{}` cannot be read",
                node.id, node.source
            ));
            return;
        };
        if let KeySpec::Fixed { constant, .. } = &node.key {
            // `RootTree::Tokens` is declared as `Tokens` in its file
            let name = constant.rsplit("::").next().unwrap_or(constant);
            if !name.is_empty() && !contents.contains(name) {
                problems.push(format!(
                    "{}: `{}` is not in `{}`",
                    node.id, constant, node.source
                ));
            }
        }
    });
}
