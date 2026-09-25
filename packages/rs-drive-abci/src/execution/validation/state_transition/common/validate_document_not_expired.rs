use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::document::document_expired_error::DocumentExpiredError;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::validation::SimpleConsensusValidationResult;

/// Refuses an action on a document whose type declares a `ttl` once that has passed:
/// `$createdAt` plus the time to live at or before block time, the moment the cleanup after
/// the block's state transitions may delete it. The expiry is read from the document itself
/// and its type, never from the documents expirations tree, so it holds whether or not the
/// cleanup has reached the document yet.
///
/// Replacing, transferring, buying and repricing an expired document are refused, and so is
/// a moderator's restore of one, since the cleanup would delete it again in the same block.
/// Its owner's deletion is not: it only removes the document sooner.
///
/// Reachable from protocol version 14 only: `documents_ttl_seconds` is `Some` only on a
/// document type parsed from the `ttl` keyword, which earlier versions do not read.
///
/// `created_at` is the document's stored `$createdAt`, which every action on an existing
/// document carries unchanged.
pub(crate) fn validate_document_not_expired(
    contract_id: Identifier,
    document_type: DocumentTypeRef,
    document_id: Identifier,
    created_at: Option<TimestampMillis>,
    block_info: &BlockInfo,
) -> SimpleConsensusValidationResult {
    let Some(ttl_seconds) = document_type.documents_ttl_seconds() else {
        return SimpleConsensusValidationResult::new();
    };
    // The parser requires `$createdAt` on such a type and every write keeps it; a document
    // without one has no expiry to judge.
    let Some(created_at) = created_at else {
        return SimpleConsensusValidationResult::new();
    };
    let expired_at = created_at.saturating_add(u64::from(ttl_seconds) * 1000);
    if block_info.time_ms < expired_at {
        return SimpleConsensusValidationResult::new();
    }
    SimpleConsensusValidationResult::new_with_error(
        DocumentExpiredError::new(
            contract_id,
            document_type.name().clone(),
            document_id,
            expired_at,
            block_info.time_ms,
        )
        .into(),
    )
}
