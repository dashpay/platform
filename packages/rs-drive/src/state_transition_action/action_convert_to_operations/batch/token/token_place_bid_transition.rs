use std::borrow::Cow;
use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use dpp::tokens::token_order_side::TokenOrderSide;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_place_bid_transition_action::{TokenPlaceBidTransitionAction, TokenPlaceBidTransitionActionAccessorsV0};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation::{DocumentOperation, IdentityOperation, TokenOperation};
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;

impl DriveHighLevelBatchOperationConverter for TokenPlaceBidTransitionAction {
    /// Moves what the offer is worth from the buyer's balance into the offer's escrow and records
    /// the offer as a `bid` document of the token market contract, owned and paid for by the
    /// buyer, who gets its storage back when it closes.
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
            .token_place_bid_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();
                let identity_contract_nonce = self.base().identity_contract_nonce();
                let order_id = self.order_id();
                let escrow = self.escrow();
                let document = self.order_document().clone();
                let storage_flags =
                    StorageFlags::new_single_epoch(epoch.index, Some(owner_id.to_buffer()));

                let ops = vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    }),
                    IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
                        identity_id: owner_id.to_buffer(),
                        balance_to_remove: escrow,
                    }),
                    TokenOperation(TokenOperationType::TokenHoldCreditsInOrderEscrow {
                        order_id,
                        credits: escrow,
                    }),
                    DocumentOperation(DocumentOperationType::AddDocument {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentOwnedInfo((
                                document,
                                Some(Cow::Owned(storage_flags)),
                            )),
                            owner_id: Some(owner_id.into_buffer()),
                        },
                        contract_info: DataContractInfo::DataContractFetchInfo(
                            self.token_market_contract().clone(),
                        ),
                        document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr(
                            TokenOrderSide::Bid.document_type_name(),
                        ),
                        override_document: false,
                    }),
                ];

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenPlaceBidTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
