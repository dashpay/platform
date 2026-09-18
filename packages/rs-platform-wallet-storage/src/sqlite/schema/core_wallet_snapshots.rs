//! Versioned binary snapshots of public Core wallet state.

use key_wallet::managed_account::address_pool::AddressPoolType;
use key_wallet::managed_account::managed_account_type::ManagedAccountType;
use key_wallet::managed_account::ManagedCoreKeysAccount;
use key_wallet::wallet::ManagedWalletInfo;
use key_wallet::{AddressPool, DerivationPath, Network};
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::{params, Connection, Transaction};

use super::blob;
use crate::sqlite::error::WalletStorageError;

// ManagedWalletInfo holds public account state; signing material belongs to Wallet.
blob::impl_persistable_blob!(ManagedWalletInfo, ManagedCoreKeysAccount);

const FORMAT_VERSION: i64 = 1;

pub(crate) fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    snapshot: Option<&ManagedWalletInfo>,
    invalidated: bool,
) -> Result<(), WalletStorageError> {
    if let Some(snapshot) = snapshot {
        validate(tx, wallet_id, snapshot)?;
        let bytes = blob::encode(snapshot)?;
        blob::check_size(bytes.len() as i64)?;
        tx.execute(
            "INSERT INTO core_wallet_snapshots (wallet_id, format_version, snapshot_blob, layout_marker)
             VALUES (?1, ?2, ?3, ?4) ON CONFLICT(wallet_id) DO UPDATE SET
             format_version = excluded.format_version, snapshot_blob = excluded.snapshot_blob, layout_marker = excluded.layout_marker",
            params![wallet_id.as_slice(), FORMAT_VERSION, bytes, layout_marker()?],
        )?;
    } else if invalidated {
        tx.execute(
            "DELETE FROM core_wallet_snapshots WHERE wallet_id = ?1",
            [wallet_id.as_slice()],
        )?;
    }
    Ok(())
}

pub(crate) fn load(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Option<ManagedWalletInfo>, WalletStorageError> {
    let mut stmt = conn.prepare_cached(
        "SELECT format_version, length(snapshot_blob), snapshot_blob, length(layout_marker), layout_marker
         FROM core_wallet_snapshots WHERE wallet_id = ?1",
    )?;
    let mut rows = stmt.query([wallet_id.as_slice()])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let version: i64 = row.get(0)?;
    if version != FORMAT_VERSION {
        return Err(WalletStorageError::blob_decode(
            "unsupported Core wallet snapshot version",
        ));
    }
    blob::check_size(row.get(1)?)?;
    blob::check_size(row.get(3)?)?;
    let stored_layout: Vec<u8> = row.get(4)?;
    if stored_layout != layout_marker()? {
        return Err(WalletStorageError::blob_decode(
            "incompatible Core wallet snapshot layout",
        ));
    }
    let bytes: Vec<u8> = row.get(2)?;
    let snapshot: ManagedWalletInfo = blob::decode(&bytes)?;
    validate(conn, wallet_id, &snapshot)?;
    Ok(Some(snapshot))
}

fn validate(
    conn: &Connection,
    wallet_id: &WalletId,
    snapshot: &ManagedWalletInfo,
) -> Result<(), WalletStorageError> {
    if snapshot.wallet_id != *wallet_id {
        return Err(WalletStorageError::WalletIdMismatch {
            expected: *wallet_id,
            found: snapshot.wallet_id,
        });
    }
    let Some((network, _)) = super::wallets::fetch(conn, wallet_id)? else {
        return Err(WalletStorageError::blob_decode(
            "Core snapshot has no registered wallet",
        ));
    };
    if super::wallets::parse_network(&network) != Some(snapshot.network) {
        return Err(WalletStorageError::blob_decode(
            "Core snapshot network differs from its wallet",
        ));
    }
    Ok(())
}

// Probe the upstream serde layout, including Cargo features unified by other crates.
fn layout_marker() -> Result<Vec<u8>, WalletStorageError> {
    let account = ManagedCoreKeysAccount::new(
        ManagedAccountType::IdentityRegistration {
            addresses: AddressPool::new_without_generation(
                DerivationPath::default(),
                AddressPoolType::External,
                0,
                Network::Testnet,
            ),
        },
        Network::Testnet,
    );
    blob::encode(&account)
}
