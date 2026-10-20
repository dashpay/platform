//! What the mempool does with a batch whose token shielded pool nullifiers are already spent.
//!
//! Such a batch can never execute: the pool holds the nullifier, so block validation refuses it
//! on that. Both mempool passes have to reach the same conclusion without paying for the batch's
//! Halo 2 verification first — admission, so an unauthenticated submitter cannot buy proof work
//! with a bundle that is already dead, and the re-check after every block, so a batch that was
//! admitted before the note was spent leaves the mempool rather than waiting for a proposer to
//! spend a block slot on it.

use super::token_shielded_pool_tests::{
    build_shield_bundle, build_spend_bundle, insert_token_pool_anchor, nullifier_is_spent,
    platform_with_latest_version, process, spendable_note, SHIELD_AMOUNT,
};
use super::*;
use crate::execution::check_tx::CheckTxLevel;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::shielded::{token_unshield_extra_sighash_data_v0, OrchardBundleParams};
use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionType;
use dpp::state_transition::StateTransition;
use simple_signer::signer::SimpleSigner;

/// The errors CheckTx refuses `transition` with at `level`, or a panic if it admits it.
fn check_tx_errors(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: &StateTransition,
    level: CheckTxLevel,
) -> Vec<ConsensusError> {
    let state = platform.state.load();
    let platform_ref = PlatformRef {
        drive: &platform.drive,
        state: &state,
        config: &platform.config,
        core_rpc: &platform.core_rpc,
    };
    let result = platform
        .check_tx(
            &transition
                .serialize_to_bytes()
                .expect("serialize transition"),
            level,
            &platform_ref,
            PlatformVersion::latest(),
        )
        .expect("check tx");
    assert!(
        !result.is_valid(),
        "CheckTx at {level:?} admitted a batch whose pool nullifier is already spent"
    );
    result.errors
}

/// A pool holding one spent note, and everything needed to build another unshield that spends it
/// again: the unshield's bundle (valid, and the same bytes the landed one carried) and the
/// nullifier the pool now records.
struct SpentNote {
    platform: TempPlatform<MockCoreRPCLike>,
    identity: Identity,
    signer: SimpleSigner,
    key: IdentityPublicKey,
    contract: DataContract,
    token_id: Identifier,
    recipient: Identifier,
    amount: u64,
    bundle: OrchardBundleParams,
    nullifier: [u8; 32],
}

/// Shields into a token's pool, then unshields one note out of it, so that the note's nullifier
/// is recorded and any further spend of it is refused.
async fn pool_with_a_spent_note() -> SpentNote {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let mut rng = StdRng::seed_from_u64(70_401);

    let (identity, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));
    let (contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        identity.id(),
        Some(super::token_shielded_pool_tests::enable_shielded_pool),
        None,
        None,
        None,
        platform_version,
    );

    let shield_bundle = build_shield_bundle(
        SHIELD_AMOUNT,
        71,
        TokenTransitionActionType::Shield,
        token_id,
        identity.id(),
    );
    let shield = BatchTransition::new_token_shield_transition(
        token_id,
        identity.id(),
        contract.id(),
        0,
        SHIELD_AMOUNT,
        shield_bundle,
        &key,
        2,
        0,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("token shield transition");
    assert_matches!(
        process(&platform, &shield).execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );

    let (note, anchor, merkle_path) = spendable_note(6_000, 1);
    insert_token_pool_anchor(&platform, token_id, &anchor);
    let amount = 4_000;
    let extra = token_unshield_extra_sighash_data_v0(
        &token_id.to_buffer(),
        &identity.id().to_buffer(),
        &recipient.id().to_buffer(),
        amount,
    );
    let (bundle, _) = build_spend_bundle(note, merkle_path, anchor, amount, &extra, 72);
    let nullifier = bundle.actions[0].nullifier;

    let unshield = BatchTransition::new_token_unshield_transition(
        token_id,
        identity.id(),
        contract.id(),
        0,
        amount,
        recipient.id(),
        bundle.clone(),
        &key,
        3,
        0,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("token unshield transition");
    assert_matches!(
        process(&platform, &unshield).execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert!(
        nullifier_is_spent(&platform, token_id, &nullifier),
        "the landed unshield must have recorded the note's nullifier"
    );

    SpentNote {
        platform,
        identity,
        signer,
        key,
        contract,
        token_id,
        recipient: recipient.id(),
        amount,
        bundle,
        nullifier,
    }
}

impl SpentNote {
    /// Another unshield of the same note, carrying `bundle`, at nonce 4.
    async fn unshield_again(&self, bundle: OrchardBundleParams) -> StateTransition {
        BatchTransition::new_token_unshield_transition(
            self.token_id,
            self.identity.id(),
            self.contract.id(),
            0,
            self.amount,
            self.recipient,
            bundle,
            &self.key,
            4,
            0,
            &self.signer,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("token unshield transition")
    }
}

/// The batch carries a proof that does not verify AND a nullifier the pool already holds. Both
/// are grounds to refuse it, and which error comes back says which check ran first: reading the
/// pool costs a handful of key lookups, verifying the bundle costs a Halo 2 verification, and an
/// unauthenticated submitter must not be able to buy the second with a bundle the first would
/// have thrown away.
#[tokio::test]
async fn check_tx_refuses_a_spent_pool_nullifier_without_verifying_the_bundle() {
    let fixture = pool_with_a_spent_note().await;

    let mut tampered = fixture.bundle.clone();
    tampered.proof[0] ^= 0xff;
    let repeat = fixture.unshield_again(tampered).await;

    let errors = check_tx_errors(&fixture.platform, &repeat, CheckTxLevel::FirstTimeCheck);
    assert_matches!(
        errors.as_slice(),
        [ConsensusError::StateError(StateError::NullifierAlreadySpentError(error))]
            if error.nullifier() == fixture.nullifier,
        "the pool read must refuse this batch before its proof is verified, got {errors:?}"
    );
}

/// A batch admitted while its note was still unspent is re-checked after every block. Once the
/// note is spent the batch can only fail, so the re-check must drop it: left in the mempool it
/// occupies a slot in a proposer's block and lands there as a paid failure.
#[tokio::test]
async fn recheck_evicts_a_batch_whose_pool_nullifier_has_been_spent() {
    let fixture = pool_with_a_spent_note().await;

    // A valid bundle, so nothing but the pool's record can refuse it.
    let repeat = fixture.unshield_again(fixture.bundle.clone()).await;

    let errors = check_tx_errors(&fixture.platform, &repeat, CheckTxLevel::Recheck);
    assert_matches!(
        errors.as_slice(),
        [ConsensusError::StateError(StateError::NullifierAlreadySpentError(error))]
            if error.nullifier() == fixture.nullifier,
        "the re-check must evict this batch, got {errors:?}"
    );
}
