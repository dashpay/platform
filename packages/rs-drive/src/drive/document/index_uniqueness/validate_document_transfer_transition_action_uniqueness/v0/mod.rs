use dpp::data_contract::DataContract;

use crate::drive::Drive;

use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::UniquenessOfDataRequest;
use crate::error::Error;

use dpp::data_contract::document_type::DocumentTypeRef;

use dpp::identifier::Identifier;

use dpp::validation::SimpleConsensusValidationResult;

use dpp::document::DocumentV0Getters;
use grovedb::TransactionArg;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_transfer_transition_action::{DocumentTransferTransitionAction, DocumentTransferTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;

impl Drive {
    /// Validate that a document transfer transition action would be unique in the state
    #[inline(always)]
    pub(super) fn validate_document_transfer_transition_action_uniqueness_v0(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document_transfer_transition: &DocumentTransferTransitionAction,
        owner_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let request = UniquenessOfDataRequest {
            contract,
            document_type,
            owner_id,
            document_id: document_transfer_transition.base().id(),
            allow_original: true,
            created_at: document_transfer_transition.document().created_at(),
            updated_at: document_transfer_transition.document().updated_at(),
            transferred_at: document_transfer_transition.document().transferred_at(),
            created_at_block_height: document_transfer_transition
                .document()
                .created_at_block_height(),
            updated_at_block_height: document_transfer_transition
                .document()
                .updated_at_block_height(),
            transferred_at_block_height: document_transfer_transition
                .document()
                .transferred_at_block_height(),
            created_at_core_block_height: document_transfer_transition
                .document()
                .created_at_core_block_height(),
            updated_at_core_block_height: document_transfer_transition
                .document()
                .updated_at_core_block_height(),
            transferred_at_core_block_height: document_transfer_transition
                .document()
                .transferred_at_core_block_height(),
            data: document_transfer_transition.document().properties(),
        };
        self.validate_uniqueness_of_data(request, transaction, platform_version)
    }
}
