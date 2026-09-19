//! Frozen pre-V019 profile records, and the stamped decoders that route each
//! `identities.entry_blob` / `dashpay_profiles.profile_blob` row to the shape
//! it carries. Do not edit the legacy field order.
//!
//! `DashPayProfile` gained its payment addresses as trailing `Option` fields.
//! Under `bincode::serde` a struct is a fixed-arity positional record, so a
//! record written before the widening is never "short": the trailing fields
//! are simply absent, and decoding it against the current shape reads the
//! next field's bytes as an address. The row stamp (V019) is what says which
//! shape to decode; `serde(default)` on the new fields serves map formats
//! such as JSON only.
use super::blob;
use crate::sqlite::error::WalletStorageError;
use dpp::fee::Credits;
use dpp::prelude::{Identifier, Revision};
use platform_wallet::changeset::IdentityEntry;
use platform_wallet::wallet::identity::types::block_time::BlockTime;
use platform_wallet::wallet::identity::PaymentEntry;
use platform_wallet::{ContactProfileEntry, DashPayProfile, DpnsNameInfo, IdentityStatus};
use std::collections::{BTreeMap, BTreeSet};

#[derive(serde::Serialize, serde::Deserialize)]
struct LegacyProfile {
    display_name: Option<String>,
    bio: Option<String>,
    avatar_url: Option<String>,
    avatar_hash: Option<[u8; 32]>,
    avatar_fingerprint: Option<[u8; 8]>,
    public_message: Option<String>,
}
impl From<LegacyProfile> for DashPayProfile {
    fn from(p: LegacyProfile) -> Self {
        Self {
            display_name: p.display_name,
            bio: p.bio,
            avatar_url: p.avatar_url,
            avatar_hash: p.avatar_hash,
            avatar_fingerprint: p.avatar_fingerprint,
            public_message: p.public_message,
            ..Default::default()
        }
    }
}
impl From<DashPayProfile> for LegacyProfile {
    fn from(p: DashPayProfile) -> Self {
        Self {
            display_name: p.display_name,
            bio: p.bio,
            avatar_url: p.avatar_url,
            avatar_hash: p.avatar_hash,
            avatar_fingerprint: p.avatar_fingerprint,
            public_message: p.public_message,
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
struct LegacyContactProfile {
    profile: Option<LegacyProfile>,
    checked_at_ms: u64,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct LegacyIdentityEntry {
    pub id: Identifier,
    pub balance: Credits,
    pub revision: Revision,
    pub identity_index: Option<u32>,
    pub last_updated_balance_block_time: Option<BlockTime>,
    pub last_synced_keys_block_time: Option<BlockTime>,
    pub dpns_names: Vec<DpnsNameInfo>,
    pub contested_dpns_names: Vec<String>,
    pub status: IdentityStatus,
    pub wallet_id: Option<[u8; 32]>,
    pub dashpay_profile: Option<LegacyProfile>,
    pub dashpay_payments: BTreeMap<String, PaymentEntry>,
    pub contact_profiles: BTreeMap<Identifier, LegacyContactProfile>,
    pub ignored_senders: BTreeSet<Identifier>,
}
impl From<LegacyIdentityEntry> for IdentityEntry {
    fn from(old: LegacyIdentityEntry) -> Self {
        IdentityEntry {
            id: old.id,
            balance: old.balance,
            revision: old.revision,
            identity_index: old.identity_index,
            last_updated_balance_block_time: old.last_updated_balance_block_time,
            last_synced_keys_block_time: old.last_synced_keys_block_time,
            dpns_names: old.dpns_names,
            contested_dpns_names: old.contested_dpns_names,
            status: old.status,
            wallet_id: old.wallet_id,
            dashpay_profile: old.dashpay_profile.map(Into::into),
            dashpay_payments: old.dashpay_payments,
            contact_profiles: old
                .contact_profiles
                .into_iter()
                .map(|(id, entry)| {
                    (
                        id,
                        ContactProfileEntry {
                            profile: entry.profile.map(Into::into),
                            checked_at_ms: entry.checked_at_ms,
                        },
                    )
                })
                .collect(),
            ignored_senders: old.ignored_senders,
        }
    }
}
impl From<IdentityEntry> for LegacyIdentityEntry {
    fn from(entry: IdentityEntry) -> Self {
        Self {
            id: entry.id,
            balance: entry.balance,
            revision: entry.revision,
            identity_index: entry.identity_index,
            last_updated_balance_block_time: entry.last_updated_balance_block_time,
            last_synced_keys_block_time: entry.last_synced_keys_block_time,
            dpns_names: entry.dpns_names,
            contested_dpns_names: entry.contested_dpns_names,
            status: entry.status,
            wallet_id: entry.wallet_id,
            dashpay_profile: entry.dashpay_profile.map(Into::into),
            dashpay_payments: entry.dashpay_payments,
            contact_profiles: entry
                .contact_profiles
                .into_iter()
                .map(|(id, entry)| {
                    (
                        id,
                        LegacyContactProfile {
                            profile: entry.profile.map(Into::into),
                            checked_at_ms: entry.checked_at_ms,
                        },
                    )
                })
                .collect(),
            ignored_senders: entry.ignored_senders,
        }
    }
}

// Test-only: production never writes the legacy shapes, but migration
// fixtures have to produce the bytes a pre-V019 writer produced. PUBLIC
// material only, like the current shapes they mirror.
#[cfg(any(test, feature = "__test-helpers"))]
super::blob::impl_persistable_blob!(LegacyProfile, LegacyIdentityEntry);

/// The bytes a pre-V019 writer stored for `profile`: the record without the
/// payment-address fields. For fixtures of databases written before V019.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn encode_legacy_profile(profile: &DashPayProfile) -> Result<Vec<u8>, WalletStorageError> {
    blob::encode(&LegacyProfile::from(profile.clone()))
}

/// The bytes a pre-V019 writer stored for `entry`: its own and its contacts'
/// profiles in the pre-payment-address shape. For fixtures of databases
/// written before V019.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn encode_legacy_identity(entry: &IdentityEntry) -> Result<Vec<u8>, WalletStorageError> {
    blob::encode(&LegacyIdentityEntry::from(entry.clone()))
}

/// Decode a `dashpay_profiles.profile_blob` using its V019 `profile_format`
/// stamp: zero is the pre-address shape, one includes payment addresses. Never
/// fall back to the legacy shape after a current-format decoding error.
pub fn decode_profile(payload: &[u8], format: i64) -> Result<DashPayProfile, WalletStorageError> {
    match format {
        0 => blob::decode::<LegacyProfile>(payload).map(Into::into),
        1 => blob::decode(payload),
        _ => Err(WalletStorageError::blob_decode(
            "unsupported profile encoding",
        )),
    }
}

/// Decode an `identities.entry_blob` using its `entry_format` stamp, preserving
/// the pre-address owned and contact profile shapes for format zero.
pub fn decode_identity(payload: &[u8], format: i64) -> Result<IdentityEntry, WalletStorageError> {
    match format {
        1 => blob::decode(payload),
        0 => blob::decode::<LegacyIdentityEntry>(payload).map(Into::into),
        _ => Err(WalletStorageError::blob_decode(
            "unsupported identity profile encoding",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn old_profile() -> DashPayProfile {
        DashPayProfile {
            display_name: Some("Alice".into()),
            bio: Some("hello".into()),
            public_message: Some("hello".into()),
            ..Default::default()
        }
    }

    #[test]
    fn should_decode_standalone_profiles_by_format_without_fallback() {
        let legacy = encode_legacy_profile(&old_profile()).unwrap();
        let mut profile = decode_profile(&legacy, 0).unwrap();
        assert_eq!(profile, old_profile());
        assert!(profile.core_payment_address.is_none());
        assert!(profile.platform_payment_address.is_none());
        assert!(profile.shielded_address.is_none());
        assert!(decode_profile(&legacy, 1).is_err());
        profile.shielded_address = Some(vec![9; 43]);
        let current = blob::encode(&profile).unwrap();
        assert_eq!(decode_profile(&current, 1).unwrap(), profile);
        assert!(decode_profile(&current, 0).is_err());
        assert!(decode_profile(&current, 2).is_err());
        assert!(decode_profile(&current[..current.len() - 1], 1).is_err());
    }

    /// The legacy identity record round-trips through the stamped decoder
    /// with every field after the embedded profiles intact: the profile is
    /// positional, so a shape mismatch would shift `dashpay_payments`,
    /// `contact_profiles` and `ignored_senders`, not just drop an address.
    #[test]
    fn should_read_old_owned_and_contact_profiles_without_losing_following_fields() {
        let id = Identifier::from([1; 32]);
        let entry = IdentityEntry {
            id,
            balance: 123,
            revision: 2,
            identity_index: Some(0),
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            dpns_names: vec![],
            contested_dpns_names: vec![],
            status: IdentityStatus::Unknown,
            wallet_id: Some([2; 32]),
            dashpay_profile: Some(old_profile()),
            dashpay_payments: BTreeMap::new(),
            contact_profiles: [(
                id,
                ContactProfileEntry {
                    profile: Some(old_profile()),
                    checked_at_ms: 321,
                },
            )]
            .into(),
            ignored_senders: [Identifier::from([3; 32])].into(),
        };
        let legacy = encode_legacy_identity(&entry).unwrap();
        assert_eq!(decode_identity(&legacy, 0).unwrap(), entry);
        // Current bytes carry three more `None`s per embedded profile.
        let current = blob::encode(&entry).unwrap();
        assert_eq!(current.len(), legacy.len() + 6);
        assert_eq!(decode_identity(&current, 1).unwrap(), entry);
        assert!(decode_identity(&current[..current.len() - 1], 1).is_err());
        assert!(decode_identity(&current, 2).is_err());
        assert!(decode_identity(&legacy, 2).is_err());
    }
}
