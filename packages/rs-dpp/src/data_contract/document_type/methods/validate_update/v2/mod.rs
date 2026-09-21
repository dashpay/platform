//! Protocol v15 generation of document type update validation.
//!
//! v2 is v1 plus the keep-history document lifecycle. Two rules are new:
//!
//! * `canBeErased` is immutable in both directions. Widening it would give an
//!   operation over already-stored revisions to a type registered without it;
//!   narrowing it after a first erase chunk has irreversibly removed revisions
//!   would strand a partially erased, invisible document forever.
//! * A keep-history type may withdraw deletion (`canBeDeleted: true -> false`)
//!   while keeping history, and only in that direction, and only while it is
//!   not erasable: erase applies to deleted documents only and erasability is
//!   itself immutable, so an erasable type that stopped allowing deletion could
//!   never again reach a state erase acts on.
//!
//! Every other check is v1's, which v2 delegates to once the two lifecycle
//! rules above have passed. The one case where v1 alone would decide
//! differently, an erasable type withdrawing deletion, is refused here before
//! v1 sees it, so v1's option computation stays what it was.

use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV3Getters,
};
use crate::data_contract::document_type::DocumentTypeRef;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DocumentTypeRef<'_> {
    #[inline(always)]
    pub(super) fn validate_update_v2(
        &self,
        new_document_type: DocumentTypeRef,
        new_contract_version: u32,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if new_document_type.documents_can_be_erased() != self.documents_can_be_erased() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change whether its documents can be erased: changing from {} to {}",
                        self.documents_can_be_erased(),
                        new_document_type.documents_can_be_erased()
                    ),
                )
                .into(),
            ));
        }

        if self.documents_can_be_erased()
            && self.documents_can_be_deleted()
            && !new_document_type.documents_can_be_deleted()
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    "document type whose documents can be erased can not stop allowing deletion"
                        .to_string(),
                )
                .into(),
            ));
        }

        self.validate_update_v1(new_document_type, new_contract_version, platform_version)
    }
}
