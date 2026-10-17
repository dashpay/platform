// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The SDK's two signer traits over the embedder's `WalletSigner`, in the
//! shape of `rs-sdk-ffi`'s `VTableSigner`: dpp's identity-key [`Signer`]
//! forwards the key and the signable bytes, and key-wallet's digest signer
//! forwards the asset lock's sighash. Neither sees a private key.

use async_trait::async_trait;
use dash_sdk::dpp::address_funds::AddressWitness;
use dash_sdk::dpp::dashcore::secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use dash_sdk::dpp::dashcore::secp256k1::{ecdsa, Message, PublicKey};
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType};
use dash_sdk::dpp::key_wallet::bip32::DerivationPath;
use dash_sdk::dpp::key_wallet::signer::{Signer as KeyWalletSigner, SignerMethod};
use dash_sdk::dpp::platform_value::BinaryData;
use dash_sdk::dpp::ProtocolError;

use crate::ffi;
use crate::runtime::blocking;

// SAFETY: `WalletSigner` and `ContactKeys` are documented as callable from
// any thread; their callbacks take the wallet's own locks.
unsafe impl Send for ffi::WalletSigner {}
unsafe impl Sync for ffi::WalletSigner {}
unsafe impl Send for ffi::ContactKeys {}
unsafe impl Sync for ffi::ContactKeys {}

/// The embedder's `WalletSigner`.
pub struct WalletSigner(cxx::UniquePtr<ffi::WalletSigner>);

impl WalletSigner {
    pub fn new(signer: cxx::UniquePtr<ffi::WalletSigner>) -> Result<Self, String> {
        if signer.is_null() {
            return Err("a WalletSigner is required".to_string());
        }
        Ok(WalletSigner(signer))
    }

    /// The asset-lock signer for the key the embedder knows as `key_ref`.
    pub fn asset_lock(&self, key_ref: u32) -> AssetLockSigner<'_> {
        AssetLockSigner {
            signer: self,
            key_ref,
        }
    }
}

impl std::fmt::Debug for WalletSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WalletSigner")
    }
}

#[async_trait]
impl Signer<IdentityPublicKey> for WalletSigner {
    async fn sign(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        if !self.can_sign_with(key) {
            return Err(ProtocolError::Generic(format!(
                "key {} is {:?}; the wallet signs ECDSA keys only",
                key.id(),
                key.key_type()
            )));
        }
        let mut signature = [0u8; 65];
        if !blocking(|| {
            self.0.SignIdentityKey(
                key.id(),
                key.key_type() as u8,
                key.data().as_slice(),
                data,
                &mut signature,
            )
        }) {
            return Err(ProtocolError::Generic(format!(
                "the wallet refused to sign with key {}",
                key.id()
            )));
        }
        Ok(BinaryData::new(signature.to_vec()))
    }

    /// No entry point signs for Platform addresses.
    async fn sign_create_witness(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Err(ProtocolError::Generic(
            "the wallet does not sign address witnesses".to_string(),
        ))
    }

    fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
        matches!(
            key.key_type(),
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160
        )
    }
}

/// key-wallet's digest signer for an asset lock's credit-output key. The
/// derivation path is ignored: `key_ref` names the key to the embedder.
#[derive(Debug)]
pub struct AssetLockSigner<'a> {
    signer: &'a WalletSigner,
    key_ref: u32,
}

#[async_trait]
impl KeyWalletSigner for AssetLockSigner<'_> {
    type Error = String;

    fn supported_methods(&self) -> &[SignerMethod] {
        &[SignerMethod::Digest]
    }

    /// Recovers the public key from the compact signature, so the wallet
    /// never exports it.
    async fn sign_ecdsa(
        &self,
        _path: &DerivationPath,
        sighash: [u8; 32],
    ) -> Result<(ecdsa::Signature, PublicKey), String> {
        let mut signature = [0u8; 65];
        if !blocking(|| {
            self.signer
                .0
                .SignAssetLock(self.key_ref, &sighash, &mut signature)
        }) {
            return Err("the wallet refused to sign the asset lock".to_string());
        }
        let recoverable = compressed_key_signature(&signature)?;
        let public_key = recoverable
            .recover_ecdsa(Message::from_digest(sighash))
            .map_err(|e| format!("the asset lock signature recovers no public key: {e}"))?;
        Ok((recoverable.to_standard(), public_key))
    }

    async fn public_key(&self, _path: &DerivationPath) -> Result<PublicKey, String> {
        Err("the wallet does not export the asset lock public key".to_string())
    }
}

/// A compact signature by a compressed key: header 31 + recovery id.
fn compressed_key_signature(signature: &[u8; 65]) -> Result<RecoverableSignature, String> {
    let recovery_id = RecoveryId::try_from(i32::from(signature[0]) - 31).map_err(|_| {
        format!(
            "signature header {:#04x} is not a compressed-key header (31..=34)",
            signature[0]
        )
    })?;
    RecoverableSignature::from_compact(&signature[1..], recovery_id)
        .map_err(|e| format!("malformed asset lock signature: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::dashcore::secp256k1::SecretKey;

    #[test]
    fn an_asset_lock_signature_recovers_its_compressed_key() {
        let key = SecretKey::from_secret_bytes([1; 32]).expect("key");
        let message = Message::from_digest([9; 32]);
        let (recovery_id, compact) =
            RecoverableSignature::sign_ecdsa_recoverable(message, &key).serialize_compact();
        let mut signature = [0u8; 65];
        signature[0] = 31 + u8::from(recovery_id);
        signature[1..].copy_from_slice(&compact);

        let recovered = compressed_key_signature(&signature)
            .and_then(|sig| sig.recover_ecdsa(message).map_err(|e| e.to_string()))
            .expect("recovers");
        assert_eq!(recovered, PublicKey::from_secret_key(&key));
        // An uncompressed-key header is refused.
        signature[0] -= 4;
        assert!(compressed_key_signature(&signature).is_err());
    }
}
