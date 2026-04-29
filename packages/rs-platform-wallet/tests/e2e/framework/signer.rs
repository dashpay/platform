//! Seed-backed `Signer<PlatformAddress>` for the e2e harness. Composes
//! `simple_signer::SimpleSigner` populated via DIP-17
//! (`m/9'/coin_type'/17'/account'/key_class'/index`) eager derivation.

use async_trait::async_trait;
use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::identity::signer::Signer;
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use key_wallet::Network;
use simple_signer::signer::SimpleSigner;

use super::{FrameworkError, FrameworkResult};

/// DIP-17 default account / key-class for clear-funds platform
/// payments. Matches `WalletAccountCreationOptions::Default`.
const DEFAULT_ACCOUNT_INDEX: u32 = 0;
const DEFAULT_KEY_CLASS: u32 = 0;

/// Default gap window pre-derived at construction
/// (`key-wallet`'s `DIP17_GAP_LIMIT`).
pub const DEFAULT_GAP_LIMIT: u32 = 20;

/// Resolves `Signer<PlatformAddress>::sign` against a seed-derived
/// key cache. Construction is fallible; the hot path is sync.
#[derive(Clone, Debug, Default)]
pub struct SeedBackedPlatformAddressSigner {
    inner: SimpleSigner,
}

impl SeedBackedPlatformAddressSigner {
    /// Pre-derive the [`DEFAULT_GAP_LIMIT`] window for `seed_bytes`
    /// on `network`. Use [`Self::new_with_gap`] for a custom window.
    pub fn new(seed_bytes: &[u8; 64], network: Network) -> FrameworkResult<Self> {
        Self::new_with_gap(seed_bytes, network, DEFAULT_GAP_LIMIT)
    }

    /// Same as [`Self::new`] but with an explicit gap-window size.
    pub fn new_with_gap(
        seed_bytes: &[u8; 64],
        network: Network,
        gap_limit: u32,
    ) -> FrameworkResult<Self> {
        let inner = SimpleSigner::from_seed_for_platform_address_account(
            seed_bytes,
            network,
            DEFAULT_ACCOUNT_INDEX,
            DEFAULT_KEY_CLASS,
            gap_limit,
        )
        .map_err(|err| FrameworkError::Wallet(format!("SeedBackedPlatformAddressSigner: {err}")))?;
        Ok(Self { inner })
    }

    /// Number of pre-derived keys in the cache.
    pub fn cached_key_count(&self) -> usize {
        self.inner.address_private_keys.len()
    }
}

#[async_trait]
impl Signer<PlatformAddress> for SeedBackedPlatformAddressSigner {
    async fn sign(&self, key: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        Signer::<PlatformAddress>::sign(&self.inner, key, data).await
    }

    async fn sign_create_witness(
        &self,
        key: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Signer::<PlatformAddress>::sign_create_witness(&self.inner, key, data).await
    }

    fn can_sign_with(&self, key: &PlatformAddress) -> bool {
        Signer::<PlatformAddress>::can_sign_with(&self.inner, key)
    }
}
