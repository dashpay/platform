//! `serde::with` adapters for upstream types that don't (yet) derive
//! their own `Serialize`/`Deserialize`.
//!
//! Compiled only when the crate's `serde` feature is on (see the
//! `#[cfg(feature = "serde")]` gate on the `pub mod` line in
//! `changeset/mod.rs`).

use dash_sdk::platform::address_sync::AddressFunds;
use dpp::balances::credits::Credits;
use dpp::prelude::AddressNonce;
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

/// Adapter for `AddressFunds` (re-exported from `dash-sdk`; no serde
/// derive there). Encodes the scalar fields side-by-side.
pub mod address_funds {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct Wire {
        nonce: AddressNonce,
        balance: Credits,
        /// Height pin (see `AddressFunds::as_of_height`). Defaults to 0
        /// ("unknown provenance") when decoding blobs persisted before
        /// the pin existed.
        #[serde(default)]
        as_of_height: u64,
    }

    pub fn serialize<S: Serializer>(
        value: &AddressFunds,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        Wire {
            nonce: value.nonce,
            balance: value.balance,
            as_of_height: value.as_of_height,
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<AddressFunds, D::Error> {
        let w = Wire::deserialize(deserializer)?;
        Ok(AddressFunds {
            nonce: w.nonce,
            balance: w.balance,
            as_of_height: w.as_of_height,
        })
    }
}

/// Adapter for `Option<AssetLockProof>`.
///
/// `AssetLockProof`'s own serde impl is not self-describing-format
/// agnostic (deserializing it requires `deserialize_any`, which
/// bincode-serde rejects with `AnyNotSupported`), so a proof-carrying
/// `AssetLockEntry` written to the SQLite `lifecycle_blob` could never
/// be read back. Encode the proof as opaque bytes via dpp's own
/// bincode `Encode`/`Decode` instead — the exact encoding the FFI
/// layer already uses for proof round-trips (swift-sdk
/// `PersistentAssetLock.proofBytes`), so every persistence surface
/// speaks one proof wire format.
///
/// Blob compatibility: `None` encodes identically to the old derive
/// (`Option` tag byte 0). Old `Some` blobs were unreadable to begin
/// with (the decode failed before this adapter existed), so no
/// decodable data changes meaning.
pub mod optional_asset_lock_proof {
    use super::*;
    use dpp::prelude::AssetLockProof;

    pub fn serialize<S: Serializer>(
        value: &Option<AssetLockProof>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let bytes: Option<Vec<u8>> = match value {
            Some(proof) => Some(
                dpp::bincode::encode_to_vec(proof, dpp::bincode::config::standard())
                    .map_err(serde::ser::Error::custom)?,
            ),
            None => None,
        };
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<AssetLockProof>, D::Error> {
        let bytes = Option::<Vec<u8>>::deserialize(deserializer)?;
        bytes
            .map(|b| {
                let (proof, consumed) =
                    dpp::bincode::decode_from_slice(&b, dpp::bincode::config::standard())
                        .map_err(serde::de::Error::custom)?;
                // `decode_from_slice` stops at the value's end without
                // rejecting trailing bytes — but this blob holds exactly
                // one proof, so a longer payload is corruption (or
                // smuggled data), not a valid encoding. Fail loudly
                // rather than silently dropping the tail.
                if consumed != b.len() {
                    return Err(serde::de::Error::custom(format!(
                        "asset lock proof blob has {} trailing byte(s)",
                        b.len() - consumed
                    )));
                }
                Ok(proof)
            })
            .transpose()
    }
}

#[cfg(test)]
mod optional_asset_lock_proof_tests {
    use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use dpp::prelude::AssetLockProof;

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Carrier {
        #[serde(with = "super::optional_asset_lock_proof")]
        proof: Option<AssetLockProof>,
    }

    fn chain_proof() -> Option<AssetLockProof> {
        Some(AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 411_495,
            out_point: dashcore::OutPoint::null(),
        }))
    }

    /// Exact-length payloads round-trip; a payload with appended bytes
    /// is corruption and must fail decode instead of silently dropping
    /// the tail.
    #[test]
    fn rejects_trailing_bytes() {
        let encoded = dpp::bincode::serde::encode_to_vec(
            &Carrier {
                proof: chain_proof(),
            },
            dpp::bincode::config::standard(),
        )
        .expect("encode");
        let (decoded, _): (Carrier, _) =
            dpp::bincode::serde::decode_from_slice(&encoded, dpp::bincode::config::standard())
                .expect("exact payload decodes");
        assert_eq!(decoded.proof, chain_proof());

        // Corrupt the blob by appending a byte INSIDE the proof bytes:
        // re-encode with a tampered inner vec. The adapter serializes
        // the proof as Option<Vec<u8>>, so build that shape directly.
        let proof_bytes =
            dpp::bincode::encode_to_vec(chain_proof().unwrap(), dpp::bincode::config::standard())
                .expect("proof bytes");
        let mut padded = proof_bytes;
        padded.push(0xAA);
        let tampered =
            dpp::bincode::serde::encode_to_vec(&(Some(padded),), dpp::bincode::config::standard())
                .expect("encode tampered carrier");
        let result: Result<(Carrier, _), _> =
            dpp::bincode::serde::decode_from_slice(&tampered, dpp::bincode::config::standard());
        assert!(
            result.is_err(),
            "a proof blob with trailing bytes must be rejected"
        );
    }
}
