//! `platform_addresses` + `platform_address_sync` writers + readers.

use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use dash_sdk::platform::address_sync::AddressFunds;
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::PlatformP2PKHAddress;
use platform_wallet::changeset::PlatformAddressChangeSet;
use platform_wallet::changeset::PlatformAddressSyncStartState;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::wallet::{PerAccountPlatformAddressState, PerWalletPlatformAddressState};

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::accounts;
use crate::sqlite::schema::blob;
use crate::sqlite::util::safe_cast;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &PlatformAddressChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.addresses.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO platform_addresses \
                (wallet_id, account_index, address_index, address, balance, nonce) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(wallet_id, address) DO UPDATE SET \
                account_index = excluded.account_index, \
                address_index = excluded.address_index, \
                balance = excluded.balance, \
                nonce = excluded.nonce",
        )?;
        for entry in &cs.addresses {
            // Reject an entry naming a different wallet; the typed error
            // pinpoints the mismatch instead of a raw FOREIGN KEY failure.
            if entry.wallet_id != *wallet_id {
                return Err(WalletStorageError::WalletIdMismatch {
                    expected: *wallet_id,
                    found: entry.wallet_id,
                });
            }
            stmt.execute(params![
                wallet_id.as_slice(),
                i64::from(entry.account_index),
                i64::from(entry.address_index),
                entry.address.as_bytes(),
                safe_cast::u64_to_i64("platform_addresses.balance", entry.funds.balance)?,
                i64::from(entry.funds.nonce),
            ])?;
        }
    }
    if cs.sync_height.is_some()
        || cs.sync_timestamp.is_some()
        || cs.last_known_recent_block.is_some()
    {
        let current: Option<(i64, i64, i64)> = tx
            .query_row(
                "SELECT sync_height, sync_timestamp, last_known_recent_block \
                 FROM platform_address_sync WHERE wallet_id = ?1",
                params![wallet_id.as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let (cur_h, cur_t, cur_r) = match current {
            Some((h, t, r)) => (
                safe_cast::i64_to_u64("platform_address_sync.sync_height", h)?,
                safe_cast::i64_to_u64("platform_address_sync.sync_timestamp", t)?,
                safe_cast::i64_to_u64("platform_address_sync.last_known_recent_block", r)?,
            ),
            None => (0u64, 0u64, 0u64),
        };
        let h = cs.sync_height.map(|x| x.max(cur_h)).unwrap_or(cur_h);
        let t = cs.sync_timestamp.map(|x| x.max(cur_t)).unwrap_or(cur_t);
        let r = cs
            .last_known_recent_block
            .map(|x| x.max(cur_r))
            .unwrap_or(cur_r);
        tx.execute(
            "INSERT INTO platform_address_sync \
                (wallet_id, sync_height, sync_timestamp, last_known_recent_block) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id) DO UPDATE SET \
                sync_height = excluded.sync_height, \
                sync_timestamp = excluded.sync_timestamp, \
                last_known_recent_block = excluded.last_known_recent_block",
            params![
                wallet_id.as_slice(),
                safe_cast::u64_to_i64("platform_address_sync.sync_height", h)?,
                safe_cast::u64_to_i64("platform_address_sync.sync_timestamp", t)?,
                safe_cast::u64_to_i64("platform_address_sync.last_known_recent_block", r)?,
            ],
        )?;
    }
    Ok(())
}

/// Row from `platform_addresses` keyed by wallet for tests/load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformAddressRow {
    pub account_index: u32,
    pub address_index: u32,
    pub address: PlatformP2PKHAddress,
    pub funds: AddressFunds,
}

#[cfg(any(test, feature = "__test-helpers"))]
pub fn list_per_wallet(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Vec<PlatformAddressRow>, WalletStorageError> {
    // length(address) is read first (O(1)) so an oversize or wrong-width
    // address blob is caught before materializing the Vec.
    let mut stmt = conn.prepare(
        "SELECT account_index, address_index, length(address), address, balance, nonce \
         FROM platform_addresses WHERE wallet_id = ?1 \
         ORDER BY account_index, address_index, address",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let account_index: i64 = row.get(0)?;
        let address_index: i64 = row.get(1)?;
        blob::check_fixed_width(
            row.get::<_, i64>(2)?,
            20,
            "platform_addresses.address is not 20 bytes",
        )?;
        let address_bytes: Vec<u8> = row.get(3)?;
        let balance: i64 = row.get(4)?;
        let nonce: i64 = row.get(5)?;
        out.push(decode_address_row(
            account_index,
            address_index,
            &address_bytes,
            balance,
            nonce,
        )?);
    }
    Ok(out)
}

/// Reassemble the per-account committed state from `platform_payment`
/// registrations (one [`PerAccountPlatformAddressState`] each) plus the
/// `platform_addresses` rows whose `account_index` matches. Address rows for an
/// unregistered account are skipped — without the xpub there's nothing to
/// restore.
fn build_per_account(
    registrations: &[accounts::PlatformPaymentRegistration],
    address_rows: &[PlatformAddressRow],
) -> PerWalletPlatformAddressState {
    let mut per_account = PerWalletPlatformAddressState::new();
    for (account_index, xpub) in registrations {
        per_account.insert(
            *account_index,
            account_state_from_rows(*xpub, *account_index, address_rows),
        );
    }
    per_account
}

/// Build one account's state from its xpub and the address rows that
/// belong to it.
fn account_state_from_rows(
    xpub: ExtendedPubKey,
    account_index: u32,
    address_rows: &[PlatformAddressRow],
) -> PerAccountPlatformAddressState {
    let mut state = PerAccountPlatformAddressState::from_persisted(
        xpub,
        Default::default(),
        Default::default(),
    );
    for row in address_rows
        .iter()
        .filter(|r| r.account_index == account_index)
    {
        state.insert_persisted_entry(row.address_index, row.address, row.funds);
    }
    state
}

/// Build `PlatformAddressSyncStartState` for one wallet: the sync watermark
/// plus the per-account state from registrations + address rows. `load()` uses
/// the grouped [`load_all`] path; this per-wallet form is for tests.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<PlatformAddressSyncStartState, WalletStorageError> {
    let row: Option<(i64, i64, i64)> = conn
        .query_row(
            "SELECT sync_height, sync_timestamp, last_known_recent_block \
             FROM platform_address_sync WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (h, t, r) = match row {
        Some((h, t, r)) => (
            safe_cast::i64_to_u64("platform_address_sync.sync_height", h)?,
            safe_cast::i64_to_u64("platform_address_sync.sync_timestamp", t)?,
            safe_cast::i64_to_u64("platform_address_sync.last_known_recent_block", r)?,
        ),
        None => (0u64, 0u64, 0u64),
    };
    let registrations = accounts::list_platform_payment_registrations(conn, wallet_id)?;
    let address_rows = list_per_wallet(conn, wallet_id)?;
    Ok(PlatformAddressSyncStartState {
        per_account: build_per_account(&registrations, &address_rows),
        sync_height: h,
        sync_timestamp: t,
        last_known_recent_block: r,
    })
}

/// Total `platform_addresses` row count per wallet — used by tests
/// that want a stable lower-bound check without re-deriving the
/// address.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn count_per_wallet(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<usize, WalletStorageError> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM platform_addresses WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
        |row| row.get(0),
    )?;
    Ok(usize::try_from(n).unwrap_or(usize::MAX))
}

/// One [`load_all`] row per wallet: `(sync_state,
/// reconstructed_address_count)`. The count includes only address rows
/// actually rebuilt into `sync_state.per_account` (those with a matching
/// registration), so it never claims state `load()` did not surface.
pub type LoadAllEntry = (PlatformAddressSyncStartState, usize);

/// Bulk reader for `load()`: three grouped scans (over `platform_address_sync`,
/// `platform_addresses`, and the `platform_payment` `account_registrations`)
/// regardless of wallet count, instead of a per-wallet fan-out.
///
/// Driven by [`wallets::list_ids`](crate::sqlite::schema::wallets::list_ids), so
/// rows whose `wallet_id` is absent from `wallets` are not surfaced (native FKs
/// prevent such orphans anyway).
pub fn load_all(conn: &Connection) -> Result<BTreeMap<WalletId, LoadAllEntry>, WalletStorageError> {
    let sync_by_wallet = all_sync_state(conn)?;
    let addresses_by_wallet = all_address_rows(conn)?;
    let registrations_by_wallet = accounts::all_platform_payment_registrations(conn)?;

    let empty_rows: Vec<PlatformAddressRow> = Vec::new();
    let empty_regs: Vec<accounts::PlatformPaymentRegistration> = Vec::new();

    let mut out: BTreeMap<WalletId, LoadAllEntry> = BTreeMap::new();
    for wallet_id in crate::sqlite::schema::wallets::list_ids(conn)? {
        let (h, t, r) = sync_by_wallet.get(&wallet_id).copied().unwrap_or((0, 0, 0));
        let address_rows = addresses_by_wallet.get(&wallet_id).unwrap_or(&empty_rows);
        let registrations = registrations_by_wallet
            .get(&wallet_id)
            .unwrap_or(&empty_regs);
        let sync = PlatformAddressSyncStartState {
            per_account: build_per_account(registrations, address_rows),
            sync_height: h,
            sync_timestamp: t,
            last_known_recent_block: r,
        };
        let reconstructed = reconstructed_address_count(registrations, address_rows);
        out.insert(wallet_id, (sync, reconstructed));
    }
    Ok(out)
}

/// Count the `platform_addresses` rows [`build_per_account`] rebuilds: those
/// whose `account_index` has a matching registration, keeping the count aligned
/// with `per_account`.
fn reconstructed_address_count(
    registrations: &[accounts::PlatformPaymentRegistration],
    address_rows: &[PlatformAddressRow],
) -> usize {
    let registered: std::collections::BTreeSet<u32> =
        registrations.iter().map(|(idx, _)| *idx).collect();
    address_rows
        .iter()
        .filter(|r| registered.contains(&r.account_index))
        .count()
}

/// One grouped scan of `platform_address_sync` → `(sync_height,
/// sync_timestamp, last_known_recent_block)` per wallet.
fn all_sync_state(
    conn: &Connection,
) -> Result<BTreeMap<WalletId, (u64, u64, u64)>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT wallet_id, sync_height, sync_timestamp, last_known_recent_block \
         FROM platform_address_sync",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Vec<u8>>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;
    let mut out: BTreeMap<WalletId, (u64, u64, u64)> = BTreeMap::new();
    for r in rows {
        let (wid_bytes, h, t, recent) = r?;
        let wallet_id = wallet_id_from_bytes(&wid_bytes)?;
        out.insert(
            wallet_id,
            (
                safe_cast::i64_to_u64("platform_address_sync.sync_height", h)?,
                safe_cast::i64_to_u64("platform_address_sync.sync_timestamp", t)?,
                safe_cast::i64_to_u64("platform_address_sync.last_known_recent_block", recent)?,
            ),
        );
    }
    Ok(out)
}

/// One grouped scan of `platform_addresses` → the decoded rows per
/// wallet, ordered for stable per-account grouping.
fn all_address_rows(
    conn: &Connection,
) -> Result<BTreeMap<WalletId, Vec<PlatformAddressRow>>, WalletStorageError> {
    // length(address) is read first (O(1)) so an oversize or wrong-width
    // address blob is caught before materializing the Vec.
    let mut stmt = conn.prepare(
        "SELECT wallet_id, account_index, address_index, length(address), address, balance, nonce \
         FROM platform_addresses ORDER BY wallet_id, account_index, address_index, address",
    )?;
    let mut rows = stmt.query([])?;
    let mut out: BTreeMap<WalletId, Vec<PlatformAddressRow>> = BTreeMap::new();
    while let Some(row) = rows.next()? {
        let wid_bytes: Vec<u8> = row.get(0)?;
        let account_index: i64 = row.get(1)?;
        let address_index: i64 = row.get(2)?;
        blob::check_fixed_width(
            row.get::<_, i64>(3)?,
            20,
            "platform_addresses.address is not 20 bytes",
        )?;
        let address_bytes: Vec<u8> = row.get(4)?;
        let balance: i64 = row.get(5)?;
        let nonce: i64 = row.get(6)?;
        let wallet_id = wallet_id_from_bytes(&wid_bytes)?;
        out.entry(wallet_id).or_default().push(decode_address_row(
            account_index,
            address_index,
            &address_bytes,
            balance,
            nonce,
        )?);
    }
    Ok(out)
}

/// Decode one `platform_addresses` row into a [`PlatformAddressRow`].
fn decode_address_row(
    account_index: i64,
    address_index: i64,
    address_bytes: &[u8],
    balance: i64,
    nonce: i64,
) -> Result<PlatformAddressRow, WalletStorageError> {
    if address_bytes.len() != 20 {
        return Err(WalletStorageError::blob_decode(
            "platform_addresses.address column is not 20 bytes",
        ));
    }
    let mut hash160 = [0u8; 20];
    hash160.copy_from_slice(address_bytes);
    let balance = safe_cast::i64_to_u64("platform_addresses.balance", balance)?;
    let nonce = safe_cast::i64_to_u32("platform_addresses.nonce", nonce)?;
    let account_index = safe_cast::i64_to_u32("platform_addresses.account_index", account_index)?;
    let address_index = safe_cast::i64_to_u32("platform_addresses.address_index", address_index)?;
    Ok(PlatformAddressRow {
        account_index,
        address_index,
        address: PlatformP2PKHAddress::new(hash160),
        funds: AddressFunds { balance, nonce },
    })
}

fn wallet_id_from_bytes(bytes: &[u8]) -> Result<WalletId, WalletStorageError> {
    <[u8; 32]>::try_from(bytes).map_err(|_| WalletStorageError::InvalidWalletIdLength {
        actual: bytes.len(),
    })
}
