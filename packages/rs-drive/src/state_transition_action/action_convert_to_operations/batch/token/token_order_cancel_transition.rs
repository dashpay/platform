use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_cancel_transition_action::action::TokenOrderCancelTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_cancel_transition_action::TokenOrderCancelTransitionActionAccessorsV0;
use crate::util::batch::{DriveOperation, IdentityOperationType};
use crate::util::batch::DriveOperation::{IdentityOperation};

impl DriveHighLevelBatchOperationConverter for TokenOrderCancelTransitionAction {
    fn into_high_level_batch_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .token_mint_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();

                let identity_contract_nonce = self.base().identity_contract_nonce();

                let mut ops = vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )];

                // TODO: Token Event?

                // TODO: Add operation

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method:
                    "TokenOrderCancelTransitionAction::into_high_level_document_drive_operations"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
