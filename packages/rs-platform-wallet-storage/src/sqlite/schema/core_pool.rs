//! Writer + account-attribution helper for the `core_address_pool` table.
//!
//! Per-index address-pool rows carrying a `used` flag, scoped by
//! `(wallet_id, account_index, key_class, pool_type, address_index)`. The
//! first-class row store the reader consumes verbatim — no `core_utxos`
//! script-derivation, no horizon-walk re-derivation. Populated from the
//! `account_address_pools` changeset snapshots; the UTXO writer reads it
//! back to attribute an outpoint to its owning account.

use rusqlite::{params, OptionalExtension, Transaction};

use platform_wallet::changeset::AccountAddressPoolEntry;
use platform_wallet::wallet::platform_wallet::WalletId;

use key_wallet::managed_account::address_pool::AddressPoolType;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::accounts;

/// Stored `pool_type` discriminant. Kept in the primary key so an External
/// and an Internal pool never collide at the same `address_index`.
pub(crate) fn pool_type_to_i64(pool_type: AddressPoolType) -> i64 {
    match pool_type {
        AddressPoolType::External => 0,
        AddressPoolType::Internal => 1,
        AddressPoolType::Absent => 2,
        AddressPoolType::AbsentHardened => 3,
    }
}

const UPSERT_POOL_SQL: &str = "INSERT INTO core_address_pool \
        (wallet_id, account_index, key_class, pool_type, address_index, script, used) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
     ON CONFLICT(wallet_id, account_index, key_class, pool_type, address_index) DO UPDATE SET \
        script = excluded.script, \
        used = MAX(used, excluded.used)";

/// Expand `account_address_pools` snapshots into per-index
/// `core_address_pool` rows. Idempotent: `script` is derivation-stable and
/// `used` is monotonic (`MAX`), so re-applying the same snapshot is a no-op
/// and a used address can never revert to unused (the reuse-guard invariant).
pub fn apply_pools(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    pools: &[AccountAddressPoolEntry],
) -> Result<(), WalletStorageError> {
    if pools.is_empty() {
        return Ok(());
    }
    let mut stmt = tx.prepare_cached(UPSERT_POOL_SQL)?;
    for entry in pools {
        let account_index = i64::from(accounts::account_index(&entry.account_type));
        // TODO(key_class): PlatformPayment carries a real key_class; every
        // other account maps to the 0 sentinel until the pool snapshot
        // threads a per-pool key class.
        let key_class = i64::from(accounts::account_key_class(&entry.account_type));
        let pool_type = pool_type_to_i64(entry.pool_type);
        for info in &entry.addresses {
            stmt.execute(params![
                wallet_id.as_slice(),
                account_index,
                key_class,
                pool_type,
                i64::from(info.index),
                info.script_pubkey.as_bytes(),
                info.used,
            ])?;
        }
    }
    Ok(())
}

/// Owning account index for a UTXO, matched by its `script_pubkey` against a
/// pool row. `None` when no pool row covers the script — the UTXO writer
/// then falls back to account 0 (the one-way historical-attribution default,
/// R7): funds are never dropped, only conservatively bucketed.
pub fn account_index_for_script(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    script: &[u8],
) -> Result<Option<u32>, WalletStorageError> {
    let idx: Option<i64> = tx
        .prepare_cached(
            "SELECT account_index FROM core_address_pool \
             WHERE wallet_id = ?1 AND script = ?2 LIMIT 1",
        )?
        .query_row(params![wallet_id.as_slice(), script], |row| row.get(0))
        .optional()?;
    idx.map(|v| crate::sqlite::util::safe_cast::i64_to_u32("core_address_pool.account_index", v))
        .transpose()
}

/// Used addresses for a wallet, read verbatim from `core_address_pool`
/// (`used = 1`) with no re-derivation. Possibly empty. The caller **unions**
/// this with the `core_utxos`-derived set — the reuse guard is monotonic, so
/// a mixed store (historical UTXOs a later partial pool snapshot never
/// enumerates) must surface both sources, never drop the historical ones.
///
/// `network` turns each stored `script` back into an [`Address`]; a script
/// that isn't a valid address is a hard error — corruption is never silently
/// dropped, matching [`crate::sqlite::schema::core_state::load_used_addresses`].
pub fn load_used_addresses(
    conn: &rusqlite::Connection,
    wallet_id: &WalletId,
    network: dashcore::Network,
) -> Result<Vec<dashcore::Address>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT script FROM core_address_pool \
         WHERE wallet_id = ?1 AND used = 1 ORDER BY script",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        row.get::<_, Vec<u8>>(0)
    })?;
    let mut out = Vec::new();
    for r in rows {
        let script = dashcore::ScriptBuf::from_bytes(r?);
        let address = dashcore::Address::from_script(&script, network).map_err(|_| {
            WalletStorageError::blob_decode("core_address_pool.script not an address")
        })?;
        out.push(address);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_type_discriminants_are_stable_and_distinct() {
        let all = [
            AddressPoolType::External,
            AddressPoolType::Internal,
            AddressPoolType::Absent,
            AddressPoolType::AbsentHardened,
        ];
        let mapped: Vec<i64> = all.iter().copied().map(pool_type_to_i64).collect();
        assert_eq!(mapped, vec![0, 1, 2, 3]);
    }
}
