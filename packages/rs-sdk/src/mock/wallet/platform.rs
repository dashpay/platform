//! Platform wallet using [SimpleSigner].
use dpp::{
    identity::{signer::Signer, IdentityPublicKey},
    platform_value::BinaryData,
    ProtocolError,
};
use simple_signer::signer::SimpleSigner;

use crate::wallet::PlatformWallet;

/// Platform wallet using [SimpleSigner].
#[derive(Debug)]
pub struct PlatformSignerWallet {
    /// Signer used to sign Platform transactions.
    pub signer: SimpleSigner,
}

impl PlatformSignerWallet {
    /// Create new Platform wallet using [SimpleSigner].
    pub fn new() -> Self {
        Self {
            signer: SimpleSigner::default(),
        }
    }
}

impl Default for PlatformSignerWallet {
    fn default() -> Self {
        Self::new()
    }
}

impl From<SimpleSigner> for PlatformSignerWallet {
    fn from(signer: SimpleSigner) -> Self {
        Self { signer }
    }
}

impl Signer for PlatformSignerWallet {
    fn sign(
        &self,
        pubkey: &IdentityPublicKey,
        message: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        self.signer.sign(pubkey, message)
    }
}

impl PlatformWallet for PlatformSignerWallet {}
