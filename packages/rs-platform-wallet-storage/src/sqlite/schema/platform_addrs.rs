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
use crate::sqlite::util::safe_cast;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &PlatformAddressChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.addresses.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO platform_addresses \
                (wallet_id, account_index, address_index, address, balance, nonce, \
                 as_of_height) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(wallet_id, address) DO UPDATE SET \
                account_index = excluded.account_index, \
                address_index = excluded.address_index, \
                balance = excluded.balance, \
                nonce = excluded.nonce, \
                as_of_height = excluded.as_of_height",
        )?;
        for entry in &cs.addresses {
            // The row is keyed by the outer `wallet_id`; an entry that
            // names a different wallet would otherwise be mis-filed. The
            // native FK also rejects an unknown parent, but this typed
            // error pinpoints the mismatch instead of surfacing a raw
            // FOREIGN KEY failure.
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
                safe_cast::u64_to_i64("platform_addresses.as_of_height", entry.funds.as_of_height)?,
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
    let mut stmt = conn.prepare(
        "SELECT account_index, address_index, address, balance, nonce, as_of_height \
         FROM platform_addresses WHERE wallet_id = ?1 \
         ORDER BY account_index, address_index, address",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Vec<u8>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;
    let mut out = Vec::new();
    for r in rows {
        let (account_index, address_index, address_bytes, balance, nonce, as_of_height) = r?;
        out.push(decode_address_row(
            account_index,
            address_index,
            &address_bytes,
            balance,
            nonce,
            as_of_height,
        )?);
    }
    Ok(out)
}

/// Reassemble the per-account committed state for one wallet from its
/// `platform_payment` registrations (xpub per account index) and its
/// `platform_addresses` rows (the derived address set + known balances).
///
/// Each registration becomes one [`PerAccountPlatformAddressState`]; the
/// address rows whose `account_index` matches populate its `addresses`
/// bijection and `found` balance map via
/// [`PerAccountPlatformAddressState::insert_persisted_entry`]. Address
/// rows for an account with no registration are skipped — without the
/// xpub the provider can't extend that account's gap window, so there's
/// nothing to restore.
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

/// Build `PlatformAddressSyncStartState` for one wallet: the
/// network-scoped sync watermark plus the per-account committed state
/// reconstructed from registrations + address rows.
///
/// `load()` uses the grouped [`load_all`] path; this per-wallet form is
/// retained for this crate's integration tests.
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

/// One row of [`load_all`] aggregated state per wallet:
/// `(sync_state, address_row_count)`.
///
/// `address_row_count` is the number of `platform_addresses` rows for the
/// wallet — `load()` uses it (with the watermark and per_account) to
/// decide whether the wallet carries any platform state worth surfacing.
pub type LoadAllEntry = (PlatformAddressSyncStartState, usize);

/// Bulk reader for `load()`. Cost is a fixed number of grouped scans —
/// one over `platform_address_sync`, one over `platform_addresses`, and
/// one over the `platform_payment` `account_registrations` — regardless
/// of wallet count, rather than a per-wallet fan-out.
///
/// Driven by [`wallet_meta::list_ids`](crate::sqlite::schema::wallet_meta::list_ids):
/// orphaned `platform_addresses` / `platform_address_sync` rows whose
/// `wallet_id` is absent from `wallet_metadata` are intentionally NOT
/// surfaced. Native foreign keys prevent such orphans; a future re-wire
/// that needs them must restore the id-union over the area tables.
pub fn load_all(conn: &Connection) -> Result<BTreeMap<WalletId, LoadAllEntry>, WalletStorageError> {
    let sync_by_wallet = all_sync_state(conn)?;
    let addresses_by_wallet = all_address_rows(conn)?;
    let registrations_by_wallet = accounts::all_platform_payment_registrations(conn)?;

    let empty_rows: Vec<PlatformAddressRow> = Vec::new();
    let empty_regs: Vec<accounts::PlatformPaymentRegistration> = Vec::new();

    let mut out: BTreeMap<WalletId, LoadAllEntry> = BTreeMap::new();
    for wallet_id in crate::sqlite::schema::wallet_meta::list_ids(conn)? {
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
        out.insert(wallet_id, (sync, address_rows.len()));
    }
    Ok(out)
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
    let mut stmt = conn.prepare(
        "SELECT wallet_id, account_index, address_index, address, balance, nonce, \
                as_of_height \
         FROM platform_addresses ORDER BY wallet_id, account_index, address_index, address",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Vec<u8>>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, Vec<u8>>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
        ))
    })?;
    let mut out: BTreeMap<WalletId, Vec<PlatformAddressRow>> = BTreeMap::new();
    for r in rows {
        let (wid_bytes, account_index, address_index, address_bytes, balance, nonce, as_of_height) =
            r?;
        let wallet_id = wallet_id_from_bytes(&wid_bytes)?;
        out.entry(wallet_id).or_default().push(decode_address_row(
            account_index,
            address_index,
            &address_bytes,
            balance,
            nonce,
            as_of_height,
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
    as_of_height: i64,
) -> Result<PlatformAddressRow, WalletStorageError> {
    if address_bytes.len() != 20 {
        return Err(WalletStorageError::blob_decode(
            "platform_addresses.address column is not 20 bytes",
        ));
    }
    let mut hash160 = [0u8; 20];
    hash160.copy_from_slice(address_bytes);
    let balance = safe_cast::i64_to_u64("platform_addresses.balance", balance)?;
    let as_of_height = safe_cast::i64_to_u64("platform_addresses.as_of_height", as_of_height)?;
    let nonce = u32::try_from(nonce).map_err(|_| WalletStorageError::IntegerOverflow {
        field: "platform_addresses.nonce",
        value: nonce as u64,
        target: safe_cast::SafeCastTarget::U64,
    })?;
    let account_index =
        u32::try_from(account_index).map_err(|_| WalletStorageError::IntegerOverflow {
            field: "platform_addresses.account_index",
            value: account_index as u64,
            target: safe_cast::SafeCastTarget::U64,
        })?;
    let address_index =
        u32::try_from(address_index).map_err(|_| WalletStorageError::IntegerOverflow {
            field: "platform_addresses.address_index",
            value: address_index as u64,
            target: safe_cast::SafeCastTarget::U64,
        })?;
    Ok(PlatformAddressRow {
        account_index,
        address_index,
        address: PlatformP2PKHAddress::new(hash160),
        funds: AddressFunds {
            balance,
            nonce,
            as_of_height,
        },
    })
}

fn wallet_id_from_bytes(bytes: &[u8]) -> Result<WalletId, WalletStorageError> {
    <[u8; 32]>::try_from(bytes).map_err(|_| WalletStorageError::InvalidWalletIdLength {
        actual: bytes.len(),
    })
}
