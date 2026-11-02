use dpp::consensus::ConsensusError;
use dpp::data_contract::document_type::DocumentReferenceKind;
use dpp::errors::consensus::state::document::referenced_document_type_deletable_error::ReferencedDocumentTypeDeletableError;
use dpp::errors::consensus::state::document::referenced_document_type_moderated_error::ReferencedDocumentTypeModeratedError;
use dpp::errors::consensus::state::document::referenced_document_type_not_deletable_error::ReferencedDocumentTypeNotDeletableError;
use dpp::errors::consensus::state::document::referenced_document_type_not_moderated_error::ReferencedDocumentTypeNotModeratedError;
use dpp::identifier::Identifier;

/// The error refusing a document reference declared as `declared` against a document type
/// admitting `admitted` ([`DocumentTypeV2Getters::document_reference_kind`]), `None` when they
/// agree. The three kinds are disjoint, so a declaration always states which guarantee the
/// reference carries:
/// - a `permanentDocument` reference to a type whose documents can leave state:
///   [`ReferencedDocumentTypeDeletableError`] (40122);
/// - a `deletableDocument` reference to a type whose documents never leave it:
///   [`ReferencedDocumentTypeNotDeletableError`] (40131);
/// - a `deletableDocument` reference to a type whose documents leave it only through a
///   moderator's recorded removal: [`ReferencedDocumentTypeModeratedError`] (40144);
/// - a `moderatedDocument` reference to any other type:
///   [`ReferencedDocumentTypeNotModeratedError`] (40143).
///
/// Shared by the registration check of a contract's declarations and the write-time check of
/// a referring document, which ask the same question of the same document type. Both are
/// generation 0 validators; the third kind and its two errors exist from protocol version 14,
/// the first whose parser produces a document reference at all, so the answer for the two
/// kinds that existed before it is unchanged.
///
/// [`DocumentTypeV2Getters::document_reference_kind`]: dpp::data_contract::document_type::accessors::DocumentTypeV2Getters::document_reference_kind
pub(crate) fn document_reference_kind_mismatch(
    declared: DocumentReferenceKind,
    admitted: DocumentReferenceKind,
    contract_id: Identifier,
    document_type_name: &str,
    path: &str,
) -> Option<ConsensusError> {
    let (contract_id, document_type_name, path) = (
        contract_id,
        document_type_name.to_string(),
        path.to_string(),
    );
    match (declared, admitted) {
        (declared, admitted) if declared == admitted => None,
        (DocumentReferenceKind::Permanent, _) => Some(
            ReferencedDocumentTypeDeletableError::new(contract_id, document_type_name, path).into(),
        ),
        (DocumentReferenceKind::Deletable, DocumentReferenceKind::Permanent) => Some(
            ReferencedDocumentTypeNotDeletableError::new(contract_id, document_type_name, path)
                .into(),
        ),
        (DocumentReferenceKind::Deletable, _) => Some(
            ReferencedDocumentTypeModeratedError::new(contract_id, document_type_name, path).into(),
        ),
        (DocumentReferenceKind::Moderated, _) => Some(
            ReferencedDocumentTypeNotModeratedError::new(contract_id, document_type_name, path)
                .into(),
        ),
    }
}
