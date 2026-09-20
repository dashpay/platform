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
/// version of `platform_version`; and every node that is created with its
/// parent and never deleted must be there.
///
/// Where several templates of a layer accept a key, the one whose description
/// fits what is below the element wins. A key alone cannot always tell: below
/// a contested index a 32 byte key is a contender's identity id at the last
/// level and an index value at the levels before it.
///
/// Trees that are not a Merk of elements and references are checked but not
/// looked into.
pub fn check_conformance(
    drive: &Drive,
    root: &StructureNode,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConformanceReport, Error> {
    let walk = Walk {
        drive,
        root,
        transaction,
        platform_version,
    };
    let mut report = ConformanceReport::default();
    report.visited.insert(root.id.clone());
    walk.layer(&[], root, &mut report)?;
    Ok(report)
}

struct Walk<'a> {
    drive: &'a Drive,
    root: &'a StructureNode,
    transaction: TransactionArg<'a, 'a>,
    platform_version: &'a PlatformVersion,
}

impl Walk<'_> {
    /// Checks every element of the layer at `path`, which `node` describes
    fn layer(
        &self,
        path: &[Vec<u8>],
        node: &StructureNode,
        report: &mut ConformanceReport,
    ) -> Result<(), Error> {
        let protocol_version = self.platform_version.protocol_version;
        let described = children_of(self.root, node);

        let mut query = Query::new();
        query.insert_all();
        let path_query = PathQuery::new(path.to_vec(), SizedQuery::new(query, None, None));
        let (elements, _) = self.drive.grove_get_raw_path_query(
            &path_query,
            self.transaction,
            QueryKeyElementPairResultType,
            &mut vec![],
            &self.platform_version.drive,
        )?;

        let mut found = BTreeSet::new();
        for (key, element) in elements.to_key_elements() {
            let kind = ElementKind::of(&element);
            let candidates = candidates(described, &key, kind);
            let Some(first) = candidates.first() else {
                report.violations.push(Violation::UndescribedKey {
                    layer: node.id.clone(),
                    path: path.to_vec(),
                    key,
                    kind,
                });
                continue;
            };

            // Try each candidate on a report of its own and keep the best fit
            let mut chosen = (*first, ConformanceReport::default());
            self.element(path, &key, kind, first, &mut chosen.1)?;
            for candidate in &candidates[1..] {
                if chosen.1.violations.is_empty() {
                    break;
                }
                let mut attempt = ConformanceReport::default();
                self.element(path, &key, kind, candidate, &mut attempt)?;
                if attempt.violations.len() < chosen.1.violations.len() {
                    chosen = (*candidate, attempt);
                }
            }
            found.insert(chosen.0.id.as_str());
            report.violations.append(&mut chosen.1.violations);
            report.visited.append(&mut chosen.1.visited);
        }

        for child in described {
            let required = matches!(child.key, KeySpec::Fixed { .. })
                && child.presence == Presence::Always
                && child.exists_in(protocol_version);
            if required && !found.contains(child.id.as_str()) {
                report.violations.push(Violation::MissingRequired {
                    node: child.id.clone(),
                    path: path.to_vec(),
                });
            }
        }
        Ok(())
    }

    /// Checks one element against the node taken to describe it, and
    /// everything below the element
    fn element(
        &self,
        path: &[Vec<u8>],
        key: &[u8],
        kind: ElementKind,
        node: &StructureNode,
        report: &mut ConformanceReport,
    ) -> Result<(), Error> {
        report.visited.insert(node.id.clone());
        if !node.exists_in(self.platform_version.protocol_version) {
            report.violations.push(Violation::OutsideItsVersions {
                node: node.id.clone(),
                since: node.since,
                until: node.until,
            });
        }
        if !node.kinds.contains(&kind) {
            report.violations.push(Violation::KindMismatch {
                node: node.id.clone(),
                path: path.to_vec(),
                expected: node.kinds.clone(),
                actual: kind,
            });
            return Ok(());
        }
        if kind.is_tree() && !kind.is_opaque() {
            let mut below = path.to_vec();
            below.push(key.to_vec());
            self.layer(&below, node, report)?;
        }
        Ok(())
    }
}

/// The nodes describing the layer below `node`, following `recurse`
fn children_of<'a>(root: &'a StructureNode, node: &'a StructureNode) -> &'a [StructureNode] {
    match node.recurse.as_deref().and_then(|target| root.find(target)) {
        Some(target) => &target.children,
        None => &node.children,
    }
}

/// The nodes that could describe an element, most likely first: the fixed key
/// if one matches, otherwise every template accepting the key. Templates that
/// list the element's kind come first, then templates for keys of a given
/// length before templates for any key.
fn candidates<'a>(
    described: &'a [StructureNode],
    key: &[u8],
    kind: ElementKind,
) -> Vec<&'a StructureNode> {
    if let Some(fixed) = described
        .iter()
        .find(|child| child.fixed_key_bytes() == Some(key))
    {
        return vec![fixed];
    }
    let mut templates: Vec<_> = described
        .iter()
        .filter_map(|child| match &child.key {
            KeySpec::Dynamic { matcher, .. } if matcher.accepts(key) => {
                Some((!child.kinds.contains(&kind), matcher.is_any(), child))
            }
            KeySpec::Root | KeySpec::Fixed { .. } | KeySpec::Dynamic { .. } => None,
        })
        .collect();
    templates.sort_by_key(|(wrong_kind, any_key, _)| (*wrong_kind, *any_key));
    // A template of another kind only matters when none lists the kind
    let listed = templates
        .iter()
        .filter(|(wrong_kind, ..)| !wrong_kind)
        .count();
    templates.truncate(listed.max(1));
    templates.into_iter().map(|(.., child)| child).collect()
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
