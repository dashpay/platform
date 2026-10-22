use dpp::block::epoch::Epoch;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action::v0::GroupActionV0;
use dpp::group::GroupStateTransitionResolvedInfo;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;
use dpp::tokens::token_event::TokenEvent;
use dpp::tokens::token_order_side::TokenOrderSide;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::{TokenDestroyFrozenFundsTransitionAction, TokenDestroyFrozenFundsTransitionActionAccessorsV0};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::{GroupOperationType, TokenOperationType};
use crate::util::batch::DriveOperation::{DocumentOperation, GroupOperation, IdentityOperation, TokenOperation};
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};

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

                if let Some(GroupStateTransitionResolvedInfo {
                    group_contract_position,
                    action_id,
                    action_is_proposer,
                    signer_power,
                    ..
                }) = self.base().store_in_group()
                {
                    let event =
                        TokenEvent::DestroyFrozenFunds(self.frozen_identity_id(), self.amount(), self.public_note().cloned());

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
                    ops.push(TokenOperation(TokenOperationType::TokenBurn {
                        token_id: self.token_id(),
                        identity_balance_holder_id: self.frozen_identity_id(),
                        burn_amount: self.amount(),
                    }));

                    ops.push(TokenOperation(TokenOperationType::TokenHistory {
                        token_id: self.token_id(),
                        owner_id,
                        nonce: identity_contract_nonce,
                        event: TokenEvent::DestroyFrozenFunds(
                            self.frozen_identity_id(),
                            self.amount(),
                            self.public_note_owned(),
                        ),
                    }));
                }

                Ok(ops)
            }
            // From protocol version 15: also destroys what the frozen identity's open listings
            // hold in escrow, and deletes their `ask` documents.
            1 => self.destroy_frozen_funds_and_listings_operations_v1(owner_id),
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
    /// Generation 1 (protocol version 15): generation 0's operations, and with them the
    /// destruction of the frozen identity's open listings the transformer read. One
    /// `TokenDestroyFrozenHolding` burns the balance, empties each listing's escrow and lowers
    /// the supply by everything destroyed; one `DeleteDocuments` deletes the listings' `ask`
    /// documents, each built against the ones before it so index trees they share are removed
    /// and refunded; the history and group-action events record everything destroyed. Without
    /// listings the operations are generation 0's.
    fn destroy_frozen_funds_and_listings_operations_v1<'b>(
        self,
        owner_id: Identifier,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        let data_contract_id = self.base().data_contract_id();

        let identity_contract_nonce = self.base().identity_contract_nonce();

        // What the destroy needs of each listing: its id and the tokens it holds.
        let listings: Vec<(Identifier, TokenAmount)> = self
            .escrowed_listings()
            .iter()
            .map(|listing| (listing.id, listing.remaining))
            .collect();
        let destroyed_amount =
            self.total_destroyed()
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a frozen holding is more tokens than a u64 holds",
                )))?;

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
                let token_market_contract = self.token_market_contract().cloned().ok_or(
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "listings to destroy come with the token market contract",
                    )),
                )?;
                let document_ids = listings.iter().map(|(id, _)| *id).collect();
                ops.push(TokenOperation(
                    TokenOperationType::TokenDestroyFrozenHolding {
                        token_id: self.token_id(),
                        holder_id: self.frozen_identity_id(),
                        balance_amount: self.amount(),
                        listings,
                    },
                ));
                ops.push(DocumentOperation(DocumentOperationType::DeleteDocuments {
                    document_ids,
                    contract_info: DataContractInfo::DataContractFetchInfo(token_market_contract),
                    document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr(
                        TokenOrderSide::Ask.document_type_name(),
                    ),
                }));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
        TokenBaseTransitionAction, TokenBaseTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::TokenDestroyFrozenFundsTransitionActionV0;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::tokens::token_order::TokenOrder;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const TOKEN_ID: [u8; 32] = [0xCC; 32];
    const SIGNER: [u8; 32] = [0xAA; 32];
    const FROZEN: [u8; 32] = [0xEE; 32];
    const NONCE: u64 = 4;
    const AMOUNT: TokenAmount = 999;

    fn pv14() -> &'static PlatformVersion {
        PlatformVersion::get(14).expect("expected protocol version 14")
    }

    /// The signer proposes (`proposer`) or completes a group action, which performs it when
    /// `perform_action`.
    fn in_group(proposer: bool) -> GroupStateTransitionResolvedInfo {
        GroupStateTransitionResolvedInfo {
            group_contract_position: 0,
            group: Group::V0(GroupV0 {
                members: BTreeMap::from([(Identifier::from(SIGNER), 1)]),
                required_power: 2,
            }),
            action_id: Identifier::from([0x77; 32]),
            action_is_proposer: proposer,
            signer_power: 1,
        }
    }

    fn listing(id: u8, remaining: TokenAmount) -> TokenOrder {
        TokenOrder {
            id: Identifier::from([id; 32]),
            side: TokenOrderSide::Ask,
            maker_id: Identifier::from(FROZEN),
            token_id: Identifier::from(TOKEN_ID),
            price: 1,
            amount: remaining,
            remaining,
            min_fill: 1,
            expires_at: None,
            escrow: 0,
            decimals: 0,
        }
    }

    fn action(
        group: Option<GroupStateTransitionResolvedInfo>,
        perform_action: bool,
        escrowed_listings: Vec<TokenOrder>,
    ) -> TokenDestroyFrozenFundsTransitionAction {
        let protocol_version = PlatformVersion::latest().protocol_version;
        let contract = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
            protocol_version,
        ));
        let token_market_contract = (!escrowed_listings.is_empty()).then(|| Arc::clone(&contract));
        TokenDestroyFrozenFundsTransitionActionV0 {
            base: TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
                token_id: Identifier::from(TOKEN_ID),
                identity_contract_nonce: NONCE,
                token_contract_position: 0,
                data_contract: contract,
                store_in_group: group.map(|info| (info, None)),
                perform_action,
            }),
            frozen_identity_id: Identifier::from(FROZEN),
            amount: AMOUNT,
            escrowed_listings,
            token_market_contract,
            public_note: Some("note".to_string()),
        }
        .into()
    }

    fn convert(
        action: TokenDestroyFrozenFundsTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Vec<DriveOperation<'static>> {
        action
            .into_high_level_batch_drive_operations(
                &Epoch::new(0).expect("expected an epoch"),
                Identifier::from(SIGNER),
                platform_version,
            )
            .expect("expected the operations")
    }

    fn is_nonce(operation: &DriveOperation) -> bool {
        matches!(
            operation,
            IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                identity_id,
                nonce: NONCE,
                ..
            }) if *identity_id == SIGNER
        )
    }

    fn is_burn_of_the_balance(operation: &DriveOperation) -> bool {
        matches!(
            operation,
            TokenOperation(TokenOperationType::TokenBurn {
                identity_balance_holder_id,
                burn_amount: AMOUNT,
                ..
            }) if *identity_balance_holder_id == Identifier::from(FROZEN)
        )
    }

    fn is_history_of(operation: &DriveOperation, destroyed: TokenAmount) -> bool {
        matches!(
            operation,
            TokenOperation(TokenOperationType::TokenHistory {
                nonce: NONCE,
                event: TokenEvent::DestroyFrozenFunds(frozen, amount, Some(note)),
                ..
            }) if *frozen == Identifier::from(FROZEN) && *amount == destroyed && note == "note"
        )
    }

    /// Generation 0, which protocol version 14 selects: the signer's nonce, the burn of the
    /// frozen balance and its history, and nothing of any listing even when the action carries
    /// some.
    #[test]
    fn should_burn_the_frozen_balance_alone_at_protocol_version_14() {
        for listings in [vec![], vec![listing(1, 300), listing(2, 200)]] {
            let operations = convert(action(None, true, listings), pv14());
            assert_eq!(operations.len(), 3, "{operations:?}");
            assert!(is_nonce(&operations[0]));
            assert!(is_burn_of_the_balance(&operations[1]));
            assert!(is_history_of(&operations[2], AMOUNT));
        }
    }

    /// Generation 0 in a group: a proposal that does not yet perform the action records the
    /// group action with its event and burns nothing; the signature that completes it records
    /// its power, closes the action and burns.
    #[test]
    fn should_record_the_group_action_at_protocol_version_14() {
        let operations = convert(action(Some(in_group(true)), false, vec![]), pv14());
        assert_eq!(operations.len(), 2, "{operations:?}");
        assert!(is_nonce(&operations[0]));
        assert!(matches!(
            &operations[1],
            GroupOperation(GroupOperationType::AddGroupAction {
                initialize_with_insert_action_info: Some(GroupAction::V0(GroupActionV0 {
                    event: GroupActionEvent::TokenEvent(TokenEvent::DestroyFrozenFunds(
                        _,
                        AMOUNT,
                        _
                    )),
                    ..
                })),
                closes_group_action: false,
                signer_power: 1,
                ..
            })
        ));

        let operations = convert(action(Some(in_group(false)), true, vec![]), pv14());
        assert_eq!(operations.len(), 4, "{operations:?}");
        assert!(is_nonce(&operations[0]));
        assert!(matches!(
            &operations[1],
            GroupOperation(GroupOperationType::AddGroupAction {
                initialize_with_insert_action_info: None,
                closes_group_action: true,
                ..
            })
        ));
        assert!(is_burn_of_the_balance(&operations[2]));
        assert!(is_history_of(&operations[3], AMOUNT));
    }

    /// Generation 1, which protocol version 15 selects: without listings, generation 0's
    /// operations; with listings, one holding destroy of the balance and every listing's escrow,
    /// one `DeleteDocuments` of their `ask` documents and the history of everything destroyed.
    #[test]
    fn should_destroy_the_listings_with_the_balance_at_protocol_version_15() {
        let latest = PlatformVersion::latest();
        let operations = convert(action(None, true, vec![]), latest);
        assert_eq!(operations.len(), 3, "{operations:?}");
        assert!(is_burn_of_the_balance(&operations[1]));
        assert!(is_history_of(&operations[2], AMOUNT));

        let operations = convert(
            action(None, true, vec![listing(1, 300), listing(2, 200)]),
            latest,
        );
        assert_eq!(operations.len(), 4, "{operations:?}");
        assert!(is_nonce(&operations[0]));
        assert!(matches!(
            &operations[1],
            TokenOperation(TokenOperationType::TokenDestroyFrozenHolding {
                balance_amount: AMOUNT,
                listings,
                ..
            }) if *listings == vec![(Identifier::from([1; 32]), 300), (Identifier::from([2; 32]), 200)]
        ));
        assert!(matches!(
            &operations[2],
            DocumentOperation(DocumentOperationType::DeleteDocuments { document_ids, .. })
                if *document_ids == vec![Identifier::from([1; 32]), Identifier::from([2; 32])]
        ));
        assert!(is_history_of(&operations[3], AMOUNT + 500));
    }
}
