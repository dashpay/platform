use dpp::consensus::basic::document::{InvalidDocumentTransitionActionError, InvalidDocumentTypeError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::DocumentV0Getters;
use dpp::nft::TradeMode;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::{DocumentPurchaseTransitionAction, DocumentPurchaseTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentPurchaseTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentPurchaseTransitionActionStructureValidationV0 for DocumentPurchaseTransitionAction {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let data_contract = &contract_fetch_info.contract;
        let document_type_name = self.base().document_type_name();

        // Make sure that the document type is defined in the contract
        let Some(document_type) = data_contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), data_contract.id())
                    .into(),
            ));
        };

        // We can not purchase from ourselves
        // The document owner id is already our owner id, as the action we want to take is to
        //  insert this document into the state (with our owner id)
        if self.original_owner_id() == self.document().owner_id() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "on document type: {} identity trying to purchase a document that is already owned by the purchaser",
                    document_type_name
                ))
                    .into(),
            ));
        }

        if document_type.trade_mode() != TradeMode::DirectPurchase {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "{} trade mode is not direct purchase but we are trying to purchase directly",
                    document_type_name
                ))
                .into(),
            ));
        }

        // Added in place at protocol version 14, inert for every earlier version this
        // generation serves: their meta-schemas refuse `distinctFrom`, their parser ignores
        // it (`apply_distinct_from` is `None`), and `validate_distinct_from` is `None` there,
        // so the call sees no declaration and returns an empty result. From 14, the
        // document changes owner and a `distinctFrom: $ownerId` property of the stored
        // document must differ from the new owner, which the action already carries on the
        // document. The data was schema-validated when it was written, so every value
        // compared is a 32-byte identifier.
        document_type
            .validate_distinct_from_properties(
                self.document().properties(),
                self.document().owner_id(),
                platform_version,
            )
            .map_err(Error::Protocol)
    }
}
