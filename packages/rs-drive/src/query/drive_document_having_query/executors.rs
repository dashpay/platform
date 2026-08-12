//! Per-mode having-range executors on `impl Drive`, plus the shared
//! mode-to-query resolution. The dispatcher
//! ([`super::drive_dispatcher`]) picks between the two executors on the
//! request's `prove` flag.
//!
//! Index resolution reuses the ranked surface's covering-index picker
//! ([`find_ranked_index_for_axis`]) — both surfaces read the same
//! indexed tree, and sharing the picker is what guarantees a proof and
//! an unproven read are about the same subtree.

use super::super::drive_document_ranked_query::index_picker::find_ranked_index_for_axis;
use super::super::drive_document_ranked_query::RankedEntry;
use super::{DocumentHavingMode, DriveDocumentHavingQuery};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

/// Resolve a validated [`DocumentHavingMode`] against a document type's
/// indexes into the executable [`DriveDocumentHavingQuery`].
///
/// `indexes` is threaded in separately for the same lifetime reason as
/// the ranked resolver: the returned query's `&'a Index` must outlive
/// this frame. Callers pass `document_type.indexes()`.
pub(super) fn having_query_for_mode<'a>(
    contract_id: [u8; 32],
    document_type: DocumentTypeRef<'a>,
    document_type_name: String,
    indexes: &'a BTreeMap<String, Index>,
    mode: &DocumentHavingMode,
) -> Result<DriveDocumentHavingQuery<'a>, Error> {
    let axis = mode.bounds.axis();
    let index = find_ranked_index_for_axis(
        indexes,
        &mode.group_by_property,
        axis,
        &mode.aggregate_field,
    )
    .ok_or_else(|| {
        Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(format!(
            "no ranked index covers `group_by = [{}]` on the {:?} axis: a `having` bound \
             is served from that axis's pre-sorted secondary, so the document type needs \
             a single-property index on `{}` declaring `{}`{}",
            mode.group_by_property,
            axis,
            mode.group_by_property,
            axis.required_index_keyword(),
            if mode.aggregate_field.is_empty() {
                String::new()
            } else {
                format!(" with `summable: \"{}\"`", mode.aggregate_field)
            }
        )))
    })?;
    Ok(DriveDocumentHavingQuery {
        document_type,
        contract_id,
        document_type_name,
        index,
        bounds: mode.bounds,
        descending: mode.descending,
        limit: mode.limit,
    })
}

impl Drive {
    /// One page of groups matching a having bound, read without a proof.
    pub fn execute_document_having_range_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        mode: &DocumentHavingMode,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<RankedEntry>, Error> {
        let indexes = document_type.indexes();
        let having_query = having_query_for_mode(
            contract_id,
            document_type,
            document_type_name,
            indexes,
            mode,
        )?;
        having_query.execute_range_no_proof(self, transaction, platform_version)
    }

    /// Proof of one page of groups matching a having bound.
    ///
    /// The client verifies it with
    /// [`DriveDocumentHavingQuery::verify_having_range_proof`](crate::query::DriveDocumentHavingQuery::verify_having_range_proof),
    /// reconstructing the same query from the same contract — which is
    /// why index resolution is shared with the no-proof executor.
    pub fn execute_document_having_range_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        mode: &DocumentHavingMode,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let indexes = document_type.indexes();
        let having_query = having_query_for_mode(
            contract_id,
            document_type,
            document_type_name,
            indexes,
            mode,
        )?;
        having_query.execute_range_with_proof(self, transaction, platform_version)
    }
}
