use std::collections::BTreeMap;

use dpp::address_funds::PlatformAddress;
use dpp::dashcore::PrivateKey;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, AssetLockProof};
use zeroize::Zeroizing;

/// Generic funding sources for credit-backed transitions.
pub enum FundingSource {
    /// Use an asset lock proof/private key pair.
    AssetLock {
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
    },
    /// Use balances held on Platform addresses (nonces fetched automatically).
    Addresses {
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Zeroizing<Vec<Vec<u8>>>,
    },
    /// Use balances held on Platform addresses with explicitly provided nonces.
    AddressesWithNonce {
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Zeroizing<Vec<Vec<u8>>>,
    },
}

impl FundingSource {
    pub fn from_asset_lock(
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
    ) -> Self {
        Self::AssetLock {
            asset_lock_proof,
            asset_lock_private_key,
        }
    }

    pub fn from_addresses<Z: Into<Zeroizing<Vec<Vec<u8>>>>>(
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Z,
    ) -> Self {
        Self::Addresses {
            inputs,
            input_private_keys: input_private_keys.into(),
        }
    }

    pub fn from_addresses_with_nonce<Z: Into<Zeroizing<Vec<Vec<u8>>>>>(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Z,
    ) -> Self {
        Self::AddressesWithNonce {
            inputs,
            input_private_keys: input_private_keys.into(),
        }
    }
}

impl From<(AssetLockProof, PrivateKey)> for FundingSource {
    fn from(value: (AssetLockProof, PrivateKey)) -> Self {
        Self::from_asset_lock(value.0, value.1)
    }
}

impl From<(BTreeMap<PlatformAddress, Credits>, Vec<Vec<u8>>)> for FundingSource {
    fn from(value: (BTreeMap<PlatformAddress, Credits>, Vec<Vec<u8>>)) -> Self {
        Self::from_addresses(value.0, value.1)
    }
}

impl
    From<(
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        Vec<Vec<u8>>,
    )> for FundingSource
{
    fn from(
        value: (
            BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
            Vec<Vec<u8>>,
        ),
    ) -> Self {
        Self::from_addresses_with_nonce(value.0, value.1)
    }
}
