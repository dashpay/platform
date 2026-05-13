//! `serde::with` adapters for upstream types that don't (yet) derive
//! their own `Serialize`/`Deserialize`.
//!
//! Compiled only when the crate's `serde` feature is on (see the
//! `#[cfg(feature = "serde")]` gate on the `pub mod` line in
//! `changeset/mod.rs`).

use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Adapter for `AssetLockFundingType` (upstream has no serde derive).
///
/// Encodes each variant as a stable u8 tag — same tag space the
/// hand-rolled `BlobWriter` used before the serde swap, kept for
/// forward/backward compatibility of on-disk blobs.
pub mod asset_lock_funding_type {
    use super::*;

    pub fn serialize<S: Serializer>(
        value: &AssetLockFundingType,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let tag: u8 = match value {
            AssetLockFundingType::IdentityRegistration => 0,
            AssetLockFundingType::IdentityTopUp => 1,
            AssetLockFundingType::IdentityTopUpNotBound => 2,
            AssetLockFundingType::IdentityInvitation => 3,
            AssetLockFundingType::AssetLockAddressTopUp => 4,
            AssetLockFundingType::AssetLockShieldedAddressTopUp => 5,
        };
        tag.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<AssetLockFundingType, D::Error> {
        let tag = u8::deserialize(deserializer)?;
        Ok(match tag {
            0 => AssetLockFundingType::IdentityRegistration,
            1 => AssetLockFundingType::IdentityTopUp,
            2 => AssetLockFundingType::IdentityTopUpNotBound,
            3 => AssetLockFundingType::IdentityInvitation,
            4 => AssetLockFundingType::AssetLockAddressTopUp,
            5 => AssetLockFundingType::AssetLockShieldedAddressTopUp,
            other => {
                return Err(serde::de::Error::custom(format!(
                    "unknown AssetLockFundingType tag: {other}"
                )))
            }
        })
    }
}
