//! The grove path a ranked read / proof / verification is issued against.
//!
//! This is the **prover/verifier-agreement boundary** for the ranked
//! surface. Unlike the count and sum surfaces there is no `PathQuery` to
//! agree on — grovedb's indexed-axis envelope carries the query shape
//! itself and re-checks `(axis, k, descending)` at verification time — so
//! the entire shared contract is these path segments. Prover and verifier
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

impl DriveDocumentRankedQuery<'_> {
    /// Path of the **terminal property-name tree** — the indexed tree
    /// whose primary holds one value tree per group and whose per-axis
    /// secondaries hold the ranking.
    ///
    /// ```text
    /// [ RootTree::DataContractDocuments as u8 ]   // 0x01
    ///   / <contract_id: 32 bytes>
    ///   / [ 0x01 ]                                // "documents", not "contract"
    ///   / <document_type_name: utf-8>
    ///   / <index property name: utf-8>
    /// ```
    ///
    /// The children of that tree are the groups, keyed by the raw
    /// index-key bytes of the property value — the same bytes that come
    /// back as [`super::RankedEntry::key`].
    ///
    /// Errors when the index is not single-property. A compound ranked
    /// index would terminate one level *below* a prefix value tree, so
    /// its path would need the prefix property's value — which only a
    /// `where` clause could supply, and ranked queries take none. rs-dpp
    /// rejects compound ranked indexes at contract-parse time, so this is
    /// a fail-closed backstop for the day that grammar relaxes: better a
    /// typed error than a path pointing at a prefix level whose element
    /// is not an indexed tree at all.
    pub fn indexed_property_name_tree_path(&self) -> Result<Vec<Vec<u8>>, Error> {
        let [property] = self.index.properties.as_slice() else {
            return Err(Error::Drive(DriveError::NotSupported(
                "ranked queries require a single-property index: the ranked secondary \
                 lives on the index's terminal property-name tree, and for a compound \
                 index that tree sits under a prefix value tree whose value only a \
                 `where` clause could name — but ranked queries accept no `where` \
                 clauses",
            )));
        };
        Ok(vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
            property.name.as_bytes().to_vec(),
        ])
    }
}
