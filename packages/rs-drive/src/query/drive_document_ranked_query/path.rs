//! The grove path a ranked read / proof / verification is issued against.
//!
//! This is one half of the **prover/verifier-agreement boundary** for
//! the ranked surface: both sides build the same axis `PathQuery` —
//! these path segments plus the traversal (`axis, k, offset,
//! descending`) — and grovedb re-executes the proof against that
//! reconstruction at verification time. Prover and verifier
//! both call [`DriveDocumentRankedQuery::indexed_property_name_tree_path`];
//! a divergence here surfaces as a failed root-hash reconstruction, not a
//! wrong answer, but it is still the one place the two sides must not
//! drift.
//!
//! Gated `any(server, verify)` so the verifier crate reaches it through
//! `DriveDocumentRankedQuery::*` method syntax.

use super::DriveDocumentRankedQuery;
use crate::drive::RootTree;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::{Index, IndexProperty};

/// The ranked level a request with `pin_count` pins addresses, split
/// into the leading properties (one pin each) and the ranked property
/// itself. An index can host secondaries at up to TWO levels — its
/// `rankedCountable.at` property and, independently, its terminal
/// property (the boolean axes) — so the level cannot be derived from
/// the index alone: the request's pin count names it, since every
/// property before the ranked level must be pinned and none after it
/// may appear. Fails closed when the pin count lands on a level that
/// hosts no secondary.
///
/// Shared by the path builder and the prefix encoder — and, through
/// them, by the server executors and the SDK verifier — so both sides
/// agree on where a request's secondary lives without re-deriving it.
pub(crate) fn ranked_level_split(
    index: &Index,
    pin_count: usize,
) -> Result<(&[IndexProperty], &IndexProperty), Error> {
    let Some(ranked_property) = index.properties.get(pin_count) else {
        return Err(Error::Drive(DriveError::NotSupported(
            "ranked and having-range queries require exactly one pinned value per property \
             before the ranked level — the resolved pin set addresses no index property",
        )));
    };
    let is_at_level = index.ranked_countable_at.as_deref() == Some(ranked_property.name.as_str());
    let is_terminal = pin_count + 1 == index.properties.len();
    if !is_at_level && !is_terminal {
        return Err(Error::Drive(DriveError::NotSupported(
            "ranked and having-range queries must land on a level hosting a ranking \
             secondary — the index's rankedCountable.at property or its terminal property; \
             the resolved pin set addresses an intermediate level",
        )));
    }
    Ok((&index.properties[..pin_count], ranked_property))
}

/// Path of an index's **ranked-level** property-name tree — the terminal
/// one for the boolean ranking axes, the `at` level for a prefix-level
/// `rankedCountable`. Shared by the ranked and having-range query
/// surfaces, which read the same indexed tree. See
/// [`DriveDocumentRankedQuery::indexed_property_name_tree_path`]
/// for the segment layout.
///
/// The branch's prefix segments carry the **encoded index-key bytes** of
/// each leading property's pinned value, in index-property order — one
/// per property before the ranked one. Empty for an index ranked at its
/// first property. The arity must match exactly: the ranked level's
/// tree sits under one prefix value tree per leading property, and only
/// a `where` pin (an equality, or one element of the single permitted
/// `IN`) can name those values, so a missing or surplus value means the
/// caller resolved the wrong index — a typed error, not a guess.
pub(crate) fn indexed_property_name_tree_path_for_index(
    contract_id: &[u8; 32],
    document_type_name: &str,
    index: &Index,
    equality_prefix_values: &[Vec<u8>],
) -> Result<Vec<Vec<u8>>, Error> {
    // The pin count names the ranked level (see `ranked_level_split`),
    // which also makes the leading-property arity match by construction;
    // the split's fail-closed error replaces the old arity backstop.
    let (leading_properties, terminal_property) =
        ranked_level_split(index, equality_prefix_values.len())?;
    let mut path = Vec::with_capacity(5 + 2 * leading_properties.len());
    path.push(vec![RootTree::DataContractDocuments as u8]);
    path.push(contract_id.to_vec());
    path.push(vec![1u8]);
    path.push(document_type_name.as_bytes().to_vec());
    for (property, value) in leading_properties.iter().zip(equality_prefix_values) {
        path.push(property.name.as_bytes().to_vec());
        path.push(value.clone());
    }
    path.push(terminal_property.name.as_bytes().to_vec());
    Ok(path)
}

impl DriveDocumentRankedQuery<'_> {
    /// Path of the **terminal property-name tree** — the indexed tree
    /// whose primary holds one value tree per group and whose per-axis
    /// secondaries hold the ranking.
    ///
    /// For a single-property index:
    ///
    /// ```text
    /// [ RootTree::DataContractDocuments as u8 ]   // 0x01
    ///   / <contract_id: 32 bytes>
    ///   / [ 0x01 ]                                // "documents", not "contract"
    ///   / <document_type_name: utf-8>
    ///   / <index property name: utf-8>
    /// ```
    ///
    /// For a compound index `[p1, …, pn]`, each leading property
    /// contributes two segments — its name and the **encoded index-key
    /// bytes of its pinned value** (from
    /// [`Self::prefix_branches`]) — and the terminal property
    /// name closes the path:
    ///
    /// ```text
    ///   … / <p1 name> / <p1 pinned value bytes> / … / <pn name>
    /// ```
    ///
    /// so the ranking read lands on **that prefix's** indexed tree: the
    /// per-prefix secondary orders only the pinned prefix's groups.
    ///
    /// The children of the terminal tree are the groups, keyed by the
    /// raw index-key bytes of the terminal property value — the same
    /// bytes that come back as [`super::RankedEntry::key`].
    ///
    /// Errors when the number of encoded prefix values does not match
    /// the index's leading-property count — the fail-closed backstop
    /// for a caller that resolved the query against the wrong index.
    ///
    /// `branch` indexes into [`Self::prefix_branches`]; single-branch
    /// queries (no `IN` pin) always pass `0`.
    pub fn indexed_property_name_tree_path(&self, branch: usize) -> Result<Vec<Vec<u8>>, Error> {
        let prefix_values =
            self.prefix_branches
                .get(branch)
                .ok_or(Error::Drive(DriveError::NotSupported(
                    "ranked and having-range queries addressed a prefix branch outside the \
                 query's resolved branch set",
                )))?;
        indexed_property_name_tree_path_for_index(
            &self.contract_id,
            &self.document_type_name,
            self.index,
            prefix_values,
        )
    }
}
