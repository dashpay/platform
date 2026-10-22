//! Signing tests for `ShieldFromAssetLockTransition` (Type 18).
//!
//! Covers the new external-signer path
//! (`try_from_asset_lock_with_bundle_and_signer`) and exercises the
//! version dispatcher in `methods/mod.rs` plus the high-level builder
//! `build_shield_from_asset_lock_transition_with_signer`.
//!
//! The raw-key path (`try_from_asset_lock_with_bundle`) is already
//! covered by existing tests in `v0/mod.rs` and the byte-parity test
//! in `state_transition::mod` pins `sign_with_core_signer` against
//! `sign_by_private_key` — we don't re-derive that contract here.

// `crate::shielded::builder` (the high-level bundle builder these tests drive) only
// exists under `shielded-client`, so this module must require it too — otherwise the
// `--all-targets` feature-unified lib-test target (which enables `state-transition-signing`
// + `core_key_wallet` without `shielded-client`) fails to resolve the builder import.
#![cfg(all(
    test,
    feature = "state-transition-signing",
    feature = "core_key_wallet",
    feature = "shielded-client"
))]

use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use crate::prelude::AssetLockProof;
use crate::serialization::PlatformSerializable;
use crate::shielded::builder::build_shield_from_asset_lock_transition_with_signer;
use crate::shielded::builder::test_helpers::{test_orchard_address, TestProver};
use crate::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use crate::state_transition::shield_from_asset_lock_transition::v1::ShieldFromAssetLockTransitionV1;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::StateTransition;
use dashcore::bls_sig_utils::BLSSignature;
use dashcore::hash_types::CycleHash;
use dashcore::hashes::Hash;
use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
use dashcore::transaction::special_transaction::TransactionPayload;
use dashcore::{InstantLock, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid};
use platform_version::version::PlatformVersion;

use async_trait::async_trait;
use dashcore::ecdsa::{EcdsaPublicKey, EcdsaSignature};
use key_wallet::bip32::{DerivationPath, ExtendedPubKey};
use key_wallet::signer::{ExtendedPubKeySigner, Signer as KwSigner, SignerMethod};
use secp256k1::{Message, PublicKey, SecretKey};

/// Fixed-key in-memory `key_wallet::signer::Signer`. Mirrors how a
/// Swift KeychainSigner behaves: derive once, sign atomically. Path
/// is ignored — the wrapper holds exactly one key. Same pattern as
/// the `AddressFundingFromAssetLockTransition` signing test and the
/// byte-parity test in `state_transition::mod`.
#[derive(Debug)]
struct FixedKeySigner {
    secret: SecretKey,
    public: PublicKey,
}

impl FixedKeySigner {
    fn new(seed: [u8; 32]) -> Self {
        let secret = SecretKey::from_secret_bytes(seed).expect("valid secret");
        let public = PublicKey::from_secret_key(&secret);
        Self { secret, public }
    }
}

#[async_trait]
impl KwSigner for FixedKeySigner {
    type Error = String;

    fn supported_methods(&self) -> &[SignerMethod] {
        &[SignerMethod::Digest]
    }

    async fn sign_ecdsa(
        &self,
        _path: &DerivationPath,
        sighash: [u8; 32],
    ) -> Result<(EcdsaSignature, EcdsaPublicKey), Self::Error> {
        let msg = Message::from_digest(sighash);
        Ok((self.secret.sign_ecdsa(msg).into(), self.public.into()))
    }

    async fn public_key(&self, _path: &DerivationPath) -> Result<EcdsaPublicKey, Self::Error> {
        Ok(self.public.into())
    }
}

#[async_trait]
impl ExtendedPubKeySigner for FixedKeySigner {
    async fn extended_public_key(
        &self,
        _path: &DerivationPath,
    ) -> Result<ExtendedPubKey, Self::Error> {
        Err("FixedKeySigner does not derive extended public keys".to_string())
    }
}

fn make_chain_asset_lock_proof() -> AssetLockProof {
    AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: 100,
        out_point: OutPoint::from([11u8; 36]),
    })
}

fn extract_v0(state_transition: StateTransition) -> ShieldFromAssetLockTransitionV0 {
    let StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(v0)) =
        state_transition
    else {
        panic!("expected ShieldFromAssetLock V0 variant");
    };
    v0
}

fn extract_v1(state_transition: StateTransition) -> ShieldFromAssetLockTransitionV1 {
    let StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V1(v1)) =
        state_transition
    else {
        panic!("expected ShieldFromAssetLock V1 variant");
    };
    v1
}

#[tokio::test]
async fn try_from_asset_lock_with_bundle_and_signer_produces_recoverable_compact_sig_v0() {
    // Exercises `ShieldFromAssetLockTransitionV0::try_from_asset_lock_with_bundle_and_signer`
    // directly (no version dispatcher). Pins that the outer ECDSA signature
    // produced via `sign_with_core_signer` is 65 bytes (recoverable compact
    // shape), matching the raw-key path that Type 18 uses on storage.
    let asset_lock_proof = make_chain_asset_lock_proof();
    let signer = FixedKeySigner::new([7u8; 32]);
    let path = DerivationPath::default();

    // Bundle fixture values — the trait method doesn't validate the
    // Halo 2 proof or anchor, it only computes signable bytes and
    // signs them, so any fixed-size payload works here. Real bundle
    // construction is covered by the builder-level test below.
    let actions: Vec<crate::shielded::SerializedAction> = vec![];
    let value_balance = 1_000_000u64;
    let anchor = [0u8; 32];
    let proof = vec![];
    let binding_signature = [0u8; 64];

    let st = ShieldFromAssetLockTransitionV0::try_from_asset_lock_with_bundle_and_signer(
        asset_lock_proof,
        &path,
        &signer,
        actions,
        value_balance,
        anchor,
        proof,
        binding_signature,
        None,
        PlatformVersion::latest(),
    )
    .await
    .expect("transition should sign");

    let v0 = extract_v0(st);
    assert_eq!(v0.value_balance, value_balance);
    assert_eq!(
        v0.signature.len(),
        65,
        "asset-lock signature must be 65-byte recoverable compact",
    );
}

#[tokio::test]
async fn try_from_asset_lock_with_bundle_and_signer_via_outer_dispatcher() {
    // Same call but routed through the outer-enum dispatcher in
    // `methods/mod.rs` — pins that the version-routing path lands in
    // the version the protocol version builds: 1 at the latest, 0 at 13.
    let signer = FixedKeySigner::new([7u8; 32]);
    let path = DerivationPath::default();

    let st_13 = ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle_and_signer(
        make_chain_asset_lock_proof(),
        &path,
        &signer,
        vec![],
        500_000,
        [0u8; 32],
        vec![],
        [0u8; 64],
        None,
        PlatformVersion::get(13).expect("protocol version 13"),
    )
    .await
    .expect("outer dispatch should succeed");
    assert_eq!(extract_v0(st_13).value_balance, 500_000);

    let st = ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle_and_signer(
        make_chain_asset_lock_proof(),
        &path,
        &signer,
        vec![],
        500_000,
        [0u8; 32],
        vec![],
        [0u8; 64],
        None,
        PlatformVersion::latest(),
    )
    .await
    .expect("outer dispatch should succeed");

    let v1 = extract_v1(st);
    assert_eq!(v1.value_balance, 500_000);
    assert_eq!(v1.signature.len(), 65);
}

#[tokio::test]
async fn outer_dispatcher_rejects_unknown_serialization_version() {
    // Synthesise a platform-version whose
    // `shield_from_asset_lock_state_transition.default_current_version`
    // is an unknown version (99), and confirm the dispatcher surfaces
    // `UnknownVersionMismatch` instead of silently coercing to a known
    // version, so a future version can't route through the wrong impl
    // without an explicit code change.
    let signer = FixedKeySigner::new([7u8; 32]);
    let path = DerivationPath::default();

    let mut bad_version = PlatformVersion::latest().clone();
    bad_version
        .dpp
        .state_transition_serialization_versions
        .shield_from_asset_lock_state_transition
        .default_current_version = 99;

    let err = ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle_and_signer(
        make_chain_asset_lock_proof(),
        &path,
        &signer,
        vec![],
        1,
        [0u8; 32],
        vec![],
        [0u8; 64],
        None,
        &bad_version,
    )
    .await
    .expect_err("unknown version must be rejected");

    match err {
        crate::ProtocolError::UnknownVersionMismatch {
            method, received, ..
        } => {
            assert!(
                method.contains("ShieldFromAssetLockTransition")
                    && method.contains("try_from_asset_lock_with_bundle_and_signer"),
                "unexpected method: {method}",
            );
            assert_eq!(received, 99);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn build_shield_from_asset_lock_transition_with_signer_end_to_end() {
    // Full builder path — constructs a real output-only Orchard
    // bundle (via the TestProver, which builds the proving key on
    // first call) and signs the outer ST with the external signer.
    // This is the codepath the wallet orchestration uses in production.

    let recipient = test_orchard_address();
    let signer = FixedKeySigner::new([7u8; 32]);
    let path = DerivationPath::default();
    let shield_amount = 50_000u64;

    let st = build_shield_from_asset_lock_transition_with_signer(
        &recipient,
        shield_amount,
        make_chain_asset_lock_proof(),
        &path,
        &signer,
        &TestProver,
        [0u8; 36],
        None, // sender_ovk
        None, // surplus_output
        0,    // dummy_outputs
        PlatformVersion::latest(),
    )
    .await
    .expect("builder should succeed");

    let v1 = extract_v1(st);
    assert_eq!(
        v1.value_balance, shield_amount,
        "builder must thread the shield amount into the transition's value_balance",
    );
    assert_eq!(
        v1.signature.len(),
        65,
        "asset-lock signature must be 65-byte recoverable compact",
    );
    assert!(
        !v1.actions.is_empty(),
        "output-only bundle must produce at least one Orchard action",
    );
}

#[tokio::test]
async fn seed_pool_batch_fits_max_state_transition_size() {
    // The pool-seeding batch size (MAX_ACTIONS_PER_BATCH = 6 in
    // rs-platform-wallet's seed_pool.rs) is bounded by the 20 KiB
    // transaction-size limit, not the 16-action consensus cap: the Halo 2
    // proof grows ~2,681 bytes per action (measured: 2 actions → 8,294 B,
    // 6 → 19,018 B, 7 → 21,699 B — the last rejected by tenderdash's
    // `mempool.max-tx-bytes = 20480` as "Tx too large"). Pin the largest
    // seeding batch under `system_limits.max_state_transition_size` so a
    // proof- or action-encoding size change that breaks seeding fails here
    // instead of on a devnet.
    let platform_version = PlatformVersion::latest();
    let recipient = test_orchard_address();
    let signer = FixedKeySigner::new([7u8; 32]);
    let path = DerivationPath::default();

    let st = build_shield_from_asset_lock_transition_with_signer(
        &recipient,
        50_000u64,
        make_chain_asset_lock_proof(),
        &path,
        &signer,
        &TestProver,
        [0u8; 36],
        None, // sender_ovk
        None, // surplus_output
        5,    // dummy_outputs -> 6 on-wire actions, the seeding batch max
        platform_version,
    )
    .await
    .expect("builder should succeed");

    let bytes = st.serialize_to_bytes().expect("serialize");
    let max = platform_version.system_limits.max_state_transition_size as usize;
    assert!(
        bytes.len() <= max,
        "a 6-action seeding batch must fit max_state_transition_size: {} > {}",
        bytes.len(),
        max,
    );

    let v1 = extract_v1(st);
    assert_eq!(
        v1.actions.len(),
        6,
        "1 real + 5 dummy outputs must serialize to 6 Orchard actions",
    );
    assert_eq!(
        v1.value_balance, 50_000,
        "dummy outputs are zero-value: value_balance must equal the real amount",
    );
}

// ---------------------------------------------------------------------
// Asset-lock proof size (dashwallet-ios#1042)
//
// An `InstantAssetLockProof` embeds the whole funding transaction and its
// InstantLock, so it grows ~184 B per funding input (148 B signed P2PKH
// input + 36 B outpoint repeated in the lock). A wallet fed by many small
// payouts built an 83-input lock (raw tx 12,340 B) whose Type 18 transition
// DAPI refused at 20,480 B. A ChainLock proof names only the outpoint and a
// height, so the same lock fits once it is chainlocked — which is what the
// wallet falls back to.
// ---------------------------------------------------------------------

/// Input count of the asset lock reported in dashwallet-ios#1042.
const ISSUE_1042_INPUTS: usize = 83;

/// Wire length of a signed P2PKH `script_sig` (72-byte DER signature with
/// sighash byte + 33-byte key + two push opcodes): a 148-byte input.
const P2PKH_SCRIPT_SIG_LEN: usize = 107;

/// An asset-lock funding transaction with `n_inputs` signed P2PKH spends,
/// the burn output and a change output — the shape the issue reports.
fn make_asset_lock_tx(n_inputs: usize) -> Transaction {
    let input = (0..n_inputs)
        .map(|i| {
            let mut raw = [0u8; 32];
            raw[..8].copy_from_slice(&(i as u64).to_le_bytes());
            TxIn {
                previous_output: OutPoint::new(Txid::from_byte_array(raw), 0),
                script_sig: ScriptBuf::from_bytes(vec![0u8; P2PKH_SCRIPT_SIG_LEN]),
                sequence: 0xffff_ffff,
                witness: Default::default(),
            }
        })
        .collect();
    let credit_output = TxOut {
        value: 6_442_042_028,
        script_pubkey: ScriptBuf::from_bytes(vec![0u8; 25]),
    };
    Transaction {
        version: 3,
        lock_time: 0,
        input,
        output: vec![
            TxOut {
                value: 6_442_042_028,
                script_pubkey: ScriptBuf::new_op_return(&[]),
            },
            TxOut {
                value: 6_366,
                script_pubkey: ScriptBuf::from_bytes(vec![0u8; 25]),
            },
        ],
        special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(
            AssetLockPayload::new(vec![credit_output]),
        )),
    }
}

fn make_instant_asset_lock_proof(n_inputs: usize) -> AssetLockProof {
    let transaction = make_asset_lock_tx(n_inputs);
    let instant_lock = InstantLock {
        version: 1,
        inputs: transaction
            .input
            .iter()
            .map(|i| i.previous_output)
            .collect(),
        txid: transaction.txid(),
        cyclehash: CycleHash::from_byte_array([9u8; 32]),
        signature: BLSSignature::from([3u8; 96]),
    };
    AssetLockProof::Instant(InstantAssetLockProof::new(instant_lock, transaction, 0))
}

/// Serialized size of a `ShieldFromAssetLock` transition over `proof` on the
/// wallet's single-recipient path (2 on-wire actions).
async fn shield_transition_size(proof: AssetLockProof) -> usize {
    build_shield_from_asset_lock_transition_with_signer(
        &test_orchard_address(),
        50_000u64,
        proof,
        &DerivationPath::default(),
        &FixedKeySigner::new([7u8; 32]),
        &TestProver,
        [0u8; 36],
        None, // sender_ovk
        None, // surplus_output
        0,    // dummy_outputs -> 2 on-wire actions
        PlatformVersion::latest(),
    )
    .await
    .expect("builder should succeed")
    .serialize_to_bytes()
    .expect("serialize")
    .len()
}

#[tokio::test]
async fn should_exceed_max_size_with_the_issue_1042_instant_proof() {
    let max = PlatformVersion::latest()
        .system_limits
        .max_state_transition_size as usize;

    let one_input = shield_transition_size(make_instant_asset_lock_proof(1)).await;
    let issue = shield_transition_size(make_instant_asset_lock_proof(ISSUE_1042_INPUTS)).await;

    assert!(
        one_input <= max,
        "a 1-input instant proof must fit: {one_input} > {max}"
    );
    // ~148 B of signed P2PKH input plus its 36-byte outpoint repeated in the
    // InstantLock. A band, not an exact figure: length prefixes and the
    // Orchard proof vary by a few bytes between builds.
    let per_input = (issue - one_input) as f64 / (ISSUE_1042_INPUTS - 1) as f64;
    assert!(
        (180.0..=190.0).contains(&per_input),
        "an instant proof should cost ~184 B per funding input, got {per_input:.1}",
    );
    assert!(
        issue > max,
        "the {ISSUE_1042_INPUTS}-input instant proof must overflow \
         max_state_transition_size, or the wallet's chain fallback is never needed: \
         {issue} <= {max}",
    );
}

#[tokio::test]
async fn should_fit_max_size_with_a_chain_proof_for_the_issue_1042_lock() {
    let max = PlatformVersion::latest()
        .system_limits
        .max_state_transition_size as usize;

    // The same 83-input lock, proven by its ChainLock: the proof names only
    // the outpoint and a height, so its size does not depend on how many
    // inputs funded the lock.
    let chain_proof = AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: 2_000_000,
        out_point: OutPoint::new(make_asset_lock_tx(ISSUE_1042_INPUTS).txid(), 0),
    });
    let size = shield_transition_size(chain_proof).await;

    assert!(
        size <= max,
        "the chain-proof fallback for an {ISSUE_1042_INPUTS}-input lock must fit \
         max_state_transition_size: {size} > {max}",
    );
}
