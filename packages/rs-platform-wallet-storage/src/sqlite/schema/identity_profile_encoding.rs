//! Frozen pre-address profile records. Do not edit their field order.
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

pub(super) fn decode_identity(
    payload: &[u8],
    format: i64,
) -> Result<IdentityEntry, WalletStorageError> {
    match format {
        1 => blob::decode(payload),
        0 => {
            let old: LegacyIdentityEntry = blob::decode(payload)?;
            Ok(IdentityEntry {
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
            })
        }
        _ => Err(WalletStorageError::blob_decode(
            "unsupported identity profile encoding",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn old_profile() -> LegacyProfile {
        LegacyProfile {
            display_name: Some("Alice".into()),
            bio: Some("hello".into()),
            avatar_url: None,
            avatar_hash: None,
            avatar_fingerprint: None,
            public_message: Some("hello".into()),
        }
    }
    #[test]
    fn should_read_old_owned_and_contact_profiles_without_losing_following_fields() {
        let id = Identifier::from([1; 32]);
        let old = LegacyIdentityEntry {
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
                LegacyContactProfile {
                    profile: Some(old_profile()),
                    checked_at_ms: 321,
                },
            )]
            .into(),
            ignored_senders: [Identifier::from([3; 32])].into(),
        };
        let encoded = blob::encode(&old).unwrap();
        // Apply the real V7 -> V8 migration around an existing binary record.
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::runner()
            .set_target(refinery::Target::Version(7))
            .run(&mut conn)
            .unwrap();
        conn.execute("INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)", rusqlite::params![[2u8; 32].as_slice()]).unwrap();
        conn.execute("INSERT INTO identities (identity_id, wallet_id, entry_blob, tombstoned) VALUES (?1, ?2, ?3, 0)", rusqlite::params![id.as_slice(), [2u8; 32].as_slice(), &encoded]).unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let (restored, tombstoned) =
            super::super::identities::fetch(&conn, &[2; 32], &id.to_buffer())
                .unwrap()
                .unwrap();
        assert!(!tombstoned);
        assert_eq!(
            conn.query_row("SELECT entry_format FROM identities", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(restored.balance, 123);
        assert_eq!(
            restored
                .dashpay_profile
                .as_ref()
                .unwrap()
                .display_name
                .as_deref(),
            Some("Alice")
        );
        assert!(restored
            .dashpay_profile
            .as_ref()
            .unwrap()
            .shielded_address
            .is_none());
        assert_eq!(restored.contact_profiles[&id].checked_at_ms, 321);
        assert!(restored
            .ignored_senders
            .contains(&Identifier::from([3; 32])));
        let mut current = restored;
        current.dashpay_profile.as_mut().unwrap().shielded_address = Some(vec![9; 43]);
        let tx = conn.transaction().unwrap();
        let changes = platform_wallet::changeset::IdentityChangeSet {
            identities: [(id, current.clone())].into(),
            ..Default::default()
        };
        super::super::identities::apply(&tx, &[2; 32], &changes).unwrap();
        tx.commit().unwrap();
        assert_eq!(
            conn.query_row("SELECT entry_format FROM identities", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            super::super::identities::fetch(&conn, &[2; 32], &id.to_buffer())
                .unwrap()
                .unwrap()
                .0,
            current
        );
        let encoded = blob::encode(&current).unwrap();
        assert_eq!(decode_identity(&encoded, 1).unwrap(), current);
        assert!(decode_identity(&encoded[..encoded.len() - 1], 1).is_err());
        assert!(decode_identity(&encoded, 2).is_err());
    }
}
