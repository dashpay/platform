use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::document::DriveHighLevelDocumentOperationConverter;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::document::documents_batch::document_transition::token_issuance_transition_action::{TokenIssuanceTransitionAction, TokenIssuanceTransitionActionAccessorsV0};
use crate::util::batch::{DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation::{IdentityOperation, TokenOperation};
use crate::util::object_size_info::DataContractInfo::DataContractFetchInfo;

impl DriveHighLevelDocumentOperationConverter for TokenIssuanceTransitionAction {
    fn into_high_level_document_drive_operations<'b>(
        mut self,
        _epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .token_issuance_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();

                let contract_fetch_info = self.base().data_contract_fetch_info();

                let identity_contract_nonce = self.base().identity_contract_nonce();

                let mut ops = vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )];

                ops.push(TokenOperation(TokenOperationType::TokenMint {
                    contract_info: DataContractFetchInfo(contract_fetch_info),
                    token_position: self.token_position(),
                    token_id: self.token_id(),
                    mint_amount: self.issuance_amount(),
                }));

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenIssuanceTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
