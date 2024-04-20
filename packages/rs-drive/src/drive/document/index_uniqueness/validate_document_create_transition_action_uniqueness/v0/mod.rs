use dpp::data_contract::DataContract;

use crate::drive::Drive;

use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::UniquenessOfDataRequest;
use crate::error::Error;

use dpp::data_contract::document_type::DocumentTypeRef;

use dpp::identifier::Identifier;

use dpp::validation::SimpleConsensusValidationResult;

use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::property_names::{
    CREATED_AT, CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, TRANSFERRED_AT,
    TRANSFERRED_AT_BLOCK_HEIGHT, TRANSFERRED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT,
    UPDATED_AT_BLOCK_HEIGHT, UPDATED_AT_CORE_BLOCK_HEIGHT,
};
use grovedb::TransactionArg;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;

impl Drive {
    /// Validate that a document create transition action would be unique in the state
    #[inline(always)]
    pub(super) fn validate_document_create_transition_action_uniqueness_v0(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document_create_transition: &DocumentCreateTransitionAction,
        owner_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let required_fields = document_type.required_fields();

        let is_created_at_required = required_fields.contains(CREATED_AT);
        let is_updated_at_required = required_fields.contains(UPDATED_AT);
        let is_transferred_at_required = required_fields.contains(TRANSFERRED_AT);

        let is_created_at_block_height_required = required_fields.contains(CREATED_AT_BLOCK_HEIGHT);
        let is_updated_at_block_height_required = required_fields.contains(UPDATED_AT_BLOCK_HEIGHT);
        let is_transferred_at_block_height_required =
            required_fields.contains(TRANSFERRED_AT_BLOCK_HEIGHT);

        let is_created_at_core_block_height_required =
            required_fields.contains(CREATED_AT_CORE_BLOCK_HEIGHT);
        let is_updated_at_core_block_height_required =
            required_fields.contains(UPDATED_AT_CORE_BLOCK_HEIGHT);
        let is_transferred_at_core_block_height_required =
            required_fields.contains(TRANSFERRED_AT_CORE_BLOCK_HEIGHT);

        let block_info = document_create_transition.block_info();

        let request = UniquenessOfDataRequest {
            contract,
            document_type,
            owner_id,
            document_id: document_create_transition.base().id(),
            allow_original: false,
            created_at: if is_created_at_required {
                Some(block_info.time_ms)
            } else {
                None
            },
            updated_at: if is_updated_at_required {
                Some(block_info.time_ms)
            } else {
                None
            },
            transferred_at: if is_transferred_at_required {
                Some(block_info.time_ms)
            } else {
                None
            },
            created_at_block_height: if is_created_at_block_height_required {
                Some(block_info.height)
            } else {
                None
            },
            updated_at_block_height: if is_updated_at_block_height_required {
                Some(block_info.height)
            } else {
                None
            },
            transferred_at_block_height: if is_transferred_at_block_height_required {
                Some(block_info.height)
            } else {
                None
            },
            created_at_core_block_height: if is_created_at_core_block_height_required {
                Some(block_info.core_height)
            } else {
                None
            },
            updated_at_core_block_height: if is_updated_at_core_block_height_required {
                Some(block_info.core_height)
            } else {
                None
            },
            transferred_at_core_block_height: if is_transferred_at_core_block_height_required {
                Some(block_info.core_height)
            } else {
                None
            },
            data: document_create_transition.data(),
        };
        self.validate_uniqueness_of_data(request, transaction, platform_version)
    }
}
