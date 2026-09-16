use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_to_pool_transition_action::{TokenDirectPurchaseToPoolTransitionAction, TokenDirectPurchaseToPoolTransitionActionAccessorsV0};
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation::{IdentityOperation, TokenOperation};
use crate::util::batch::{DriveOperation, IdentityOperationType};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use platform_version::version::PlatformVersion;

impl DriveHighLevelBatchOperationConverter for TokenDirectPurchaseToPoolTransitionAction {
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
            .token_direct_purchase_to_pool_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();
                let identity_contract_nonce = self.base().identity_contract_nonce();
                let token_id = self.token_id();

                let mut ops = vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )];

                ops.push(TokenOperation(TokenOperationType::TokenMintToPool {
                    token_id,
                    amount: self.token_count(),
                    allow_first_mint: true,
                    notes: self.notes(),
                }));

                let contract_owner_id = self.base().data_contract_fetch_info().contract.owner_id();
                if owner_id != contract_owner_id {
                    ops.push(IdentityOperation(
                        IdentityOperationType::RemoveFromIdentityBalance {
                            identity_id: owner_id.to_buffer(),
                            balance_to_remove: self.total_agreed_price(),
                        },
                    ));
                    ops.push(IdentityOperation(
                        IdentityOperationType::AddToIdentityBalance {
                            identity_id: contract_owner_id.to_buffer(),
                            added_balance: self.total_agreed_price(),
                        },
                    ));
                }

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenDirectPurchaseToPoolTransitionAction::into_high_level_batch_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
