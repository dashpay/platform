use std::borrow::Cow;
use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use dpp::tokens::token_order_side::TokenOrderSide;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_fill_ask_transition_action::{TokenFillAskTransitionAction, TokenFillAskTransitionActionAccessorsV0};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation::{DocumentOperation, IdentityOperation, TokenOperation};
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;

impl DriveHighLevelBatchOperationConverter for TokenFillAskTransitionAction {
    /// Moves the amount the fill takes from the listing's escrow to the buyer and the credits it
    /// costs from the buyer to the seller. A partial fill rewrites the listing's `ask` document;
    /// a fill that takes everything open deletes it, and its storage goes back to the seller.
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
            .token_fill_ask_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();
                let identity_contract_nonce = self.base().identity_contract_nonce();
                let order = self.order();
                let maker_id = order.maker_id;
                let order_id = order.id;
                let credits = self.credits();

                let mut ops = vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    }),
                    TokenOperation(TokenOperationType::TokenReleaseFromOrderEscrow {
                        token_id: self.token_id(),
                        order_id,
                        recipient_id: owner_id,
                        amount: self.fill_amount(),
                        remaining_after: self.remaining_after(),
                    }),
                ];

                if credits > 0 {
                    ops.push(IdentityOperation(
                        IdentityOperationType::RemoveFromIdentityBalance {
                            identity_id: owner_id.to_buffer(),
                            balance_to_remove: credits,
                        },
                    ));
                    ops.push(IdentityOperation(
                        IdentityOperationType::AddToIdentityBalance {
                            identity_id: maker_id.to_buffer(),
                            added_balance: credits,
                        },
                    ));
                }

                let contract_info =
                    DataContractInfo::DataContractFetchInfo(self.token_market_contract().clone());
                let document_type_info = DocumentTypeInfo::DocumentTypeNameAsStr(
                    TokenOrderSide::Ask.document_type_name(),
                );
                match self.updated_order_document() {
                    Some(updated_document) => {
                        let storage_flags =
                            StorageFlags::new_single_epoch(epoch.index, Some(maker_id.to_buffer()));
                        ops.push(DocumentOperation(DocumentOperationType::UpdateDocument {
                            owned_document_info: OwnedDocumentInfo {
                                document_info: DocumentOwnedInfo((
                                    updated_document.clone(),
                                    Some(Cow::Owned(storage_flags)),
                                )),
                                owner_id: Some(maker_id.into_buffer()),
                            },
                            contract_info,
                            document_type_info,
                        }));
                    }
                    None => {
                        ops.push(DocumentOperation(DocumentOperationType::DeleteDocument {
                            document_id: order_id,
                            contract_info,
                            document_type_info,
                        }));
                    }
                }

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenFillAskTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
