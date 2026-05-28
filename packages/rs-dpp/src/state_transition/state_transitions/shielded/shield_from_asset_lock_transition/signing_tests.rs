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

#![cfg(all(
    test,
    feature = "state-transition-signing",
    feature = "core_key_wallet"
))]

use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::prelude::AssetLockProof;
use crate::shielded::builder::build_shield_from_asset_lock_transition_with_signer;
use crate::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::StateTransition;
use dashcore::OutPoint;
use platform_version::version::PlatformVersion;

use async_trait::async_trait;
use dashcore::secp256k1::{ecdsa, Message, PublicKey, Secp256k1, SecretKey};
use key_wallet::bip32::DerivationPath;
use key_wallet::signer::{Signer as KwSigner, SignerMethod};

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
    // the V0 impl and returns an equivalent transition.
    let signer = FixedKeySigner::new([7u8; 32]);
    let path = DerivationPath::default();

    let st = ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle_and_signer(
        make_chain_asset_lock_proof(),
        &path,
        &signer,
        vec![],
        500_000,
        [0u8; 32],
        vec![],
        [0u8; 64],
        PlatformVersion::latest(),
    )
    .await
    .expect("outer dispatch should succeed");

    let v0 = extract_v0(st);
    assert_eq!(v0.value_balance, 500_000);
    assert_eq!(v0.signature.len(), 65);
}

#[tokio::test]
async fn outer_dispatcher_rejects_unknown_serialization_version() {
    // Synthesise a platform-version whose
    // `shield_from_asset_lock_state_transition.default_current_version`
    // is non-zero, and confirm the dispatcher surfaces
    // `UnknownVersionMismatch` instead of silently coercing to V0.
    // This guards the V0-only assumption baked into the dispatcher
    // so a future V1 introduction can't accidentally route through
    // the wrong impl without an explicit code change.
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
        PlatformVersion::latest(),
    )
    .await
    .expect("builder should succeed");

    let v0 = extract_v0(st);
    assert_eq!(
        v0.value_balance, shield_amount,
        "builder must thread the shield amount into the transition's value_balance",
    );
    assert_eq!(
        v0.signature.len(),
        65,
        "asset-lock signature must be 65-byte recoverable compact",
    );
    assert!(
        !v0.actions.is_empty(),
        "output-only bundle must produce at least one Orchard action",
    );
}
