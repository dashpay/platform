use crate::drive::Drive;
use crate::error::Error;
use crate::structure::{ElementKind, KeySpec, NodeId, Presence, StructureNode};
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use std::collections::BTreeSet;
use std::fmt;

/// One way a real GroveDB differs from the description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Violation {
    /// An element exists that no node describes
    UndescribedKey {
        /// The node describing the layer the element is in
        layer: NodeId,
        /// The path of the layer
        path: Vec<Vec<u8>>,
        /// The key of the element
        key: Vec<u8>,
        /// The kind of the element
        kind: ElementKind,
    },
    /// An element is of a kind its node does not list
    KindMismatch {
        /// The node describing the element
        node: NodeId,
        /// The path of the layer
        path: Vec<Vec<u8>>,
        /// The kinds the node lists
        expected: Vec<ElementKind>,
        /// The kind of the element
        actual: ElementKind,
    },
    /// An element exists in a protocol version its node says it is not in
    OutsideItsVersions {
        /// The node describing the element
        node: NodeId,
        /// The first protocol version of the node
        since: ProtocolVersion,
        /// The last protocol version of the node
        until: Option<ProtocolVersion>,
    },
    /// A node that is created with its parent is missing
    MissingRequired {
        /// The missing node
        node: NodeId,
        /// The path of the layer it should be in
        path: Vec<Vec<u8>>,
    },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Violation::UndescribedKey {
                layer,
                path,
                key,
                kind,
            } => write!(
                f,
                "{:?} at key {} of layer `{layer}` {} is not described",
                kind,
                readable_key(key),
                readable_path(path),
            ),
            Violation::KindMismatch {
                node,
                path,
                expected,
                actual,
            } => write!(
                f,
                "`{node}` in {} is {actual:?}, expected one of {expected:?}",
                readable_path(path),
            ),
            Violation::OutsideItsVersions { node, since, until } => write!(
                f,
                "`{node}` exists but is described as since {since} until {until:?}"
            ),
            Violation::MissingRequired { node, path } => write!(
                f,
                "`{node}` is created with its parent but is missing in {}",
                readable_path(path),
            ),
        }
    }
}

/// The result of comparing a GroveDB with the description.
#[derive(Clone, Debug, Default)]
pub struct ConformanceReport {
    /// Every difference found
    pub violations: Vec<Violation>,
    /// Every node at least one element was matched to
    pub visited: BTreeSet<NodeId>,
}

impl ConformanceReport {
    /// Panics with every violation listed if there is any
    pub fn assert_conforms(&self) {
        assert!(
            self.violations.is_empty(),
            "GroveDB differs from the structure description:\n{}\n\n\
             Describe new structure in the `structure.rs` beside the area's `paths.rs`.",
            self.violations
                .iter()
                .map(|violation| format!("  - {violation}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
}

/// Walks the whole GroveDB of `drive` and compares every element with the
/// description: each one must match a node (a fixed key first, then the
/// layer's templates), be of a kind the node lists, and exist in the protocol
/// version of `platform_version`; and every node created with its parent
/// must be there.
///
/// Trees that are not a Merk of elements and references are checked but not
/// looked into.
pub fn check_conformance(
    drive: &Drive,
    root: &StructureNode,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConformanceReport, Error> {
    let protocol_version = platform_version.protocol_version;
    let mut report = ConformanceReport::default();
    report.visited.insert(root.id.clone());

    let mut layers: Vec<(Vec<Vec<u8>>, &StructureNode)> = vec![(vec![], root)];
    while let Some((path, node)) = layers.pop() {
        let described = children_of(root, node);

        let mut query = Query::new();
        query.insert_all();
        let path_query = PathQuery::new(path.clone(), SizedQuery::new(query, None, None));
        let (elements, _) = drive.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let mut found = BTreeSet::new();
        for (key, element) in elements.to_key_elements() {
            let kind = ElementKind::of(&element);
            let Some(child) = resolve(described, &key, kind) else {
                report.violations.push(Violation::UndescribedKey {
                    layer: node.id.clone(),
                    path: path.clone(),
                    key,
                    kind,
                });
                continue;
            };
            found.insert(child.id.as_str());
            report.visited.insert(child.id.clone());

            if !child.exists_in(protocol_version) {
                report.violations.push(Violation::OutsideItsVersions {
                    node: child.id.clone(),
                    since: child.since,
                    until: child.until,
                });
            }
            if !child.kinds.contains(&kind) {
                report.violations.push(Violation::KindMismatch {
                    node: child.id.clone(),
                    path: path.clone(),
                    expected: child.kinds.clone(),
                    actual: kind,
                });
                continue;
            }
            if kind.is_tree() && !kind.is_opaque() {
                let mut child_path = path.clone();
                child_path.push(key);
                layers.push((child_path, child));
            }
        }

        for child in described {
            let required = matches!(child.key, KeySpec::Fixed { .. })
                && child.presence == Presence::Always
                && child.exists_in(protocol_version);
            if required && !found.contains(child.id.as_str()) {
                report.violations.push(Violation::MissingRequired {
                    node: child.id.clone(),
                    path: path.clone(),
                });
            }
        }
    }

    Ok(report)
}

/// The nodes describing the layer below `node`, following `recurse`
fn children_of<'a>(root: &'a StructureNode, node: &'a StructureNode) -> &'a [StructureNode] {
    match node.recurse.as_deref().and_then(|target| root.find(target)) {
        Some(target) => &target.children,
        None => &node.children,
    }
}

/// The node describing an element: the fixed key if one matches, otherwise a
/// template accepting the key. Among templates, one that lists the element's
/// kind wins, then one for keys of a given length over one for any key.
fn resolve<'a>(
    described: &'a [StructureNode],
    key: &[u8],
    kind: ElementKind,
) -> Option<&'a StructureNode> {
    if let Some(fixed) = described
        .iter()
        .find(|child| child.fixed_key_bytes() == Some(key))
    {
        return Some(fixed);
    }
    described
        .iter()
        .filter_map(|child| match &child.key {
            KeySpec::Dynamic { matcher, .. } if matcher.accepts(key) => {
                Some((child, !child.kinds.contains(&kind), matcher.is_any()))
            }
            KeySpec::Root | KeySpec::Fixed { .. } | KeySpec::Dynamic { .. } => None,
        })
        .min_by_key(|(_, wrong_kind, any_key)| (*wrong_kind, *any_key))
        .map(|(child, _, _)| child)
}

fn readable_key(key: &[u8]) -> String {
    let printable = !key.is_empty() && key.iter().all(|byte| byte.is_ascii_graphic());
    if printable && key.len() > 1 || key.len() == 1 && key[0].is_ascii_alphabetic() {
        format!(
            "0x{} ('{}')",
            hex::encode(key),
            String::from_utf8_lossy(key)
        )
    } else {
        format!("0x{}", hex::encode(key))
    }
}

fn readable_path(path: &[Vec<u8>]) -> String {
    format!(
        "[{}]",
        path.iter()
            .map(|key| readable_key(key))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
