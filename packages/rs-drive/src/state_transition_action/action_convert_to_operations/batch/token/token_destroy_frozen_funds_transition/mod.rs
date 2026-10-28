use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::TokenDestroyFrozenFundsTransitionAction;
use crate::util::batch::DriveOperation;

mod v0;
mod v1;

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
            0 => self.into_high_level_batch_drive_operations_v0(owner_id),
            // From protocol version 15: also destroys what the frozen identity's open listings
            // hold in escrow, and deletes their `ask` documents.
            1 => self.into_high_level_batch_drive_operations_v1(owner_id),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenDestroyFrozenFundsTransitionAction::into_high_level_batch_drive_operations"
                    .to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::group::action_event::GroupActionEvent;
    use dpp::group::group_action::GroupAction;
    use dpp::group::group_action::v0::GroupActionV0;
    use dpp::group::GroupStateTransitionResolvedInfo;
    use dpp::tokens::token_event::TokenEvent;
    use crate::util::batch::drive_op_batch::{GroupOperationType, TokenOperationType};
    use crate::util::batch::DriveOperation::{GroupOperation, IdentityOperation, TokenOperation};
    use crate::util::batch::IdentityOperationType;
    use crate::util::batch::DocumentOperationType;
    use crate::util::batch::DriveOperation::DocumentOperation;
    use crate::util::object_size_info::DocumentTypeInfo;
    use dpp::balances::credits::TokenAmount;
    use dpp::tokens::token_order_side::TokenOrderSide;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
        TokenBaseTransitionAction, TokenBaseTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::{FrozenListings, TokenDestroyFrozenFundsTransitionActionV0};
    use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
    use dpp::block::block_info::BlockInfo;
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
        let frozen_listings = (!escrowed_listings.is_empty()).then(|| FrozenListings {
            listings: escrowed_listings,
            token_market_contract: Arc::clone(&contract),
        });
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
            frozen_listings,
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

    /// The holding destroy of the balance and the escrow of listings 1 and 2, lowering the
    /// supply by the balance and their 500.
    fn is_holding_destroy(operation: &DriveOperation) -> bool {
        matches!(
            operation,
            TokenOperation(TokenOperationType::TokenDestroyFrozenHolding {
                balance_amount: AMOUNT,
                order_ids,
                destroyed_amount,
                ..
            }) if *order_ids == vec![Identifier::from([1; 32]), Identifier::from([2; 32])]
                && *destroyed_amount == AMOUNT + 500
        )
    }

    /// The deletion of listing `id`'s `ask` document.
    fn is_delete_of(operation: &DriveOperation, id: u8) -> bool {
        matches!(
            operation,
            DocumentOperation(DocumentOperationType::DeleteDocument {
                document_id,
                document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr(name),
                ..
            }) if *document_id == Identifier::from([id; 32])
                && *name == TokenOrderSide::Ask.document_type_name()
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
    /// operations, in a group too; with listings, one holding destroy of the balance and every
    /// listing's escrow, a `DeleteDocument` per listing and the history of everything destroyed.
    #[test]
    fn should_destroy_the_listings_with_the_balance_at_protocol_version_15() {
        let latest = PlatformVersion::latest();
        for (group, perform_action) in [
            (None, true),
            (Some(in_group(true)), false),
            (Some(in_group(false)), true),
        ] {
            let at_15 = convert(action(group.clone(), perform_action, vec![]), latest);
            let at_14 = convert(action(group, perform_action, vec![]), pv14());
            assert_eq!(format!("{at_15:?}"), format!("{at_14:?}"));
        }

        let operations = convert(
            action(None, true, vec![listing(1, 300), listing(2, 200)]),
            latest,
        );
        assert_eq!(operations.len(), 5, "{operations:?}");
        assert!(is_nonce(&operations[0]));
        assert!(is_holding_destroy(&operations[1]));
        assert!(is_delete_of(&operations[2], 1));
        assert!(is_delete_of(&operations[3], 2));
        assert!(is_history_of(&operations[4], AMOUNT + 500));
    }

    /// Generation 1 in a group, with listings: a proposal that does not perform yet records
    /// the group action with everything the destroy will take (the balance and the listings'
    /// escrow) and destroys nothing; the signature that completes it closes the action,
    /// destroys the holding, deletes the listings and records the total.
    #[test]
    fn should_record_the_listings_in_a_group_action_at_protocol_version_15() {
        let latest = PlatformVersion::latest();
        let listings = || vec![listing(1, 300), listing(2, 200)];

        let operations = convert(action(Some(in_group(true)), false, listings()), latest);
        assert_eq!(operations.len(), 2, "{operations:?}");
        assert!(is_nonce(&operations[0]));
        assert!(matches!(
            &operations[1],
            GroupOperation(GroupOperationType::AddGroupAction {
                initialize_with_insert_action_info: Some(GroupAction::V0(GroupActionV0 {
                    event: GroupActionEvent::TokenEvent(TokenEvent::DestroyFrozenFunds(
                        frozen,
                        destroyed,
                        _
                    )),
                    ..
                })),
                closes_group_action: false,
                ..
            }) if *frozen == Identifier::from(FROZEN) && *destroyed == AMOUNT + 500
        ));

        let operations = convert(action(Some(in_group(false)), true, listings()), latest);
        assert_eq!(operations.len(), 6, "{operations:?}");
        assert!(is_nonce(&operations[0]));
        assert!(matches!(
            &operations[1],
            GroupOperation(GroupOperationType::AddGroupAction {
                initialize_with_insert_action_info: None,
                closes_group_action: true,
                ..
            })
        ));
        assert!(is_holding_destroy(&operations[2]));
        assert!(is_delete_of(&operations[3], 1));
        assert!(is_delete_of(&operations[4], 2));
        assert!(is_history_of(&operations[5], AMOUNT + 500));
    }

    /// The history the converter emits is the history the shared builders reconstruct
    /// (`associated_token_event`, `build_historical_document`): the frozen balance alone at
    /// protocol version 14, whose transformer reads no listings, and the balance with the
    /// listings' escrow at 15.
    #[test]
    fn should_emit_the_history_the_shared_builders_reconstruct() {
        let block_info = BlockInfo::default();
        for (platform_version, listings, destroyed) in [
            (pv14(), vec![], AMOUNT),
            (
                PlatformVersion::latest(),
                vec![listing(1, 300), listing(2, 200)],
                AMOUNT + 500,
            ),
        ] {
            let action = action(None, true, listings);
            let token_action = TokenTransitionAction::DestroyFrozenFundsAction(action.clone());
            let operations = convert(action, platform_version);
            let Some(TokenOperation(TokenOperationType::TokenHistory { event, .. })) =
                operations.last()
            else {
                panic!("expected the history last: {operations:?}");
            };
            assert!(is_history_of(operations.last().unwrap(), destroyed));

            let reconstructed = token_action
                .associated_token_event()
                .expect("expected the event");
            assert_eq!(&reconstructed, event);

            let emitted_document = event
                .clone()
                .build_historical_document_owned(
                    Identifier::from(TOKEN_ID),
                    Identifier::from(SIGNER),
                    NONCE,
                    &block_info,
                    platform_version,
                )
                .expect("expected the emitted history document");
            let rebuilt_document = token_action
                .build_historical_document(
                    Identifier::from(TOKEN_ID),
                    Identifier::from(SIGNER),
                    NONCE,
                    &block_info,
                    platform_version,
                )
                .expect("expected the rebuilt history document");
            assert_eq!(rebuilt_document, emitted_document);
        }
    }
}
