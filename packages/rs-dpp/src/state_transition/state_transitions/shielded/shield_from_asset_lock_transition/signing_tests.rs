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
use crate::prelude::AssetLockProof;
use crate::shielded::builder::build_shield_from_asset_lock_transition_with_signer;
use crate::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use crate::state_transition::shield_from_asset_lock_transition::v1::ShieldFromAssetLockTransitionV1;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::StateTransition;
use dashcore::OutPoint;
use platform_version::version::PlatformVersion;

use async_trait::async_trait;
use dashcore::secp256k1::{ecdsa, Message, PublicKey, Secp256k1, SecretKey};
use key_wallet::bip32::{DerivationPath, ExtendedPubKey};
use key_wallet::signer::{ExtendedPubKeySigner, Signer as KwSigner, SignerMethod};

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
        let secp = Secp256k1::new();
        let secret = SecretKey::from_byte_array(&seed).expect("valid secret");
        let public = PublicKey::from_secret_key(&secp, &secret);
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
    ) -> Result<(ecdsa::Signature, PublicKey), Self::Error> {
        let secp = Secp256k1::new();
        let msg = Message::from_digest(sighash);
        Ok((secp.sign_ecdsa(&msg, &self.secret), self.public))
    }

    async fn public_key(&self, _path: &DerivationPath) -> Result<PublicKey, Self::Error> {
        Ok(self.public)
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

/// An InstantSend proof of a one-output asset-lock transaction. Its InstantSend lock is not
/// valid; nothing here verifies it.
fn make_instant_asset_lock_proof() -> AssetLockProof {
    use crate::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
    use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dashcore::transaction::special_transaction::TransactionPayload;
    use dashcore::{InstantLock, ScriptBuf, Transaction, TxOut};

    let transaction = Transaction {
        version: 3,
        lock_time: 0,
        input: vec![],
        output: vec![],
        special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(
            AssetLockPayload {
                version: 0,
                credit_outputs: vec![TxOut {
                    value: 100_000,
                    script_pubkey: ScriptBuf::new(),
                }],
            },
        )),
    };
    AssetLockProof::Instant(InstantAssetLockProof::new(
        InstantLock::default(),
        transaction,
        0,
    ))
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
    use crate::shielded::builder::test_helpers::{test_orchard_address, TestProver};

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
    use crate::serialization::PlatformSerializable;
    use crate::shielded::builder::test_helpers::{test_orchard_address, TestProver};

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

#[tokio::test]
async fn proved_bundle_assembles_around_instant_and_chain_proofs_of_its_outpoint() {
    // The bundle is proved before the InstantSend lock exists, from the outpoint alone, then
    // wrapped around the InstantSend proof and — after a rejection — around a ChainLock proof of
    // the same outpoint. Both assemblies must carry the one proof, and each must be signed afresh
    // so a resubmission never repeats a rejected transition's hash.
    use crate::shielded::builder::test_helpers::{test_orchard_address, TestProver};
    use crate::shielded::builder::ProvedShieldFromAssetLockBundle;

    let platform_version = PlatformVersion::latest();
    let signer = FixedKeySigner::new([7u8; 32]);
    let path = DerivationPath::default();
    let shield_amount = 50_000u64;

    let instant = make_instant_asset_lock_proof();
    let out_point = instant.out_point().expect("fixture has a credit output");
    let chain = AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: 100,
        out_point,
    });
    let other_lock = make_chain_asset_lock_proof();

    let proved = ProvedShieldFromAssetLockBundle::prove(
        &test_orchard_address(),
        shield_amount,
        out_point,
        &TestProver,
        [0u8; 36],
        None,
        0,
        platform_version,
    )
    .expect("proving should succeed");
    assert!(proved.is_bound_for(&instant, platform_version).unwrap());
    assert!(proved.is_bound_for(&chain, platform_version).unwrap());
    assert!(!proved.is_bound_for(&other_lock, platform_version).unwrap());

    let assemble = |proof: AssetLockProof| {
        proved.build_transition_with_signer(proof, &path, &signer, None, platform_version)
    };
    let over_instant = extract_v1(assemble(instant.clone()).await.expect("instant assembly"));
    let resubmitted = extract_v1(assemble(instant).await.expect("instant re-assembly"));
    let over_chain = extract_v1(assemble(chain).await.expect("chain assembly"));

    for v1 in [&over_instant, &resubmitted, &over_chain] {
        assert_eq!(v1.value_balance, shield_amount);
        assert_eq!(v1.proof, over_instant.proof, "one proof, never re-proved");
        assert_eq!(v1.anchor, over_instant.anchor);
        assert_eq!(v1.actions.len(), over_instant.actions.len());
        for (action, first) in v1.actions.iter().zip(&over_instant.actions) {
            assert_eq!(action.nullifier, first.nullifier);
            assert_eq!(action.cmx, first.cmx);
            assert_eq!(action.cv_net, first.cv_net);
            assert_eq!(action.encrypted_note, first.encrypted_note);
        }
    }
    assert_ne!(
        over_instant.binding_signature, resubmitted.binding_signature,
        "every assembly signs afresh, so the same proof never yields the same transition twice"
    );
    assert_ne!(over_instant.signature, resubmitted.signature);

    let err = assemble(other_lock)
        .await
        .expect_err("a bundle bound to another outpoint must not be assembled");
    assert!(
        matches!(&err, crate::ProtocolError::ShieldedBuildError(msg)
            if msg.contains("does not match the binding")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn proved_bundle_binding_is_the_outpoint_binding_at_every_version() {
    // `ProvedShieldFromAssetLockBundle::prove` derives the binding from a stand-in chain proof
    // of the outpoint. That is only sound because the binding never depends on the proof kind
    // or the chain-lock height: pin it at every version, binding or not.
    use crate::shielded::shield_from_asset_lock_extra_sighash_data;

    let instant = make_instant_asset_lock_proof();
    let out_point = instant.out_point().expect("fixture has a credit output");
    let versions: Vec<&PlatformVersion> = (1..=PlatformVersion::latest().protocol_version)
        .filter_map(|version| PlatformVersion::get(version).ok())
        .collect();
    assert!(versions.len() > 1);
    for platform_version in versions {
        let bound = |proof: &AssetLockProof| {
            shield_from_asset_lock_extra_sighash_data(proof, platform_version).unwrap()
        };
        for height in [0, 1, u32::MAX] {
            assert_eq!(
                bound(&instant),
                bound(&AssetLockProof::Chain(ChainAssetLockProof {
                    core_chain_locked_height: height,
                    out_point,
                })),
            );
        }
    }
}
