//! The token bundle of `TokenShieldedTransferWithShieldedFee`, `TokenUnshieldWithShieldedFee`
//! and `TokenPurchaseFromShieldedPool` binds its fee bundle's commitment into its sighash from
//! protocol version 15, and the fee bundle binds a digest of the token bundle's actions, so
//! neither bundle can be paired with a bundle of different effects. Before that version the
//! token bundle binds no fee bundle; `token_pool_outputs_only_copy_tests` pins what protocol
//! version 14 does with such a pair.

use super::token_pool_outputs_only_copy_tests::{
    build_purchase, process_in_one_block, token_for_sale,
};
use super::token_pool_paid_transitions_tests::{
    credit_pool_balance, fund_credit_pool, fund_token_pool, spendable, wallet_address, Prover,
    CREDIT_NOTE, SHIELDED,
};
use super::token_shielded_pool_tests::{
    assert_check_tx_accepts, assert_check_tx_rejects, assert_tokens_conserved, build_spend_bundle,
    current_platform_version, enable_shielded_pool, identity_token_balance, nullifier_is_spent,
    platform_with_latest_version, pool_balance, spend_keys, spendable_note,
};
use super::*;
use crate::execution::validation::state_transition::processor::traits::shielded_proof::StateTransitionShieldedProofValidationV0;
use crate::execution::validation::state_transition::state_transitions::test_helpers::{
    set_pool_total_balance, setup_platform_at_protocol_version,
};
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::shielded::builder::{
    build_token_purchase_from_shielded_pool_transition,
    build_token_shielded_transfer_with_shielded_fee_transition,
    build_token_unshield_with_shielded_fee_transition, ShieldedFeePayer, TokenPoolSpender,
};
use dpp::shielded::{
    token_pool_fee_bundle_extra_sighash_data, TOKEN_PURCHASE_FROM_SHIELDED_POOL_TYPE,
    TOKEN_SHIELDED_TRANSFER_WITH_SHIELDED_FEE_TYPE, TOKEN_UNSHIELD_WITH_SHIELDED_FEE_TYPE,
};
use dpp::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use dpp::state_transition::token_shielded_transfer_with_shielded_fee_transition::TokenShieldedTransferWithShieldedFeeTransition;
use dpp::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;
use dpp::state_transition::StateTransition;
use grovedb_commitment_tree::{Anchor, MerklePath, Note};
use platform_version::version::PlatformVersion;

/// Tokens a payment moves: an unshield credits them to the recipient, a transfer pays them to
/// a note inside the pool.
const PAYMENT: u64 = 6;
/// Tokens a purchase buys at `PRICE_PER_TOKEN` each.
const TOKEN_COUNT: u64 = 3;
/// 0.01 DASH.
const PRICE_PER_TOKEN: Credits = 1_000_000_000;

/// What a token pool transition paid from the credit pool carries, read the same way for the
/// three kinds.
struct PoolPaidParts {
    state_transition_type: u8,
    token_id: Identifier,
    token_actions: Vec<dpp::shielded::SerializedAction>,
    fee_nullifiers: Vec<[u8; 32]>,
    credit_amount: Credits,
}

fn parts(transition: &StateTransition) -> PoolPaidParts {
    match transition {
        StateTransition::TokenShieldedTransferWithShieldedFee(
            TokenShieldedTransferWithShieldedFeeTransition::V0(v0),
        ) => PoolPaidParts {
            state_transition_type: TOKEN_SHIELDED_TRANSFER_WITH_SHIELDED_FEE_TYPE,
            token_id: v0.token_id,
            token_actions: v0.token_actions.clone(),
            fee_nullifiers: v0.fee_actions.iter().map(|a| a.nullifier).collect(),
            credit_amount: v0.credit_amount,
        },
        StateTransition::TokenUnshieldWithShieldedFee(
            TokenUnshieldWithShieldedFeeTransition::V0(v0),
        ) => PoolPaidParts {
            state_transition_type: TOKEN_UNSHIELD_WITH_SHIELDED_FEE_TYPE,
            token_id: v0.token_id,
            token_actions: v0.token_actions.clone(),
            fee_nullifiers: v0.fee_actions.iter().map(|a| a.nullifier).collect(),
            credit_amount: v0.credit_amount,
        },
        StateTransition::TokenPurchaseFromShieldedPool(
            TokenPurchaseFromShieldedPoolTransition::V0(v0),
        ) => PoolPaidParts {
            state_transition_type: TOKEN_PURCHASE_FROM_SHIELDED_POOL_TYPE,
            token_id: v0.token_id,
            token_actions: v0.token_actions.clone(),
            fee_nullifiers: v0.fee_actions.iter().map(|a| a.nullifier).collect(),
            credit_amount: v0.credit_amount,
        },
        other => panic!("not a token pool transition paid from the credit pool: {other:?}"),
    }
}

/// `transition` with its token bundle unchanged and its fee bundle replaced by one that spends
/// `note` (against `anchor`), carries the same credits out of the credit pool and binds the
/// same token actions under the fee bundle's preimage. Returns the pair and its fee nullifiers.
fn with_another_fee_bundle(
    transition: &StateTransition,
    note: Note,
    anchor: Anchor,
    merkle_path: MerklePath,
    seed: u64,
    platform_version: &PlatformVersion,
) -> (StateTransition, Vec<[u8; 32]>) {
    let parts = parts(transition);
    let fee_extra = token_pool_fee_bundle_extra_sighash_data(
        parts.state_transition_type,
        &parts.token_id.to_buffer(),
        &parts.token_actions,
        platform_version,
    )
    .expect("fee bundle sighash data");
    let (fee_bundle, value_balance) = build_spend_bundle(
        note,
        merkle_path,
        anchor,
        parts.credit_amount,
        &fee_extra,
        seed,
    );
    assert_eq!(value_balance, parts.credit_amount as i64);
    let fee_nullifiers = fee_bundle.actions.iter().map(|a| a.nullifier).collect();
    let pair = match transition.clone() {
        StateTransition::TokenShieldedTransferWithShieldedFee(
            TokenShieldedTransferWithShieldedFeeTransition::V0(mut v0),
        ) => {
            v0.fee_actions = fee_bundle.actions;
            v0.fee_anchor = fee_bundle.anchor;
            v0.fee_proof = fee_bundle.proof;
            v0.fee_binding_signature = fee_bundle.binding_signature;
            TokenShieldedTransferWithShieldedFeeTransition::V0(v0).into()
        }
        StateTransition::TokenUnshieldWithShieldedFee(
            TokenUnshieldWithShieldedFeeTransition::V0(mut v0),
        ) => {
            v0.fee_actions = fee_bundle.actions;
            v0.fee_anchor = fee_bundle.anchor;
            v0.fee_proof = fee_bundle.proof;
            v0.fee_binding_signature = fee_bundle.binding_signature;
            TokenUnshieldWithShieldedFeeTransition::V0(v0).into()
        }
        StateTransition::TokenPurchaseFromShieldedPool(
            TokenPurchaseFromShieldedPoolTransition::V0(mut v0),
        ) => {
            v0.fee_actions = fee_bundle.actions;
            v0.fee_anchor = fee_bundle.anchor;
            v0.fee_proof = fee_bundle.proof;
            v0.fee_binding_signature = fee_bundle.binding_signature;
            TokenPurchaseFromShieldedPoolTransition::V0(v0).into()
        }
        other => panic!("not a token pool transition paid from the credit pool: {other:?}"),
    };
    (pair, fee_nullifiers)
}

/// The notes and ids a token payment (a transfer inside the pool or an unshield) is built from.
struct TokenPayment {
    contract_id: Identifier,
    token_id: Identifier,
    recipient_id: Identifier,
    token_note: (Note, Anchor, MerklePath),
    credit_note: (Note, Anchor, MerklePath),
}

/// A token that owns a pool holding a `SHIELDED` note of the test wallet, a recipient identity,
/// and a `CREDIT_NOTE` credit pool note (`credit_tag`) of the test wallet to pay the fee with.
async fn token_payment(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    seed: u64,
    credit_tag: u8,
    platform_version: &PlatformVersion,
) -> TokenPayment {
    let mut rng = StdRng::seed_from_u64(seed);
    let (owner, signer, key) = setup_identity(platform, rng.gen(), dash_to_credits!(0.5));
    let (recipient, _, _) = setup_identity(platform, rng.gen(), dash_to_credits!(0.1));
    let (contract, token_id) = create_token_contract_with_owner_identity(
        platform,
        owner.id(),
        Some(enable_shielded_pool),
        None,
        None,
        None,
        platform_version,
    );
    let token_note = fund_token_pool(
        platform,
        &contract,
        token_id,
        &owner,
        &key,
        &signer,
        51,
        11,
        platform_version,
    )
    .await;
    let credit_note = fund_credit_pool(platform, credit_tag);
    TokenPayment {
        contract_id: contract.id(),
        token_id,
        recipient_id: recipient.id(),
        token_note,
        credit_note,
    }
}

/// A transfer of `PAYMENT` tokens inside the pool (`unshield == false`) or an unshield of
/// `PAYMENT` tokens to the recipient, built for `platform_version` from `payment`'s notes.
fn build_payment(
    payment: &TokenPayment,
    unshield: bool,
    platform_version: &PlatformVersion,
) -> StateTransition {
    let (fvk, ask, _) = spend_keys();
    let address = wallet_address();
    let (token_note, token_anchor, token_path) = payment.token_note.clone();
    let (credit_note, credit_anchor, credit_path) = payment.credit_note.clone();
    let spender = TokenPoolSpender {
        spends: vec![spendable(token_note, token_path)],
        change_address: &address,
        fvk: &fvk,
        ask: &ask,
        anchor: token_anchor,
    };
    let fee_payer = ShieldedFeePayer {
        spends: vec![spendable(credit_note, credit_path)],
        change_address: &address,
        fvk: &fvk,
        ask: &ask,
        anchor: credit_anchor,
    };
    let (transition, _) = if unshield {
        build_token_unshield_with_shielded_fee_transition(
            payment.token_id,
            payment.contract_id,
            0,
            spender,
            payment.recipient_id,
            PAYMENT,
            [0u8; 36],
            fee_payer,
            &Prover,
            platform_version,
        )
    } else {
        build_token_shielded_transfer_with_shielded_fee_transition(
            payment.token_id,
            payment.contract_id,
            0,
            spender,
            &address,
            PAYMENT,
            [0u8; 36],
            fee_payer,
            &Prover,
            platform_version,
        )
    }
    .expect("build token payment");
    transition
}

fn credit_nullifier_is_spent(
    platform: &TempPlatform<MockCoreRPCLike>,
    nullifier: &[u8; 32],
) -> bool {
    platform
        .drive
        .has_nullifier(
            nullifier,
            None,
            &mut vec![],
            current_platform_version(platform),
        )
        .expect("credit pool nullifier lookup")
}

/// Both `original` and `pair` are admissible on their own state: the payer's and the other
/// payer's credit notes are both in the pool, which holds enough for both. CheckTx refuses the
/// pair on its proof, and in a block that sequences the pair first the pair is refused unpaid
/// and writes nothing, the original executes, and only the original's fee leaves the pool.
fn assert_pair_refused_and_original_lands(
    platform: &TempPlatform<MockCoreRPCLike>,
    original: &StateTransition,
    pair: &StateTransition,
    pair_fee_nullifiers: &[[u8; 32]],
) {
    let original_parts = parts(original);
    assert_check_tx_accepts(platform, original);
    assert_matches!(
        assert_check_tx_rejects(platform, pair).as_slice(),
        [ConsensusError::StateError(
            StateError::InvalidShieldedProofError(_)
        )]
    );
    let credit_pool_before = credit_pool_balance(platform);
    assert_matches!(
        process_in_one_block(platform, &[pair, original], &BlockInfo::default())
            .execution_results()
            .as_slice(),
        [
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                StateError::InvalidShieldedProofError(_)
            )),
            StateTransitionExecutionResult::SuccessfulExecution { .. },
        ]
    );
    assert_eq!(
        credit_pool_balance(platform),
        credit_pool_before - original_parts.credit_amount,
        "only the original's credits leave the credit pool"
    );
    for nullifier in &original_parts.fee_nullifiers {
        assert!(credit_nullifier_is_spent(platform, nullifier));
    }
    for nullifier in pair_fee_nullifiers {
        if !original_parts.fee_nullifiers.contains(nullifier) {
            assert!(
                !credit_nullifier_is_spent(platform, nullifier),
                "the refused pair leaves its fee notes unspent"
            );
        }
    }
    assert!(nullifier_is_spent(
        platform,
        original_parts.token_id,
        &original_parts.token_actions[0].nullifier
    ));
    assert_tokens_conserved(platform);
}

#[tokio::test]
async fn should_refuse_a_token_shielded_transfer_bundle_paired_with_another_fee_bundle() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let payment = token_payment(&mut platform, 9601, 12, platform_version).await;
    let original = build_payment(&payment, false, platform_version);
    let (other_note, other_anchor, other_path) = fund_credit_pool(&platform, 77);
    set_pool_total_balance(&platform, 2 * CREDIT_NOTE);
    let (pair, pair_fee_nullifiers) = with_another_fee_bundle(
        &original,
        other_note,
        other_anchor,
        other_path,
        9602,
        platform_version,
    );

    assert_pair_refused_and_original_lands(&platform, &original, &pair, &pair_fee_nullifiers);
    assert_eq!(pool_balance(&platform, payment.token_id), SHIELDED);
}

#[tokio::test]
async fn should_refuse_a_token_unshield_bundle_paired_with_another_fee_bundle() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let payment = token_payment(&mut platform, 9611, 12, platform_version).await;
    let original = build_payment(&payment, true, platform_version);
    let (other_note, other_anchor, other_path) = fund_credit_pool(&platform, 77);
    set_pool_total_balance(&platform, 2 * CREDIT_NOTE);
    let (pair, pair_fee_nullifiers) = with_another_fee_bundle(
        &original,
        other_note,
        other_anchor,
        other_path,
        9612,
        platform_version,
    );

    assert_pair_refused_and_original_lands(&platform, &original, &pair, &pair_fee_nullifiers);
    assert_eq!(
        pool_balance(&platform, payment.token_id),
        SHIELDED - PAYMENT
    );
    assert_eq!(
        identity_token_balance(&platform, payment.token_id, payment.recipient_id),
        Some(PAYMENT),
        "the recipient is credited once"
    );
}

#[tokio::test]
async fn should_refuse_a_token_purchase_bundle_paired_with_another_fee_bundle() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let (_seller_id, contract_id, token_id) =
        token_for_sale(&mut platform, 9621, PRICE_PER_TOKEN, platform_version).await;
    let (original, _fee) = build_purchase(
        &platform,
        token_id,
        contract_id,
        TOKEN_COUNT,
        PRICE_PER_TOKEN * TOKEN_COUNT,
        15,
        platform_version,
    );
    let (other_note, other_anchor, other_path) = fund_credit_pool(&platform, 16);
    set_pool_total_balance(&platform, 2 * CREDIT_NOTE);
    let (pair, pair_fee_nullifiers) = with_another_fee_bundle(
        &original,
        other_note,
        other_anchor,
        other_path,
        9622,
        platform_version,
    );

    assert_pair_refused_and_original_lands(&platform, &original, &pair, &pair_fee_nullifiers);
    assert_eq!(pool_balance(&platform, token_id), TOKEN_COUNT);
}

/// The binding is to the fee bundle itself, not to the notes it spends: a second fee bundle the
/// same payer builds over the same credit note, for the same token actions, has another
/// commitment, so the token bundle refuses it as well.
#[tokio::test]
async fn should_refuse_a_token_bundle_paired_with_a_second_fee_bundle_over_the_same_notes() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let payment = token_payment(&mut platform, 9631, 12, platform_version).await;
    let original = build_payment(&payment, false, platform_version);
    let (credit_note, credit_anchor, credit_path) = payment.credit_note.clone();
    let (pair, pair_fee_nullifiers) = with_another_fee_bundle(
        &original,
        credit_note,
        credit_anchor,
        credit_path,
        9632,
        platform_version,
    );
    assert_ne!(pair, original);

    assert_pair_refused_and_original_lands(&platform, &original, &pair, &pair_fee_nullifiers);
}

/// Each kind is verified under the binding of the protocol version it is validated at: a
/// transition built for protocol version 14 binds no fee bundle and verifies only there, one
/// built for a later version binds its fee bundle and verifies only there. The fee bundle is
/// verified in its own right at both: one that does not reconstruct, or whose binding signature
/// does not verify, refuses the transition although the token bundle's preimage is unchanged.
/// Stateless, as the proof check is.
#[test]
fn should_verify_each_token_pool_paid_kind_only_under_the_binding_it_was_built_for() {
    let v14 = PlatformVersion::get(14).expect("protocol version 14");
    let latest = PlatformVersion::latest();
    let (fvk, ask, _) = spend_keys();
    let address = wallet_address();
    let contract_id = Identifier::from([9u8; 32]);
    let token_id: Identifier = dpp::tokens::calculate_token_id(contract_id.as_bytes(), 0).into();
    let recipient_id = Identifier::from([7u8; 32]);

    let build = |kind: u8, platform_version: &PlatformVersion| -> StateTransition {
        let (token_note, token_anchor, token_path) = spendable_note(SHIELDED, 11);
        let (credit_note, credit_anchor, credit_path) = spendable_note(CREDIT_NOTE, 12);
        let fee_payer = ShieldedFeePayer {
            spends: vec![spendable(credit_note, credit_path)],
            change_address: &address,
            fvk: &fvk,
            ask: &ask,
            anchor: credit_anchor,
        };
        let spender = TokenPoolSpender {
            spends: vec![spendable(token_note, token_path)],
            change_address: &address,
            fvk: &fvk,
            ask: &ask,
            anchor: token_anchor,
        };
        let (transition, _) = match kind {
            TOKEN_SHIELDED_TRANSFER_WITH_SHIELDED_FEE_TYPE => {
                build_token_shielded_transfer_with_shielded_fee_transition(
                    token_id,
                    contract_id,
                    0,
                    spender,
                    &address,
                    PAYMENT,
                    [0u8; 36],
                    fee_payer,
                    &Prover,
                    platform_version,
                )
            }
            TOKEN_UNSHIELD_WITH_SHIELDED_FEE_TYPE => {
                build_token_unshield_with_shielded_fee_transition(
                    token_id,
                    contract_id,
                    0,
                    spender,
                    recipient_id,
                    PAYMENT,
                    [0u8; 36],
                    fee_payer,
                    &Prover,
                    platform_version,
                )
            }
            _ => build_token_purchase_from_shielded_pool_transition(
                token_id,
                contract_id,
                0,
                &address,
                &fvk,
                TOKEN_COUNT,
                PRICE_PER_TOKEN * TOKEN_COUNT,
                [0u8; 36],
                fee_payer,
                &Prover,
                platform_version,
            ),
        }
        .expect("build transition");
        transition
    };
    let verifies = |transition: &StateTransition, platform_version: &PlatformVersion| {
        transition
            .validate_shielded_proof(platform_version)
            .expect("proof validation")
            .is_valid()
    };

    for kind in [
        TOKEN_SHIELDED_TRANSFER_WITH_SHIELDED_FEE_TYPE,
        TOKEN_UNSHIELD_WITH_SHIELDED_FEE_TYPE,
        TOKEN_PURCHASE_FROM_SHIELDED_POOL_TYPE,
    ] {
        let unbound = build(kind, v14);
        let bound = build(kind, latest);
        assert!(
            verifies(&unbound, v14),
            "type {kind} built for 14 verifies at 14"
        );
        assert!(
            !verifies(&unbound, latest),
            "type {kind} built for 14 binds no fee bundle, which the latest version refuses"
        );
        assert!(
            verifies(&bound, latest),
            "type {kind} built for latest verifies there"
        );
        assert!(
            !verifies(&bound, v14),
            "type {kind} built for latest binds its fee bundle, which 14 does not expect"
        );
        for (transition, platform_version) in [(&unbound, v14), (&bound, latest)] {
            let bad_binding_signature = with_fee_bundle(transition, |_, binding_signature| {
                binding_signature[0] ^= 1;
            });
            assert!(
                !verifies(&bad_binding_signature, platform_version),
                "type {kind}: a fee bundle whose binding signature does not verify is refused"
            );
            let undecodable = with_fee_bundle(transition, |actions, _| {
                actions[0].cv_net = [0xFF; 32];
            });
            assert!(
                !verifies(&undecodable, platform_version),
                "type {kind}: a fee bundle that does not reconstruct is refused"
            );
        }
    }
}

/// `transition` with `change` applied to its fee bundle's actions and binding signature.
fn with_fee_bundle(
    transition: &StateTransition,
    change: impl FnOnce(&mut [dpp::shielded::SerializedAction], &mut [u8; 64]),
) -> StateTransition {
    match transition.clone() {
        StateTransition::TokenShieldedTransferWithShieldedFee(
            TokenShieldedTransferWithShieldedFeeTransition::V0(mut v0),
        ) => {
            change(&mut v0.fee_actions, &mut v0.fee_binding_signature);
            TokenShieldedTransferWithShieldedFeeTransition::V0(v0).into()
        }
        StateTransition::TokenUnshieldWithShieldedFee(
            TokenUnshieldWithShieldedFeeTransition::V0(mut v0),
        ) => {
            change(&mut v0.fee_actions, &mut v0.fee_binding_signature);
            TokenUnshieldWithShieldedFeeTransition::V0(v0).into()
        }
        StateTransition::TokenPurchaseFromShieldedPool(
            TokenPurchaseFromShieldedPoolTransition::V0(mut v0),
        ) => {
            change(&mut v0.fee_actions, &mut v0.fee_binding_signature);
            TokenPurchaseFromShieldedPoolTransition::V0(v0).into()
        }
        other => panic!("not a token pool transition paid from the credit pool: {other:?}"),
    }
}

/// On the first block that runs protocol version 15, a token payment signed for protocol
/// version 14 and admitted to the mempool under it is refused unpaid on the proposer's path,
/// writes nothing, and leaves its notes to a rebuild for the new version, which executes in the
/// same block.
#[tokio::test]
async fn should_refuse_a_token_payment_built_for_protocol_version_14_on_the_first_block_after_it() {
    let old_version = PlatformVersion::get(14).expect("protocol version 14");
    let new_version = PlatformVersion::latest();
    // The first block of a new protocol version reads the masternode list from Core.
    let mut platform =
        setup_platform_at_protocol_version(old_version.protocol_version).with_no_core_masternodes();
    let payment = token_payment(&mut platform, 9641, 12, old_version).await;
    let built_for_14 = build_payment(&payment, false, old_version);
    assert_check_tx_accepts(&platform, &built_for_14);

    let mut candidate_state = platform.state.load().as_ref().clone();
    candidate_state.set_current_protocol_version_in_consensus(new_version.protocol_version);
    let block_info = BlockInfo {
        height: 2,
        ..BlockInfo::default()
    };
    let candidate = platform.drive.grove.start_transaction();
    platform
        .perform_events_on_first_block_of_protocol_change(
            &candidate_state,
            &mut PlatformState::clone(&candidate_state),
            &block_info,
            &candidate,
            old_version.protocol_version,
            new_version,
        )
        .expect("first block events of the new protocol version");
    let root_hash = || {
        platform
            .drive
            .grove
            .root_hash(Some(&candidate), &new_version.drive.grove_version)
            .unwrap()
            .expect("root hash")
    };
    let process = |transition: &StateTransition| {
        platform
            .platform
            .process_raw_state_transitions(
                &[transition.serialize_to_bytes().expect("transition bytes")],
                &candidate_state,
                &block_info,
                &candidate,
                new_version,
                true,
                None,
            )
            .expect("transition processing")
    };

    let before = root_hash();
    assert_matches!(
        process(&built_for_14).execution_results().as_slice(),
        [StateTransitionExecutionResult::UnpaidConsensusError(
            ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
        )]
    );
    assert_eq!(root_hash(), before, "the refusal writes nothing");

    let rebuilt = build_payment(&payment, false, new_version);
    assert_matches!(
        process(&rebuilt).execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }],
        "the same notes rebuilt for the new version execute"
    );
}

/// A token payment built for protocol version 15 binds its fee bundle, which a protocol
/// version 14 node does not expect: CheckTx refuses it there. The bytes are not dead, though:
/// the first block of 15 executes them unchanged, so a refusal at admission is never proof that
/// the payment cannot happen.
#[tokio::test]
async fn should_refuse_a_fee_bound_token_payment_at_14_and_execute_the_same_bytes_after_it() {
    let old_version = PlatformVersion::get(14).expect("protocol version 14");
    let new_version = PlatformVersion::latest();
    // The first block of a new protocol version reads the masternode list from Core.
    let mut platform =
        setup_platform_at_protocol_version(old_version.protocol_version).with_no_core_masternodes();
    let payment = token_payment(&mut platform, 9651, 12, old_version).await;
    let built_for_latest = build_payment(&payment, false, new_version);
    assert_matches!(
        assert_check_tx_rejects(&platform, &built_for_latest).as_slice(),
        [ConsensusError::StateError(
            StateError::InvalidShieldedProofError(_)
        )]
    );

    let mut candidate_state = platform.state.load().as_ref().clone();
    candidate_state.set_current_protocol_version_in_consensus(new_version.protocol_version);
    let block_info = BlockInfo {
        height: 2,
        ..BlockInfo::default()
    };
    let candidate = platform.drive.grove.start_transaction();
    platform
        .perform_events_on_first_block_of_protocol_change(
            &candidate_state,
            &mut PlatformState::clone(&candidate_state),
            &block_info,
            &candidate,
            old_version.protocol_version,
            new_version,
        )
        .expect("first block events of the new protocol version");
    assert_matches!(
        platform
            .platform
            .process_raw_state_transitions(
                &[built_for_latest
                    .serialize_to_bytes()
                    .expect("transition bytes")],
                &candidate_state,
                &block_info,
                &candidate,
                new_version,
                true,
                None,
            )
            .expect("transition processing")
            .execution_results()
            .as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
}
