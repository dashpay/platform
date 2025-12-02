use std::collections::BTreeMap;

use dpp::address_funds::PlatformAddress;
use dpp::dashcore::PrivateKey;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, AssetLockProof};
use zeroize::Zeroize;

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
        input_private_keys: Vec<Vec<u8>>,
    },
    /// Use balances held on Platform addresses with explicitly provided nonces.
    AddressesWithNonce {
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<Vec<u8>>,
    },
}

impl Zeroize for FundingSource {
    fn zeroize(&mut self) {
        match self {
            FundingSource::AssetLock {
                asset_lock_private_key,
                ..
            } => {
                asset_lock_private_key.inner.non_secure_erase();
            }
            FundingSource::Addresses {
                input_private_keys, ..
            } => {
                input_private_keys.zeroize();
            }
            FundingSource::AddressesWithNonce {
                input_private_keys, ..
            } => {
                input_private_keys.zeroize();
            }
        }
    }
}

impl Drop for FundingSource {
    fn drop(&mut self) {
        self.zeroize();
    }
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

    pub fn from_addresses(
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Vec<Vec<u8>>,
    ) -> Self {
        Self::Addresses {
            inputs,
            input_private_keys,
        }
    }

    pub fn from_addresses_with_nonce(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<Vec<u8>>,
    ) -> Self {
        Self::AddressesWithNonce {
            inputs,
            input_private_keys,
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
