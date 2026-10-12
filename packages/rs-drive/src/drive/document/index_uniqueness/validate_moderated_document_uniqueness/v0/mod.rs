use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::{
    UniquenessOfDataRequestUpdateType, UniquenessOfDataRequestV1,
};
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0Getters};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::borrow::Cow;
use std::collections::BTreeSet;

impl Drive {
    /// Validate that a document a moderator writes would be unique in the state
    ///
    /// A moderator's restore and field change reach this only from protocol version 14,
    /// whose table selects uniqueness generation 2, the one taking the V1 request. The
    /// removed document of a restore holds no index entry, so it is checked as a new one; a
    /// changed document keeps every system value, so only its changed data is.
    #[inline(always)]
    pub(super) fn validate_moderated_document_uniqueness_v0(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document: &Document,
        changed_fields: Option<&BTreeSet<String>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let update_type = match changed_fields {
            None => UniquenessOfDataRequestUpdateType::NewDocument,
            Some(changed_fields) => UniquenessOfDataRequestUpdateType::ChangedDocument {
                changed_owner_id: false,
                changed_updated_at: false,
                changed_transferred_at: false,
                changed_updated_at_block_height: false,
                changed_transferred_at_block_height: false,
                changed_updated_at_core_block_height: false,
                changed_transferred_at_core_block_height: false,
                changed_data_values: Cow::Borrowed(changed_fields),
            },
        };
        let request = UniquenessOfDataRequestV1 {
            contract,
            document_type,
            owner_id: document.owner_id(),
            creator_id: document.creator_id(),
            document_id: document.id(),
            created_at: document.created_at(),
            updated_at: document.updated_at(),
            transferred_at: document.transferred_at(),
            created_at_block_height: document.created_at_block_height(),
            updated_at_block_height: document.updated_at_block_height(),
            transferred_at_block_height: document.transferred_at_block_height(),
            created_at_core_block_height: document.created_at_core_block_height(),
            updated_at_core_block_height: document.updated_at_core_block_height(),
            transferred_at_core_block_height: document.transferred_at_core_block_height(),
            data: document.properties(),
            update_type,
        };
        self.validate_uniqueness_of_data(request.into(), transaction, platform_version)
    }
}
