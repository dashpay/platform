//! Per-mode ranked executors on `impl Drive`. One file per response
//! shape — the dispatcher ([`super::drive_dispatcher`]) picks between
//! them on the request's `prove` flag.
//!
//! Each executor resolves the covering index, builds the
//! [`DriveDocumentRankedQuery`], and runs the matching method on it. The
//! resolution step is shared ([`ranked_query_for_mode`]) rather than
//! duplicated per file: the no-proof and prove paths must agree on the
//! index — and therefore the grove path — or a client would verify a
//! proof about a different subtree than the one an unproven read
//! returned.
//!
//! No re-exports needed: each file adds methods directly to `impl Drive`.

pub mod top_k_no_proof;
pub mod top_k_proof;

use super::index_picker::find_ranked_index_for_mode;
use super::{DocumentRankedMode, DriveDocumentRankedQuery};
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
use std::collections::BTreeMap;

/// Resolve a validated [`DocumentRankedMode`] against a document type's
/// indexes into the executable [`DriveDocumentRankedQuery`].
///
/// `indexes` is threaded in separately rather than read off
/// `document_type` here because
/// [`DocumentTypeV0Getters::indexes`] borrows its receiver — taking the
/// map from the caller lets the returned query's `&'a Index` outlive this
/// frame. Callers pass `document_type.indexes()`.
///
/// The only failure is "no index covers this", which is reported with the
/// exact contract keyword the index is missing so the caller can act on
/// it without reading the schema spec.
pub(super) fn ranked_query_for_mode<'a>(
    contract_id: [u8; 32],
    document_type: DocumentTypeRef<'a>,
    document_type_name: String,
    indexes: &'a BTreeMap<String, Index>,
    mode: &DocumentRankedMode,
) -> Result<DriveDocumentRankedQuery<'a>, Error> {
    let index = find_ranked_index_for_mode(indexes, mode).ok_or_else(|| {
        Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(format!(
            "no ranked index covers `group_by = [{}]` on the {:?} axis: the document type \
             needs a single-property index on `{}` declaring `{}`{}",
            mode.group_by_property,
            mode.axis,
            mode.group_by_property,
            mode.axis.required_index_keyword(),
            if mode.aggregate_field.is_empty() {
                String::new()
            } else {
                format!(" with `summable: \"{}\"`", mode.aggregate_field)
            }
        )))
    })?;
    Ok(DriveDocumentRankedQuery {
        document_type,
        contract_id,
        document_type_name,
        index,
        axis: mode.axis,
        descending: mode.descending,
        k: mode.k,
        offset: mode.offset,
    })
}
