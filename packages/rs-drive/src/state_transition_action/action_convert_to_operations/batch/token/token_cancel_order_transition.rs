use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use dpp::tokens::token_order_side::TokenOrderSide;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_cancel_order_transition_action::{TokenCancelOrderTransitionAction, TokenCancelOrderTransitionActionAccessorsV0};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation::{DocumentOperation, IdentityOperation, TokenOperation};
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};

impl DriveHighLevelBatchOperationConverter for TokenCancelOrderTransitionAction {
    /// Gives the maker back what the order holds in escrow and deletes its document, whose
    /// storage goes back to the maker.
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
            .token_cancel_order_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();
                let identity_contract_nonce = self.base().identity_contract_nonce();
                let order = self.order();
                let side = order.side;

                let mut ops = vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )];

                match side {
                    TokenOrderSide::Ask => {
                        ops.push(TokenOperation(
                            TokenOperationType::TokenReleaseFromOrderEscrow {
                                token_id: self.token_id(),
                                order_id: order.id,
                                recipient_id: owner_id,
                                amount: order.remaining,
                                remaining_after: 0,
                            },
                        ));
                    }
                    TokenOrderSide::Bid => {
                        return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                            "an offer cannot be cancelled before offers can be placed",
                        )));
                    }
                }

                ops.push(DocumentOperation(DocumentOperationType::DeleteDocument {
                    document_id: order.id,
                    contract_info: DataContractInfo::DataContractFetchInfo(
                        self.token_market_contract().clone(),
                    ),
                    document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr(
                        side.document_type_name(),
                    ),
                }));

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method:
                    "TokenCancelOrderTransitionAction::into_high_level_document_drive_operations"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
