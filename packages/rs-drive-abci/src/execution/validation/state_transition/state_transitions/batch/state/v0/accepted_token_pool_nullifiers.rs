use dpp::consensus::state::shielded::nullifier_already_spent_error::NullifierAlreadySpentError;
use dpp::identifier::Identifier;
use dpp::shielded::SerializedAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_index_only_delete_transition_action::v0::DocumentIndexOnlyDeleteTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::DocumentPurchaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::DocumentReplaceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::DocumentTransferTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_update_price_transition_action::DocumentUpdatePriceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_burn_from_pool_transition_action::TokenBurnFromPoolTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_claim_to_pool_transition_action::TokenClaimToPoolTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_to_pool_transition_action::TokenDirectPurchaseToPoolTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_mint_to_pool_transition_action::TokenMintToPoolTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_shield_transition_action::TokenShieldTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_shielded_transfer_transition_action::TokenShieldedTransferTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_unshield_transition_action::TokenUnshieldTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use std::collections::BTreeSet;

/// The pool footprint checked for a candidate, including outputs-only padding actions.
pub(super) struct TokenPoolNullifiers {
    token_id: Identifier,
    nullifiers: Vec<[u8; 32]>,
    writes_pool_nullifiers: bool,
}

impl TokenPoolNullifiers {
    pub(super) fn for_transition(transition: &BatchedTransitionAction) -> Option<Self> {
        let (token_id, actions, writes_pool_nullifiers) = match transition {
            BatchedTransitionAction::TokenAction(token) => match token {
                TokenTransitionAction::ShieldAction(action) => {
                    (action.token_id(), action.actions(), true)
                }
                TokenTransitionAction::UnshieldAction(action) => {
                    (action.token_id(), action.actions(), true)
                }
                TokenTransitionAction::ShieldedTransferAction(action) => {
                    (action.token_id(), action.actions(), true)
                }
                TokenTransitionAction::MintToPoolAction(action) => (
                    action.token_id(),
                    action.actions(),
                    action.base().perform_action(),
                ),
                TokenTransitionAction::BurnFromPoolAction(action) => (
                    action.token_id(),
                    action.actions(),
                    action.base().perform_action(),
                ),
                TokenTransitionAction::ClaimToPoolAction(action) => {
                    (action.token_id(), action.actions(), true)
                }
                TokenTransitionAction::DirectPurchaseToPoolAction(action) => {
                    (action.token_id(), action.actions(), true)
                }
                TokenTransitionAction::BurnAction(_)
                | TokenTransitionAction::MintAction(_)
                | TokenTransitionAction::TransferAction(_)
                | TokenTransitionAction::FreezeAction(_)
                | TokenTransitionAction::UnfreezeAction(_)
                | TokenTransitionAction::ClaimAction(_)
                | TokenTransitionAction::EmergencyActionAction(_)
                | TokenTransitionAction::DestroyFrozenFundsAction(_)
                | TokenTransitionAction::ConfigUpdateAction(_)
                | TokenTransitionAction::DirectPurchaseAction(_)
                | TokenTransitionAction::SetPriceForDirectPurchaseAction(_) => return None,
            },
            BatchedTransitionAction::DocumentAction(document) => {
                let base = match document {
                    DocumentTransitionAction::CreateAction(action) => action.base(),
                    DocumentTransitionAction::ReplaceAction(action) => action.base(),
                    DocumentTransitionAction::TransferAction(action) => action.base(),
                    DocumentTransitionAction::DeleteAction(action) => action.base(),
                    DocumentTransitionAction::UpdatePriceAction(action) => action.base(),
                    DocumentTransitionAction::PurchaseAction(action) => action.base(),
                    DocumentTransitionAction::IndexOnlyDeleteAction(action) => action.base(),
                };
                match (base.token_cost(), base.shielded_token_payment()) {
                    (Some((token_id, _, _)), Some(payment)) => {
                        (token_id, payment.payment.actions.as_slice(), true)
                    }
                    (None, _) | (_, None) => return None,
                }
            }
            BatchedTransitionAction::BumpIdentityDataContractNonce(_) => return None,
        };
        Some(Self::from_actions(
            token_id,
            actions,
            writes_pool_nullifiers,
        ))
    }

    fn from_actions(
        token_id: Identifier,
        actions: &[SerializedAction],
        writes_pool_nullifiers: bool,
    ) -> Self {
        Self {
            token_id,
            nullifiers: actions.iter().map(|action| action.nullifier).collect(),
            writes_pool_nullifiers,
        }
    }
}

/// Nullifiers reserved only by earlier accepted actions that actually write to a token pool.
#[derive(Default)]
pub(super) struct AcceptedTokenPoolNullifiers {
    accepted: BTreeSet<(Identifier, [u8; 32])>,
}

impl AcceptedTokenPoolNullifiers {
    /// Checking never claims a nullifier, even when the candidate partially overlaps.
    pub(super) fn check(
        &self,
        candidate: &TokenPoolNullifiers,
    ) -> Option<NullifierAlreadySpentError> {
        candidate
            .nullifiers
            .iter()
            .find(|nullifier| self.accepted.contains(&(candidate.token_id, **nullifier)))
            .map(|nullifier| NullifierAlreadySpentError::new(*nullifier))
    }

    /// Call only after every candidate acceptance check and settlement has succeeded.
    pub(super) fn record(&mut self, candidate: TokenPoolNullifiers) {
        if candidate.writes_pool_nullifiers {
            self.accepted.extend(
                candidate
                    .nullifiers
                    .into_iter()
                    .map(|nullifier| (candidate.token_id, nullifier)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::create_dummy_serialized_action;
    use dpp::block::epoch::Epoch;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::{Document, DocumentV0};
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::tokens::token_amount_on_contract_token::DocumentActionTokenEffect;
    use dpp::tokens::token_payment_info::v1::TokenShieldedPayment;
    use dpp::version::PlatformVersion;
    use drive::drive::contract::DataContractFetchInfo;
    use drive::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
    use drive::state_transition_action::batch::v0::BatchTransitionActionV0;
    use drive::state_transition_action::StateTransitionAction;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0, DocumentShieldedTokenPayment};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_index_only_delete_transition_action::v0::DocumentIndexOnlyDeleteTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::DocumentReplaceTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::DocumentTransferTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_update_price_transition_action::DocumentUpdatePriceTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::DocumentPurchaseTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::token_transition::token_shield_transition_action::TokenShieldTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::token_transition::token_unshield_transition_action::TokenUnshieldTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::token_transition::token_shielded_transfer_transition_action::TokenShieldedTransferTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::token_transition::token_mint_to_pool_transition_action::TokenMintToPoolTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::token_transition::token_burn_from_pool_transition_action::TokenBurnFromPoolTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::token_transition::token_claim_to_pool_transition_action::TokenClaimToPoolTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_to_pool_transition_action::TokenDirectPurchaseToPoolTransitionActionV0;
    use drive::util::batch::DriveOperation;
    use drive::util::batch::drive_op_batch::TokenOperationType;
    use std::sync::Arc;

    fn footprint(token: u8, nullifiers: &[u8], writes: bool) -> TokenPoolNullifiers {
        TokenPoolNullifiers {
            token_id: Identifier::from([token; 32]),
            nullifiers: nullifiers.iter().map(|byte| [*byte; 32]).collect(),
            writes_pool_nullifiers: writes,
        }
    }

    #[test]
    fn should_leave_partial_overlap_unclaimed_and_choose_the_first_candidate_nullifier() {
        let mut tracker = AcceptedTokenPoolNullifiers::default();
        tracker.record(footprint(1, &[10, 30], true));
        assert_eq!(
            tracker
                .check(&footprint(1, &[20, 30, 10], true))
                .unwrap()
                .nullifier(),
            [30; 32]
        );
        assert!(
            tracker.check(&footprint(1, &[20], true)).is_none(),
            "a refused candidate cannot claim its disjoint prefix"
        );
        tracker.record(footprint(1, &[20], true));
        assert_eq!(
            tracker
                .check(&footprint(1, &[10, 30], true))
                .unwrap()
                .nullifier(),
            [10; 32]
        );
    }

    #[test]
    fn should_keep_token_namespaces_separate_and_not_compare_a_candidate_with_itself() {
        let mut tracker = AcceptedTokenPoolNullifiers::default();
        assert!(
            tracker.check(&footprint(1, &[10, 10], true)).is_none(),
            "within-bundle validation owns self repetition"
        );
        tracker.record(footprint(1, &[10], true));
        assert!(tracker.check(&footprint(2, &[10], true)).is_none());
        tracker.record(footprint(2, &[10], true));
        assert!(tracker.check(&footprint(2, &[10], true)).is_some());
    }

    #[test]
    fn should_check_non_writing_groups_without_reserving_them() {
        let mut tracker = AcceptedTokenPoolNullifiers::default();
        tracker.record(footprint(1, &[10], false));
        assert!(tracker.check(&footprint(1, &[10], true)).is_none());
        tracker.record(footprint(1, &[10], true));
        assert!(tracker.check(&footprint(1, &[10], false)).is_some());
    }

    #[test]
    fn should_match_all_seven_token_pool_lowering_footprints_including_group_execution() {
        let token_id = Identifier::from([1; 32]);
        let owner = Identifier::from([2; 32]);
        let mut first = create_dummy_serialized_action();
        first.nullifier = [10; 32];
        let mut padding = create_dummy_serialized_action();
        padding.nullifier = [20; 32];
        let actions = vec![first, padding];
        for perform_action in [false, true] {
            let base: TokenBaseTransitionAction = TokenBaseTransitionActionV0 {
                token_id,
                identity_contract_nonce: 1,
                token_contract_position: 0,
                data_contract: Arc::new(DataContractFetchInfo::dpns_contract_fixture(
                    PlatformVersion::latest().protocol_version,
                )),
                store_in_group: None,
                perform_action,
            }
            .into();
            let candidates: Vec<TokenTransitionAction> = vec![
                TokenTransitionAction::ShieldAction(
                    TokenShieldTransitionActionV0 {
                        base: base.clone(),
                        amount: 1,
                        actions: actions.clone(),
                        anchor: [0; 32],
                        proof: vec![],
                        binding_signature: [0; 64],
                    }
                    .into(),
                ),
                TokenTransitionAction::UnshieldAction(
                    TokenUnshieldTransitionActionV0 {
                        base: base.clone(),
                        amount: 1,
                        recipient_id: owner,
                        actions: actions.clone(),
                        anchor: [0; 32],
                        proof: vec![],
                        binding_signature: [0; 64],
                    }
                    .into(),
                ),
                TokenTransitionAction::ShieldedTransferAction(
                    TokenShieldedTransferTransitionActionV0 {
                        base: base.clone(),
                        actions: actions.clone(),
                        anchor: [0; 32],
                        proof: vec![],
                        binding_signature: [0; 64],
                    }
                    .into(),
                ),
                TokenTransitionAction::MintToPoolAction(
                    TokenMintToPoolTransitionActionV0 {
                        base: base.clone(),
                        amount: 1,
                        public_note: None,
                        actions: actions.clone(),
                        anchor: [0; 32],
                        proof: vec![],
                        binding_signature: [0; 64],
                    }
                    .into(),
                ),
                TokenTransitionAction::BurnFromPoolAction(
                    TokenBurnFromPoolTransitionActionV0 {
                        base: base.clone(),
                        amount: 1,
                        public_note: None,
                        actions: actions.clone(),
                        anchor: [0; 32],
                        proof: vec![],
                        binding_signature: [0; 64],
                    }
                    .into(),
                ),
                TokenTransitionAction::ClaimToPoolAction(
                    TokenClaimToPoolTransitionActionV0 {
                        base: base.clone(),
                        amount: 1,
                        public_note: None,
                        distribution_info: TokenDistributionInfo::OncePerIdentity(0, owner),
                        actions: actions.clone(),
                        anchor: [0; 32],
                        proof: vec![],
                        binding_signature: [0; 64],
                    }
                    .into(),
                ),
                TokenTransitionAction::DirectPurchaseToPoolAction(
                    TokenDirectPurchaseToPoolTransitionActionV0 {
                        base,
                        token_count: 1,
                        total_agreed_price: 1,
                        actions: actions.clone(),
                        anchor: [0; 32],
                        proof: vec![],
                        binding_signature: [0; 64],
                    }
                    .into(),
                ),
            ];
            for token in candidates {
                let action = BatchedTransitionAction::TokenAction(token.clone());
                let footprint = TokenPoolNullifiers::for_transition(&action).unwrap();
                assert_eq!(footprint.token_id, token_id);
                assert_eq!(footprint.nullifiers, vec![[10; 32], [20; 32]]);
                let batch = StateTransitionAction::BatchAction(
                    BatchTransitionActionV0 {
                        owner_id: owner,
                        transitions: vec![BatchedTransitionAction::TokenAction(token)],
                        ..Default::default()
                    }
                    .into(),
                );
                let operations = batch
                    .into_high_level_drive_operations(&Epoch::default(), PlatformVersion::latest())
                    .unwrap();
                let writes: Vec<_> = operations
                    .iter()
                    .filter_map(|operation| match operation {
                        DriveOperation::TokenOperation(
                            TokenOperationType::TokenShield {
                                token_id, notes, ..
                            }
                            | TokenOperationType::TokenMintToPool {
                                token_id, notes, ..
                            },
                        ) => Some((
                            *token_id,
                            notes.iter().map(|note| note.nullifier).collect::<Vec<_>>(),
                        )),
                        DriveOperation::TokenOperation(
                            TokenOperationType::TokenUnshield {
                                token_id,
                                nullifiers,
                                ..
                            }
                            | TokenOperationType::TokenShieldedTransfer {
                                token_id,
                                nullifiers,
                                ..
                            }
                            | TokenOperationType::TokenBurnFromPool {
                                token_id,
                                nullifiers,
                                ..
                            },
                        ) => Some((*token_id, nullifiers.clone())),
                        _ => None,
                    })
                    .collect();
                let mut tracker = AcceptedTokenPoolNullifiers::default();
                tracker.record(footprint);
                let recorded: Vec<_> = tracker.accepted.into_iter().collect();
                let lowered: Vec<_> = writes
                    .into_iter()
                    .flat_map(|(id, nullifiers)| nullifiers.into_iter().map(move |nf| (id, nf)))
                    .collect();
                assert_eq!(
                    recorded, lowered,
                    "collector reservations must match actual pool writes"
                );
            }
        }
    }

    #[test]
    fn should_collect_all_document_arms_only_when_lowering_writes_a_shielded_payment() {
        let contract = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ));
        let owner = contract.contract.owner_id();
        let resolved_token = Identifier::from([7; 32]);
        let mut action = create_dummy_serialized_action();
        action.nullifier = [10; 32];
        for effect in [
            DocumentActionTokenEffect::BurnToken,
            DocumentActionTokenEffect::TransferTokenToContractOwner,
        ] {
            let base: DocumentBaseTransitionAction = DocumentBaseTransitionActionV0 {
                id: Identifier::from([3; 32]),
                identity_contract_nonce: 1,
                document_type_name: "domain".to_string(),
                data_contract: contract.clone(),
                token_cost: Some((resolved_token, effect, 0)),
                gas_fees_paid_by: GasFeesPaidBy::default(),
                contract_gas_fees_paid_by: GasFeesPaidBy::default(),
                declared_action_fee: None,
                shielded_token_payment: Some(Box::new(DocumentShieldedTokenPayment {
                    payment: TokenShieldedPayment {
                        amount: 0,
                        actions: vec![action.clone()],
                        anchor: [0; 32],
                        proof: vec![],
                        binding_signature: [0; 64],
                    },
                    token_contract_id: Identifier::from([9; 32]),
                    token_contract_position: 0,
                })),
            }
            .into();
            let document = Document::V0(DocumentV0::default());
            let documents: Vec<DocumentTransitionAction> = vec![
                DocumentTransitionAction::CreateAction(
                    DocumentCreateTransitionActionV0 {
                        base: base.clone(),
                        block_info: BlockInfo::default(),
                        data: Default::default(),
                        prefunded_voting_balance: None,
                        current_store_contest_info: None,
                        should_store_contest_info: None,
                        property_constraint_reads: None,
                        moderated: false,
                        consumed_documents: vec![],
                        derived_index_values: None,
                    }
                    .into(),
                ),
                DocumentTransitionAction::DeleteAction(
                    DocumentDeleteTransitionActionV0 { base: base.clone() }.into(),
                ),
                DocumentTransitionAction::IndexOnlyDeleteAction(
                    DocumentIndexOnlyDeleteTransitionActionV0 {
                        base: base.clone(),
                        data: Default::default(),
                    }
                    .into(),
                ),
                DocumentTransitionAction::ReplaceAction(
                    DocumentReplaceTransitionActionV0 {
                        base: base.clone(),
                        revision: 2,
                        created_at: None,
                        updated_at: None,
                        transferred_at: None,
                        created_at_block_height: None,
                        updated_at_block_height: None,
                        transferred_at_block_height: None,
                        created_at_core_block_height: None,
                        updated_at_core_block_height: None,
                        transferred_at_core_block_height: None,
                        data: Default::default(),
                        changed_data_fields: Default::default(),
                        removed_identifier_fields: Default::default(),
                        stored_changed_values: Default::default(),
                        creator_id: None,
                        moderated_at: None,
                        moderated_by: None,
                        property_constraint_reads: None,
                    }
                    .into(),
                ),
                DocumentTransitionAction::TransferAction(
                    DocumentTransferTransitionActionV0 {
                        base: base.clone(),
                        document: document.clone(),
                        property_constraint_reads: None,
                    }
                    .into(),
                ),
                DocumentTransitionAction::UpdatePriceAction(
                    DocumentUpdatePriceTransitionActionV0 {
                        base: base.clone(),
                        document: document.clone(),
                        property_constraint_reads: None,
                    }
                    .into(),
                ),
                DocumentTransitionAction::PurchaseAction(
                    DocumentPurchaseTransitionActionV0 {
                        base: base.clone(),
                        document,
                        original_owner_id: owner,
                        price: 0,
                        property_constraint_reads: None,
                    }
                    .into(),
                ),
            ];
            for document in documents {
                let candidate = BatchedTransitionAction::DocumentAction(document.clone());
                let footprint = TokenPoolNullifiers::for_transition(&candidate).unwrap();
                assert_eq!(
                    footprint.token_id, resolved_token,
                    "the resolved cost determines the actual pool namespace"
                );
                assert_eq!(footprint.nullifiers, vec![[10; 32]]);
                assert!(
                    footprint.writes_pool_nullifiers,
                    "zero cost and a contract-owner payer still lower a pool operation"
                );
                let operations = document.base().token_cost_operations(owner);
                assert!(
                    matches!(operations.as_slice(), [DriveOperation::TokenOperation(
                    TokenOperationType::TokenUnshield { token_id, nullifiers, .. }
                    | TokenOperationType::TokenBurnFromPool { token_id, nullifiers, .. })]
                    if *token_id == footprint.token_id && *nullifiers == footprint.nullifiers)
                );
            }
            for (has_cost, has_payment) in [(false, true), (true, false), (false, false)] {
                let DocumentBaseTransitionAction::V0(mut without_pool) = base.clone();
                if !has_cost {
                    without_pool.token_cost = None;
                }
                if !has_payment {
                    without_pool.shielded_token_payment = None;
                }
                let candidate = BatchedTransitionAction::DocumentAction(
                    DocumentTransitionAction::DeleteAction(
                        DocumentDeleteTransitionActionV0 {
                            base: without_pool.into(),
                        }
                        .into(),
                    ),
                );
                assert!(TokenPoolNullifiers::for_transition(&candidate).is_none());
            }
        }
    }
}
