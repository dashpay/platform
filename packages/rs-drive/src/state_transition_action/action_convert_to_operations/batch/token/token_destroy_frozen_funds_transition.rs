use dpp::block::epoch::Epoch;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action::v0::GroupActionV0;
use dpp::group::GroupStateTransitionResolvedInfo;
use dpp::identifier::Identifier;
use dpp::tokens::token_event::TokenEvent;
use dpp::tokens::token_order::TokenOrder;
use dpp::tokens::token_order_side::TokenOrderSide;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::{TokenDestroyFrozenFundsTransitionAction, TokenDestroyFrozenFundsTransitionActionAccessorsV0};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};
use crate::util::batch::drive_op_batch::{GroupOperationType, TokenOperationType};
use crate::util::batch::DriveOperation::{DocumentOperation, GroupOperation, IdentityOperation, TokenOperation};

impl DriveHighLevelBatchOperationConverter for TokenDestroyFrozenFundsTransitionAction {
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
            .token_destroy_frozen_funds_transition
        {
            0 => self.destroy_frozen_funds_operations(owner_id, false),
            // From protocol version 15: also destroys what the frozen identity's open listings
            // hold in escrow, and deletes their `ask` documents.
            1 => self.destroy_frozen_funds_operations(owner_id, true),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenDestroyFrozenFundsTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

impl TokenDestroyFrozenFundsTransitionAction {
    /// The operations of a destroy of frozen funds. Without `destroy_listings` (generation 0)
    /// the frozen balance alone is burned, which is all there is before the marketplace. With it
    /// (generation 1) the frozen identity's open listings the transformer read are destroyed
    /// too: what they hold in escrow leaves the supply with the balance, their `ask` documents
    /// are deleted, and the recorded amount is everything destroyed.
    fn destroy_frozen_funds_operations<'b>(
        self,
        owner_id: Identifier,
        destroy_listings: bool,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        let data_contract_id = self.base().data_contract_id();

        let identity_contract_nonce = self.base().identity_contract_nonce();

        let listings: Vec<TokenOrder> = if destroy_listings {
            self.escrowed_listings().to_vec()
        } else {
            vec![]
        };
        let destroyed_amount = if listings.is_empty() {
            self.amount()
        } else {
            self.total_destroyed()
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a frozen holding is more tokens than a u64 holds",
                )))?
        };

        let mut ops = vec![IdentityOperation(
            IdentityOperationType::UpdateIdentityContractNonce {
                identity_id: owner_id.into_buffer(),
                contract_id: data_contract_id.into_buffer(),
                nonce: identity_contract_nonce,
            },
        )];

        if let Some(GroupStateTransitionResolvedInfo {
            group_contract_position,
            action_id,
            action_is_proposer,
            signer_power,
            ..
        }) = self.base().store_in_group()
        {
            let event = TokenEvent::DestroyFrozenFunds(
                self.frozen_identity_id(),
                destroyed_amount,
                self.public_note().cloned(),
            );

            let initialize_with_insert_action_info = if *action_is_proposer {
                Some(GroupAction::V0(GroupActionV0 {
                    contract_id: self.base().data_contract_id(),
                    proposer_id: owner_id,
                    token_contract_position: self.base().token_position(),
                    event: GroupActionEvent::TokenEvent(event),
                }))
            } else {
                None
            };

            ops.push(GroupOperation(GroupOperationType::AddGroupAction {
                contract_id: data_contract_id,
                group_contract_position: *group_contract_position,
                initialize_with_insert_action_info,
                action_id: *action_id,
                signer_identity_id: owner_id,
                signer_power: *signer_power,
                closes_group_action: self.base().perform_action(),
            }));
        }

        if self.base().perform_action() {
            if listings.is_empty() {
                ops.push(TokenOperation(TokenOperationType::TokenBurn {
                    token_id: self.token_id(),
                    identity_balance_holder_id: self.frozen_identity_id(),
                    burn_amount: self.amount(),
                }));
            } else {
                ops.push(TokenOperation(
                    TokenOperationType::TokenDestroyFrozenHolding {
                        token_id: self.token_id(),
                        holder_id: self.frozen_identity_id(),
                        balance_amount: self.amount(),
                        listings: listings
                            .iter()
                            .map(|listing| (listing.id, listing.remaining))
                            .collect(),
                    },
                ));
                let token_market_contract = self.token_market_contract().cloned().ok_or(
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "listings to destroy come with the token market contract",
                    )),
                )?;
                for listing in &listings {
                    ops.push(DocumentOperation(DocumentOperationType::DeleteDocument {
                        document_id: listing.id,
                        contract_info: DataContractInfo::DataContractFetchInfo(
                            token_market_contract.clone(),
                        ),
                        document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr(
                            TokenOrderSide::Ask.document_type_name(),
                        ),
                    }));
                }
            }

            ops.push(TokenOperation(TokenOperationType::TokenHistory {
                token_id: self.token_id(),
                owner_id,
                nonce: identity_contract_nonce,
                event: TokenEvent::DestroyFrozenFunds(
                    self.frozen_identity_id(),
                    destroyed_amount,
                    self.public_note_owned(),
                ),
            }));
        }

        Ok(ops)
    }
}
