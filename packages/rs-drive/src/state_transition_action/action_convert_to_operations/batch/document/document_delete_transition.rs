use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;

use crate::util::batch::DriveOperation::{DocumentOperation, IdentityOperation, TokenOperation};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};

use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::tokens::token_amount_on_contract_token::DocumentActionTokenEffect;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use dpp::version::PlatformVersion;
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};
use crate::error::drive::DriveError;
use crate::util::batch::drive_op_batch::TokenOperationType;

impl DriveHighLevelBatchOperationConverter for DocumentDeleteTransitionAction {
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
            .document_delete_transition
        {
            0 => {
                let base = self.base_owned();

                let contract_fetch_info = base.data_contract_fetch_info();

                let data_contract_id = base.data_contract_id();

                let identity_contract_nonce = base.identity_contract_nonce();

                let document_deletion_token_cost = base.token_cost();

                let mut ops = vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    }),
                    DocumentOperation(DocumentOperationType::DeleteDocument {
                        document_id: base.id(),
                        contract_info: DataContractInfo::DataContractFetchInfo(
                            base.data_contract_fetch_info(),
                        ),
                        document_type_info: DocumentTypeInfo::DocumentTypeName(
                            base.document_type_name_owned(),
                        ),
                    }),
                ];

                if let Some((token_id, effect, cost)) = document_deletion_token_cost {
                    match effect {
                        DocumentActionTokenEffect::TransferTokenToContractOwner => {
                            // If we are the owner, no need to send anything
                            if owner_id != contract_fetch_info.contract.owner_id() {
                                ops.push(TokenOperation(TokenOperationType::TokenTransfer {
                                    token_id,
                                    sender_id: owner_id,
                                    recipient_id: contract_fetch_info.contract.owner_id(),
                                    amount: cost,
                                }));
                            }
                        }
                        DocumentActionTokenEffect::BurnToken => {
                            ops.push(TokenOperation(TokenOperationType::TokenBurn {
                                token_id,
                                identity_balance_holder_id: owner_id,
                                burn_amount: cost,
                            }));
                        }
                    }
                }

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DocumentDeleteTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
