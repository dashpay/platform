// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The two signer adapters dpp's builders drive over the embedder's
//! `WalletSigner`.
//!
//! [`BytesSigner`] implements dpp's identity-key signer: it forwards the
//! full signable preimage to `SignForKey`, so the embedder hashes the bytes
//! itself and can check what it is signing before it does. The compact
//! signature it returns is exactly what `dashcore::signer::sign` would
//! produce from the raw key. [`AssetLockSigner`] implements `key-wallet`'s
//! digest signer for the asset-lock outpoint key of an identity
//! registration: the only digest path, because that trait offers no
//! preimage. It recovers the public key from the recoverable signature, so
//! the embedder never has to export it.
//!
//! Both traits require `Send + Sync`. The builders are driven to completion
//! on the calling thread by a local executor, so the `WalletSigner` is only
//! ever called on the thread that called the builder; the embedder's signer
//! must nonetheless be safe to call from any thread, which is the documented
//! contract of `WalletSigner` and the basis of its `Send + Sync` impls.

use async_trait::async_trait;
use dash_sdk::dpp::address_funds::AddressWitness;
use dash_sdk::dpp::dashcore::secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use dash_sdk::dpp::dashcore::secp256k1::{ecdsa, Message, PublicKey, Secp256k1};
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType};
use dash_sdk::dpp::key_wallet::bip32::DerivationPath;
use dash_sdk::dpp::key_wallet::signer::{Signer as KeyWalletSigner, SignerMethod};
use dash_sdk::dpp::platform_value::BinaryData;
use dash_sdk::dpp::ProtocolError;

use crate::ffi;

const COMPACT_SIGNATURE_SIZE: usize = 65;

/// What the builders ask of the embedder's wallet: the two callbacks of the
/// C++ `WalletSigner`, as a Rust trait so tests can supply a wallet without
/// a C++ shim. Both return `false` to refuse; a refusal fails the build.
pub trait SignerCallbacks: Send + Sync {
    /// Signs the full signable preimage of a state transition with identity
    /// key `key_id`; the wallet hashes it (double SHA256) itself and answers
    /// with a 65-byte compact recoverable ECDSA signature.
    fn sign_for_key(&self, key_id: u32, signable: &[u8], signature: &mut Vec<u8>) -> bool;
    /// Signs the 32-byte double SHA256 the builder computed with the
    /// asset lock's outpoint key, the same compact form.
    fn sign_asset_lock_sighash(&self, sighash: &[u8; 32], signature: &mut Vec<u8>) -> bool;
}

// SAFETY: `WalletSigner` is documented as callable from any thread (its
// callbacks take the wallet's own lock), and the builders only ever call it
// on the thread that called them (`futures::executor::block_on`). The impls
// exist so the signer satisfies dpp's `Send + Sync` signer bounds.
unsafe impl Send for ffi::WalletSigner {}
unsafe impl Sync for ffi::WalletSigner {}

impl SignerCallbacks for ffi::WalletSigner {
    fn sign_for_key(&self, key_id: u32, signable: &[u8], signature: &mut Vec<u8>) -> bool {
        self.SignForKey(key_id, signable, signature)
    }

    fn sign_asset_lock_sighash(&self, sighash: &[u8; 32], signature: &mut Vec<u8>) -> bool {
        self.SignAssetLockSighash(sighash, signature)
    }
}

/// dpp's signer traits require `Debug`; the embedder's signer is opaque.
impl std::fmt::Debug for dyn SignerCallbacks + '_ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SignerCallbacks")
    }
}

fn compact(signature: Vec<u8>, what: &str) -> Result<[u8; COMPACT_SIGNATURE_SIZE], String> {
    signature.try_into().map_err(|signature: Vec<u8>| {
        format!(
            "{what}: unexpected signature size {} (want {COMPACT_SIGNATURE_SIZE})",
            signature.len()
        )
    })
}

/// dpp `Signer<IdentityPublicKey>` over `SignForKey`.
#[derive(Debug)]
pub struct BytesSigner<'a>(pub &'a dyn SignerCallbacks);

impl BytesSigner<'_> {
    fn sign_bytes(&self, key_id: u32, data: &[u8]) -> Result<BinaryData, String> {
        let mut signature = Vec::new();
        if !self.0.sign_for_key(key_id, data, &mut signature) {
            return Err(format!("the wallet refused to sign with key {key_id}"));
        }
        compact(signature, &format!("key {key_id}"))
            .map(|signature| BinaryData::new(signature.to_vec()))
    }
}

#[async_trait]
impl Signer<IdentityPublicKey> for BytesSigner<'_> {
    async fn sign(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        if !self.can_sign_with(key) {
            return Err(ProtocolError::Generic(format!(
                "key {} is {:?}; the wallet only produces ECDSA signatures",
                key.id(),
                key.key_type()
            )));
        }
        self.sign_bytes(key.id(), data)
            .map_err(ProtocolError::Generic)
    }

    async fn sign_create_witness(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let signature = self.sign(key, data).await?;
        Ok(AddressWitness::P2pkh { signature })
    }

    /// dpp checks purpose, security level and disabled state, but not the
    /// key type; the wallet answers with compact secp256k1 signatures only.
    fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
        matches!(
            key.key_type(),
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160
        )
    }
}

/// `key_wallet::signer::Signer` over `SignAssetLockSighash`. The derivation
/// path is ignored: the embedder bound the signer to the flow's funding key.
#[derive(Debug)]
pub struct AssetLockSigner<'a>(pub &'a dyn SignerCallbacks);

#[async_trait]
impl KeyWalletSigner for AssetLockSigner<'_> {
    type Error = String;

    fn supported_methods(&self) -> &[SignerMethod] {
        &[SignerMethod::Digest]
    }

    async fn sign_ecdsa(
        &self,
        _path: &DerivationPath,
        sighash: [u8; 32],
    ) -> Result<(ecdsa::Signature, PublicKey), Self::Error> {
        let mut signature = Vec::new();
        if !self.0.sign_asset_lock_sighash(&sighash, &mut signature) {
            return Err("the wallet refused to sign the asset lock".to_string());
        }
        let signature = compact(signature, "asset lock key")?;
        // The compact header is 27 + recovery id + 4 for a compressed key
        // (`dashcore::signer::CompactSignature`); the asset lock's outpoint
        // key is compressed (Core funds P2PKH of the compressed key), so the
        // uncompressed range 27..=30 is refused.
        let recovery_id = RecoveryId::try_from(i32::from(signature[0]) - 27 - 4).map_err(|_| {
            format!(
                "asset lock signature header {:#04x} is not a compressed-key recovery header \
                 (31..=34)",
                signature[0]
            )
        })?;
        let recoverable = RecoverableSignature::from_compact(&signature[1..], recovery_id)
            .map_err(|e| format!("asset lock signature is malformed: {e}"))?;
        let public_key = Secp256k1::new()
            .recover_ecdsa(&Message::from_digest(sighash), &recoverable)
            .map_err(|e| format!("asset lock signature does not recover a public key: {e}"))?;
        Ok((recoverable.to_standard(), public_key))
    }

    /// Never called by `sign_with_core_signer`, and the embedder does not
    /// export keys.
    async fn public_key(&self, _path: &DerivationPath) -> Result<PublicKey, Self::Error> {
        Err("the wallet does not export the asset lock public key".to_string())
    }
}
