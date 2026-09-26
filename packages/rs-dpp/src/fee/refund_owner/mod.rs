//! Refund owners
//!
//! Storage refunds are paid back to whoever paid for the bytes that were
//! freed. Historically that was always an identity and the owner was carried
//! as a bare 32-byte identifier. Contract credit buckets can also pay for
//! storage, so the owner of stored bytes is now a typed value whose kind is
//! recorded when the bytes are stored and never inferred from an identifier.
//!
//! The in-memory carrier that GroveDB hands back to Drive keys removed bytes
//! by a 32-byte identifier. [`RefundOwner::removal_key`] gives every owner
//! exactly one such carrier key: an identity's key is its identifier, a
//! contract bucket's key is a domain separated double SHA-256 of the contract
//! id and the bucket position. The recorded owner travels alongside the
//! carrier so that the kind is always read from the record, not from the key.

use crate::util::hash::hash_double;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Position of a credit bucket inside a contract's credit tree.
///
/// Stored as a two-byte big-endian key under the contract's credit tree.
/// The contract credit tree lands with the contract credits work on this
/// release branch; this alias keeps the refund owner independent of its
/// arrival order and is the same width either way.
pub type ContractCreditBucketPosition = u16;

/// Domain separator for the carrier key of a contract bucket refund owner.
///
/// 35 ASCII bytes without a terminator. The trailing `/0` is the derivation
/// generation: a future change to the derivation is a new storage flag type
/// byte, so the derivation carries no separate version field.
///
/// Provisional: proposed as the refund owner encoding allocation and pending
/// the owner's confirmation in the fees workstream register (issue 4689).
pub const REFUND_OWNER_CONTRACT_BUCKET_DOMAIN: &[u8; 35] = b"dash-platform/refund-owner/bucket/0";

/// Size of a contract bucket carrier key preimage: domain, contract id and
/// big-endian position, no length prefixes
const CONTRACT_BUCKET_PREIMAGE_SIZE: usize = 35 + 32 + 2;

/// The system carrier key. Bytes that no owner paid for are sectioned under
/// this key and are never refunded, so it is never a recorded owner.
pub const SYSTEM_REFUND_CARRIER_KEY: [u8; 32] = [0; 32];

/// The recorded refund owner of every carrier key of a storage removal
pub type RefundOwnersByIdentifier = BTreeMap<[u8; 32], RefundOwner>;

/// The owner of stored bytes, recorded when the bytes are stored so that a
/// later removal knows where the refund goes.
///
/// The kind is explicit. It is never derived from the shape or value of an
/// identifier.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Encode, Decode,
)]
pub enum RefundOwner {
    /// An identity paid for the bytes. Refunds are credited to its balance
    /// whether or not it may currently spend.
    Identity(Identifier),
    /// A contract credit bucket paid for the bytes. Refunds are credited to
    /// the bucket on the contract credit path.
    ContractBucket {
        /// The contract whose credit tree holds the bucket.
        contract_id: Identifier,
        /// The bucket's position inside the contract credit tree.
        position: ContractCreditBucketPosition,
    },
}

impl RefundOwner {
    /// The 32-byte key under which this owner's removed bytes are sectioned
    /// in GroveDB's storage removal carrier.
    ///
    /// An identity's key is its identifier verbatim, so every historical
    /// record keeps its key. A contract bucket's key is
    /// `hash_double(domain || contract_id || position_be)` with the domain
    /// from [`REFUND_OWNER_CONTRACT_BUCKET_DOMAIN`], 69 bytes of preimage
    /// with no length prefixes.
    pub fn removal_key(&self) -> [u8; 32] {
        match self {
            RefundOwner::Identity(identity_id) => identity_id.to_buffer(),
            RefundOwner::ContractBucket {
                contract_id,
                position,
            } => {
                let mut preimage = [0u8; CONTRACT_BUCKET_PREIMAGE_SIZE];
                preimage[..35].copy_from_slice(REFUND_OWNER_CONTRACT_BUCKET_DOMAIN);
                preimage[35..67].copy_from_slice(contract_id.as_bytes());
                preimage[67..].copy_from_slice(&position.to_be_bytes());
                hash_double(preimage)
            }
        }
    }

    /// The identity when this owner is an identity.
    pub fn as_identity(&self) -> Option<Identifier> {
        match self {
            RefundOwner::Identity(identity_id) => Some(*identity_id),
            RefundOwner::ContractBucket { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::config;

    #[test]
    fn should_use_the_identity_id_verbatim_as_the_carrier_key() {
        let identity_id = Identifier::from([7u8; 32]);

        assert_eq!(RefundOwner::Identity(identity_id).removal_key(), [7u8; 32]);
    }

    #[test]
    fn should_derive_the_pinned_carrier_key_for_a_contract_bucket() {
        let owner = RefundOwner::ContractBucket {
            contract_id: Identifier::from([0x11u8; 32]),
            position: 7,
        };

        assert_eq!(
            hex::encode(owner.removal_key()),
            "e0cb6d5af10d1ece2cf9dccc39dac999ce9cd59e8c1314bc7017f19cfb9e5a1d"
        );
    }

    #[test]
    fn should_derive_the_bucket_key_from_the_documented_preimage() {
        let contract_id = Identifier::from([0x11u8; 32]);
        let owner = RefundOwner::ContractBucket {
            contract_id,
            position: 7,
        };

        let mut preimage = REFUND_OWNER_CONTRACT_BUCKET_DOMAIN.to_vec();
        preimage.extend_from_slice(&[0x11u8; 32]);
        preimage.extend_from_slice(&7u16.to_be_bytes());
        assert_eq!(preimage.len(), 69);

        assert_eq!(owner.removal_key(), hash_double(preimage));
    }

    #[test]
    fn should_give_distinct_carrier_keys_to_distinct_buckets() {
        let contract_a = Identifier::from([1u8; 32]);
        let contract_b = Identifier::from([2u8; 32]);

        let a0 = RefundOwner::ContractBucket {
            contract_id: contract_a,
            position: 0,
        }
        .removal_key();
        let a1 = RefundOwner::ContractBucket {
            contract_id: contract_a,
            position: 1,
        }
        .removal_key();
        let b0 = RefundOwner::ContractBucket {
            contract_id: contract_b,
            position: 0,
        }
        .removal_key();

        assert_ne!(a0, a1);
        assert_ne!(a0, b0);
        assert_ne!(a1, b0);
        assert_ne!(a0, SYSTEM_REFUND_CARRIER_KEY);
    }

    #[test]
    fn should_never_read_a_bucket_key_as_the_contract_id() {
        let contract_id = Identifier::from([9u8; 32]);
        let owner = RefundOwner::ContractBucket {
            contract_id,
            position: 0,
        };

        assert_ne!(owner.removal_key(), contract_id.to_buffer());
        assert_eq!(owner.as_identity(), None);
        assert_eq!(
            RefundOwner::Identity(contract_id).as_identity(),
            Some(contract_id)
        );
    }

    #[test]
    fn should_round_trip_both_owner_kinds_through_bincode() {
        let owners = [
            RefundOwner::Identity(Identifier::from([3u8; 32])),
            RefundOwner::ContractBucket {
                contract_id: Identifier::from([4u8; 32]),
                position: u16::MAX,
            },
        ];

        for owner in owners {
            let bytes = bincode::encode_to_vec(owner, config::standard()).expect("should encode");
            let (decoded, _): (RefundOwner, usize) =
                bincode::decode_from_slice(&bytes, config::standard()).expect("should decode");
            assert_eq!(decoded, owner);
        }
    }
}
