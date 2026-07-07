//! Signing tests for AddressFundingFromAssetLockTransition.
//!
//! Covers the two construction paths:
//! - `try_from_asset_lock_with_signer_and_private_key` — asset-lock-proof
//!   signature produced from a raw private key held in-process.
//! - `try_from_asset_lock_with_signers` — asset-lock-proof signature
//!   produced by an external `key_wallet::signer::Signer` (Swift / HSM
//!   / hardware-wallet flow). Gated on `core_key_wallet`.
//!
//! These also exercise the outer-enum version dispatcher in
//! `methods/mod.rs`, which routes to the V0 impl based on the
//! `address_funding_from_asset_lock_transition` conversion version.

use std::collections::{BTreeMap, HashMap};

use dashcore::hashes::Hash;
use dashcore::secp256k1::{PublicKey as RawPublicKey, Secp256k1, SecretKey as RawSecretKey};
use dashcore::{OutPoint, PublicKey};
use platform_value::BinaryData;
use platform_version::version::PlatformVersion;

use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
use crate::identity::signer::Signer;
use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::prelude::AssetLockProof;
use crate::serialization::Signable;
use crate::state_transition::address_funding_from_asset_lock_transition::methods::AddressFundingFromAssetLockTransitionMethodsV0;
use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;

/// Per-input P2PKH signer for tests.
#[derive(Debug, Default)]
struct TestAddressSigner {
    keys: HashMap<[u8; 20], (RawSecretKey, PublicKey)>,
}

impl TestAddressSigner {
    fn add_p2pkh(&mut self, seed: [u8; 32]) -> PlatformAddress {
        let secp = Secp256k1::new();
        let secret = RawSecretKey::from_byte_array(&seed).expect("valid secret key");
        let public = PublicKey::new(RawPublicKey::from_secret_key(&secp, &secret));
        let hash = *public.pubkey_hash().as_byte_array();
        self.keys.insert(hash, (secret, public));
        PlatformAddress::P2pkh(hash)
    }
}

#[async_trait::async_trait]
impl Signer<PlatformAddress> for TestAddressSigner {
    async fn sign(&self, key: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        let PlatformAddress::P2pkh(hash) = key else {
            return Err(ProtocolError::Generic(
                "only P2PKH supported in tests".into(),
            ));
        };
        let (secret, _) = self
            .keys
            .get(hash)
            .ok_or_else(|| ProtocolError::Generic(format!("unknown key {}", hex::encode(hash))))?;
        let sig = dashcore::signer::sign(data, secret.as_ref())
            .map_err(|e| ProtocolError::Generic(e.to_string()))?;
        Ok(BinaryData::new(sig.to_vec()))
    }

    async fn sign_create_witness(
        &self,
        key: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let signature = self.sign(key, data).await?;
        Ok(AddressWitness::P2pkh { signature })
    }

    fn can_sign_with(&self, key: &PlatformAddress) -> bool {
        match key {
            PlatformAddress::P2pkh(hash) => self.keys.contains_key(hash),
            _ => false,
        }
    }
}

fn make_chain_asset_lock_proof() -> AssetLockProof {
    AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: 100,
        out_point: OutPoint::from([11u8; 36]),
    })
}

fn extract_v0(state_transition: StateTransition) -> AddressFundingFromAssetLockTransitionV0 {
    let StateTransition::AddressFundingFromAssetLock(AddressFundingFromAssetLockTransition::V0(v0)) =
        state_transition
    else {
        panic!("expected AddressFundingFromAssetLock V0 variant");
    };
    v0
}

#[tokio::test]
async fn try_from_asset_lock_with_signer_and_private_key_signs_single_p2pkh_input() {
    let mut signer = TestAddressSigner::default();
    let input_addr = signer.add_p2pkh([1u8; 32]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input_addr, (0u32, 1_000_000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(PlatformAddress::P2pkh([9u8; 20]), None);

    let asset_lock_private_key = [7u8; 32];

    // Drive the outer-enum dispatcher in `methods/mod.rs`, which routes
    // to the V0 impl in `v0/v0_methods.rs`.
    let st =
        AddressFundingFromAssetLockTransition::try_from_asset_lock_with_signer_and_private_key(
            make_chain_asset_lock_proof(),
            &asset_lock_private_key,
            inputs.clone(),
            outputs,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            &signer,
            0,
            PlatformVersion::latest(),
        )
        .await
        .expect("transition should sign");

    let v0 = extract_v0(st);
    assert_eq!(v0.inputs, inputs);
    assert_eq!(v0.input_witnesses.len(), 1);
    assert!(matches!(
        v0.input_witnesses[0],
        AddressWitness::P2pkh { .. }
    ));
    assert_eq!(
        v0.signature.len(),
        65,
        "asset-lock signature must be 65-byte recoverable compact",
    );

    // The per-input witness must verify against the transition's
    // signable bytes, which is the contract `sign_create_witness`
    // promises.
    let signable = StateTransition::from(v0.clone())
        .signable_bytes()
        .expect("signable_bytes");
    let input_addr = v0.inputs.keys().next().expect("one input");
    input_addr
        .verify_bytes_against_witness(&v0.input_witnesses[0], &signable)
        .expect("witness should verify against signable bytes");
}

#[tokio::test]
async fn try_from_asset_lock_with_signer_and_private_key_signs_multiple_inputs() {
    let mut signer = TestAddressSigner::default();
    let a = signer.add_p2pkh([1u8; 32]);
    let b = signer.add_p2pkh([2u8; 32]);
    let c = signer.add_p2pkh([3u8; 32]);

    let mut inputs = BTreeMap::new();
    inputs.insert(a, (0u32, 500_000u64));
    inputs.insert(b, (0u32, 300_000u64));
    inputs.insert(c, (0u32, 200_000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(PlatformAddress::P2pkh([9u8; 20]), None);

    let st =
        AddressFundingFromAssetLockTransitionV0::try_from_asset_lock_with_signer_and_private_key(
            make_chain_asset_lock_proof(),
            &[7u8; 32],
            inputs.clone(),
            outputs,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            &signer,
            0,
            PlatformVersion::latest(),
        )
        .await
        .expect("transition should sign");

    let v0 = extract_v0(st);

    // One witness per input, in input-iteration order. BTreeMap iteration
    // is stable, so witness[i] must verify against inputs.keys().nth(i).
    assert_eq!(v0.input_witnesses.len(), 3);
    let signable = StateTransition::from(v0.clone())
        .signable_bytes()
        .expect("signable_bytes");
    for (addr, witness) in v0.inputs.keys().zip(v0.input_witnesses.iter()) {
        addr.verify_bytes_against_witness(witness, &signable)
            .expect("witness should verify against signable bytes");
    }
}

// ----------------------------------------------------------------------------
// `try_from_asset_lock_with_signers` — external `key_wallet::signer::Signer`
// path (Swift / hardware wallet / HSM). Gated on `core_key_wallet`.
// ----------------------------------------------------------------------------

#[cfg(feature = "core_key_wallet")]
#[tokio::test]
async fn try_from_asset_lock_with_signers_produces_matching_signature() {
    use async_trait::async_trait;
    use dashcore::secp256k1::{ecdsa, Message};
    use key_wallet::bip32::{DerivationPath, ExtendedPubKey};
    use key_wallet::signer::{ExtendedPubKeySigner, Signer as KwSigner, SignerMethod};

    /// Fixed-key in-memory `key_wallet::signer::Signer`. Mirrors how the
    /// Swift KeychainSigner behaves: derive once, sign atomically. Path
    /// is ignored — the wrapper holds exactly one key.
    #[derive(Debug)]
    struct FixedKeySigner {
        secret: RawSecretKey,
        public: RawPublicKey,
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
        ) -> Result<(ecdsa::Signature, RawPublicKey), Self::Error> {
            let secp = Secp256k1::new();
            let msg = Message::from_digest(sighash);
            Ok((secp.sign_ecdsa(&msg, &self.secret), self.public))
        }

        async fn public_key(&self, _path: &DerivationPath) -> Result<RawPublicKey, Self::Error> {
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

    let secp = Secp256k1::new();
    let asset_lock_secret = RawSecretKey::from_byte_array(&[7u8; 32]).expect("valid secret");
    let asset_lock_public = RawPublicKey::from_secret_key(&secp, &asset_lock_secret);

    let mut input_signer = TestAddressSigner::default();
    let input_addr = input_signer.add_p2pkh([1u8; 32]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input_addr, (0u32, 1_000_000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(PlatformAddress::P2pkh([9u8; 20]), None);

    let asset_lock_signer = FixedKeySigner {
        secret: asset_lock_secret,
        public: asset_lock_public,
    };
    let path = DerivationPath::default();

    let st_signers = AddressFundingFromAssetLockTransition::try_from_asset_lock_with_signers(
        make_chain_asset_lock_proof(),
        &path,
        inputs.clone(),
        outputs.clone(),
        vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
        &input_signer,
        &asset_lock_signer,
        0,
        PlatformVersion::latest(),
    )
    .await
    .expect("with_signers should sign");

    // Cross-check: the byte-parity test in `state_transition::mod` pins
    // that `sign_with_core_signer` produces a byte-identical asset-lock
    // signature to `sign_by_private_key` for the same key. Here we
    // exercise the address-funding-specific path and verify the same
    // 65-byte recoverable-compact shape.
    let v0 = extract_v0(st_signers);
    assert_eq!(v0.input_witnesses.len(), 1);
    assert_eq!(
        v0.signature.len(),
        65,
        "asset-lock signature must be 65-byte recoverable compact",
    );

    // And the per-input witness must verify the same way as the
    // legacy path.
    let signable = StateTransition::from(v0.clone())
        .signable_bytes()
        .expect("signable_bytes");
    let input_addr = v0.inputs.keys().next().expect("one input");
    input_addr
        .verify_bytes_against_witness(&v0.input_witnesses[0], &signable)
        .expect("witness should verify against signable bytes");
}
