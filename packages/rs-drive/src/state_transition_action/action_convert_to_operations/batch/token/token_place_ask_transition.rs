use std::borrow::Cow;
use dpp::block::epoch::Epoch;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Getters;
use dpp::tokens::token_event::TokenEvent;
use dpp::identifier::Identifier;
use dpp::tokens::token_order_side::TokenOrderSide;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_place_ask_transition_action::{TokenPlaceAskTransitionAction, TokenPlaceAskTransitionActionAccessorsV0};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation::{DocumentOperation, IdentityOperation, TokenOperation};
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;

impl DriveHighLevelBatchOperationConverter for TokenPlaceAskTransitionAction {
    /// Moves the listed tokens from the seller's balance into the listing's escrow and records
    /// the listing as an `ask` document of the token market contract, owned and paid for by the
    /// seller, who gets its storage back when it closes.
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
            .token_place_ask_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();
                let identity_contract_nonce = self.base().identity_contract_nonce();
                let token_id = self.token_id();
                let order_id = self.order_id();
                let amount = self.amount();
                let document = self.order_document().clone();
                let storage_flags =
                    StorageFlags::new_single_epoch(epoch.index, Some(owner_id.to_buffer()));

                let mut ops = vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    }),
                    TokenOperation(TokenOperationType::TokenPlaceInOrderEscrow {
                        token_id,
                        holder_id: owner_id,
                        order_id,
                        amount,
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
                            TokenOrderSide::Ask.document_type_name(),
                        ),
                        override_document: false,
                    }),
                ];

                // The marketplace's events are a token's transfers, recorded when it keeps transfer
                // history.
                if self
                    .base()
                    .token_configuration()?
                    .keeps_history()
                    .keeps_transfer_history()
                {
                    ops.push(TokenOperation(TokenOperationType::TokenHistory {
                        token_id: self.token_id(),
                        owner_id,
                        nonce: identity_contract_nonce,
                        event: TokenEvent::OrderPlaced(
                            self.order_id(),
                            TokenOrderSide::Ask,
                            self.amount(),
                            self.price(),
                        ),
                    }));
                }

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenPlaceAskTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
