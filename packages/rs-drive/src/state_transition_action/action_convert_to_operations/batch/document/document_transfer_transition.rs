use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::util::batch::DriveOperation::{DocumentOperation, IdentityOperation, TokenOperation};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::epoch::Epoch;

use dpp::document::DocumentV0Getters;
use dpp::prelude::Identifier;
use std::borrow::Cow;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::tokens::token_amount_on_contract_token::DocumentActionTokenEffect;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::{DocumentTransferTransitionAction, DocumentTransferTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::util::batch::drive_op_batch::TokenOperationType;

impl DriveHighLevelBatchOperationConverter for DocumentTransferTransitionAction {
    fn into_high_level_batch_drive_operations<'b>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .document_transfer_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();
                let document_type_name = self.base().document_type_name().clone();
                let identity_contract_nonce = self.base().identity_contract_nonce();
                let contract_fetch_info = self.base().data_contract_fetch_info();
                let contract_owner_id = contract_fetch_info.contract.owner_id();

                let document_transfer_token_cost = self.base().token_cost();
                let document = self.document_owned();

                // we are transferring the document so the new storage flags should be on the new owner

                let new_document_owner_id = document.owner_id();

                let storage_flags = StorageFlags::new_single_epoch(
                    epoch.index,
                    Some(new_document_owner_id.to_buffer()),
                );

                let mut ops = vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    }),
                    DocumentOperation(DocumentOperationType::UpdateDocument {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentOwnedInfo((
                                document,
                                Some(Cow::Owned(storage_flags)),
                            )),
                            owner_id: Some(new_document_owner_id.into_buffer()),
                        },
                        contract_info: DataContractInfo::DataContractFetchInfo(contract_fetch_info),
                        document_type_info: DocumentTypeInfo::DocumentTypeName(document_type_name),
                    }),
                ];

                if let Some((token_id, effect, cost)) = document_transfer_token_cost {
                    match effect {
                        DocumentActionTokenEffect::TransferTokenToContractOwner => {
                            // If we are the owner, no need to send anything
                            if owner_id != contract_owner_id {
                                ops.push(TokenOperation(TokenOperationType::TokenTransfer {
                                    token_id,
                                    sender_id: owner_id,
                                    recipient_id: contract_owner_id,
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
                method:
                    "DocumentTransferTransitionAction::into_high_level_document_drive_operations"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
