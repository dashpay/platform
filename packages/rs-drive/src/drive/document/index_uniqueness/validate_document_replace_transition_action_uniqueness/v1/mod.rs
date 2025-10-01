use dpp::data_contract::DataContract;
use std::borrow::Cow;

use crate::drive::Drive;

use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::{
    UniquenessOfDataRequestUpdateType, UniquenessOfDataRequestV1,
};
use crate::error::Error;

use dpp::data_contract::document_type::DocumentTypeRef;

use dpp::identifier::Identifier;

use dpp::validation::SimpleConsensusValidationResult;

use grovedb::TransactionArg;

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;

impl Drive {
    /// Validate that a document replace transition action would be unique in the state
    #[inline(always)]
    pub(super) fn validate_document_replace_transition_action_uniqueness_v1(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document_replace_transition: &DocumentReplaceTransitionAction,
        owner_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let request = UniquenessOfDataRequestV1 {
            contract,
            document_type,
            owner_id,
            creator_id: document_replace_transition.creator_id(),
            document_id: document_replace_transition.base().id(),
            created_at: document_replace_transition.created_at(),
            updated_at: document_replace_transition.updated_at(),
            transferred_at: document_replace_transition.transferred_at(),
            created_at_block_height: document_replace_transition.created_at_block_height(),
            updated_at_block_height: document_replace_transition.updated_at_block_height(),
            transferred_at_block_height: document_replace_transition.transferred_at_block_height(),
            created_at_core_block_height: document_replace_transition
                .created_at_core_block_height(),
            updated_at_core_block_height: document_replace_transition
                .updated_at_core_block_height(),
            transferred_at_core_block_height: document_replace_transition
                .transferred_at_core_block_height(),
            data: document_replace_transition.data(),
            update_type: UniquenessOfDataRequestUpdateType::ChangedDocument {
                changed_owner_id: false,
                changed_updated_at: true,
                changed_transferred_at: false,
                changed_updated_at_block_height: true,
                changed_transferred_at_block_height: false,
                changed_updated_at_core_block_height: true,
                changed_transferred_at_core_block_height: false,
                changed_data_values: Cow::Borrowed(
                    document_replace_transition.changed_data_fields(),
                ),
            },
        };
        self.validate_uniqueness_of_data(request.into(), transaction, platform_version)
    }
}
