//! Typed conversion of the published V001-V007 state. These legacy table
//! names belong here only: V008 retains them until V011 has all destination
//! columns, and the enclosing migration transaction drops them after success.

use dashcore::Address;
use key_wallet::account::derivation::AccountDerivation;
use key_wallet::account::{Account, AccountType};
use key_wallet::managed_account::address_pool::{AddressPoolType, AddressState};
use key_wallet::AddressInfo;
use platform_wallet::changeset::{AccountAddressPoolEntry, AccountRegistrationEntry};
use rusqlite::{params, Transaction};

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::{accounts, blob, core_pool, id32, wallets};

// Legacy v8 rows only: read here once during migration, never written again,
// so the shape is admitted to `blob::decode` without becoming persistable.
blob::impl_blob_decode!(AccountAddressPoolEntry);

fn invalid(reason: &'static str) -> WalletStorageError {
    WalletStorageError::blob_decode(reason)
}

fn matches_account(label: &str, index: i64, account_type: &AccountType) -> bool {
    accounts::db_label_matches_entry(label, account_type)
        && index == i64::from(accounts::account_index(account_type))
}

fn pool_type(label: &str) -> Result<AddressPoolType, WalletStorageError> {
    match label {
        "external" => Ok(AddressPoolType::External),
        "internal" => Ok(AddressPoolType::Internal),
        "absent" => Ok(AddressPoolType::Absent),
        "absent_hardened" => Ok(AddressPoolType::AbsentHardened),
        _ => Err(invalid("legacy pool type is invalid")),
    }
}

pub(super) fn backfill_registrations(tx: &Transaction<'_>) -> Result<(), WalletStorageError> {
    let mut stmt = tx.prepare(
        "SELECT wallet_id, account_type, account_index, length(account_xpub_bytes), account_xpub_bytes
         FROM account_registrations",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let wallet_id: Vec<u8> = row.get(0)?;
        let label: String = row.get(1)?;
        let index: i64 = row.get(2)?;
        blob::check_size(row.get(3)?)?;
        let entry: AccountRegistrationEntry = blob::decode(&row.get::<_, Vec<u8>>(4)?)?;
        if !matches_account(&label, index, &entry.account_type) {
            return Err(WalletStorageError::AccountRegistrationEntryMismatch);
        }
        let (user, friend) = accounts::account_dashpay_ids(&entry.account_type);
        tx.execute(
            "UPDATE account_registrations SET key_class = ?1, user_identity_id = ?2,
             friend_identity_id = ?3 WHERE wallet_id = ?4 AND account_type = ?5 AND account_index = ?6",
            params![accounts::account_key_class(&entry.account_type), user.as_slice(),
                friend.as_slice(), wallet_id, label, index],
        )?;
    }
    Ok(())
}

pub(super) fn convert_pools(tx: &Transaction<'_>) -> Result<(), WalletStorageError> {
    // Join through wallets, preserving V008's established orphan policy: rows
    // left behind with foreign keys disabled are unreachable and may be swept.
    let mut wallet_stmt = tx.prepare("SELECT wallet_id, network FROM wallets")?;
    let mut wallet_rows = wallet_stmt.query([])?;
    while let Some(row) = wallet_rows.next()? {
        let wallet_id = id32("wallets.wallet_id", &row.get::<_, Vec<u8>>(0)?)?;
        let network = wallets::parse_network(&row.get::<_, String>(1)?)
            .ok_or_else(|| invalid("legacy wallet network is invalid"))?;
        let mut pools = Vec::<AccountAddressPoolEntry>::new();
        let mut stmt = tx.prepare(
            "SELECT account_type, account_index, pool_type, length(snapshot_blob), snapshot_blob
             FROM account_address_pools WHERE wallet_id = ?1",
        )?;
        let mut rows = stmt.query([wallet_id.as_slice()])?;
        while let Some(row) = rows.next()? {
            let label: String = row.get(0)?;
            let index: i64 = row.get(1)?;
            let kind = pool_type(&row.get::<_, String>(2)?)?;
            blob::check_size(row.get(3)?)?;
            let entry: AccountAddressPoolEntry = blob::decode(&row.get::<_, Vec<u8>>(4)?)?;
            if !matches_account(&label, index, &entry.account_type) || kind != entry.pool_type {
                return Err(invalid("legacy pool columns disagree with snapshot"));
            }
            let mut indices = std::collections::HashSet::new();
            for info in &entry.addresses {
                if info.address.script_pubkey() != info.script_pubkey || !indices.insert(info.index)
                {
                    return Err(invalid("legacy pool has conflicting address data"));
                }
            }
            pools.push(entry);
        }

        let mut registrations = Vec::<AccountRegistrationEntry>::new();
        let mut stmt = tx.prepare(
            "SELECT length(account_xpub_bytes), account_xpub_bytes FROM account_registrations
             WHERE wallet_id = ?1",
        )?;
        let mut rows = stmt.query([wallet_id.as_slice()])?;
        while let Some(row) = rows.next()? {
            blob::check_size(row.get(0)?)?;
            registrations.push(blob::decode(&row.get::<_, Vec<u8>>(1)?)?);
        }

        let mut stmt = tx.prepare(
            "SELECT account_type, account_index, address, derivation_path, used
             FROM core_derived_addresses WHERE wallet_id = ?1",
        )?;
        let mut rows = stmt.query([wallet_id.as_slice()])?;
        while let Some(row) = rows.next()? {
            let label: String = row.get(0)?;
            let account_index: i64 = row.get(1)?;
            let address = row
                .get::<_, String>(2)?
                .parse::<Address<_>>()
                .map_err(|_| invalid("legacy derived address is invalid"))?
                .require_network(network)
                .map_err(|_| invalid("legacy derived address network disagrees with wallet"))?;
            let path: String = row.get(3)?;
            // The published writer stored `pool_type/index`, not a BIP32 path.
            let (kind, index) = path
                .split_once('/')
                .ok_or_else(|| invalid("legacy derived address path is invalid"))?;
            let kind = pool_type(kind)?;
            let index: u32 = index
                .parse()
                .map_err(|_| invalid("legacy derived address index is invalid"))?;
            let used: i64 = row.get(4)?;
            if !(0..=1).contains(&used) {
                return Err(invalid("legacy derived address used flag is invalid"));
            }

            // A matching snapshot proves ownership even for a hardened pool
            // whose address cannot be regenerated from a public account key.
            let matching = pools
                .iter()
                .enumerate()
                .filter_map(|(p, pool)| {
                    if matches_account(&label, account_index, &pool.account_type)
                        && pool.pool_type == kind
                    {
                        pool.addresses
                            .iter()
                            .position(|info| info.index == index && info.address == address)
                            .map(|i| (p, i))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if let [(p, i)] = matching.as_slice() {
                if used == 1 {
                    pools[*p].addresses[*i].state = AddressState::Used;
                }
                continue;
            }
            if !matching.is_empty() {
                return Err(invalid("legacy derived address ownership is ambiguous"));
            }

            // The old row's label/index is authoritative when it names one
            // full account identity. Requiring public derivation here would
            // reject legitimate hardened addresses. Only ambiguous aliases
            // need additional evidence from an available account xpub.
            let mut owners = registrations
                .iter()
                .map(|entry| entry.account_type)
                .chain(pools.iter().map(|entry| entry.account_type))
                .filter(|owner| matches_account(&label, account_index, owner))
                .collect::<Vec<_>>();
            let mut seen = std::collections::HashSet::new();
            owners.retain(|owner| seen.insert(*owner));
            if owners.len() > 1 {
                owners.retain(|owner| {
                    registrations
                        .iter()
                        .filter(|entry| entry.account_type == *owner)
                        .any(|entry| {
                            Account::new(
                                Some(wallet_id),
                                entry.account_type,
                                entry.account_xpub,
                                network,
                            )
                            .and_then(|account| account.derive_address_at(kind, index, None))
                            .is_ok_and(|derived| derived == address)
                        })
                });
            }
            let [owner] = owners.as_slice() else {
                return Err(invalid("legacy derived address has no unique proven owner"));
            };
            let mut info = AddressInfo::new_from_script_pubkey_p2pkh(
                address.script_pubkey(),
                index,
                Default::default(),
                network,
            )
            .map_err(|_| invalid("legacy derived address script is invalid"))?;
            if used == 1 {
                info.state = AddressState::Used;
            }
            if let Some(pool) = pools
                .iter_mut()
                .find(|pool| pool.account_type == *owner && pool.pool_type == kind)
            {
                if pool.addresses.iter().any(|info| info.index == index) {
                    return Err(invalid(
                        "legacy derived address conflicts with snapshot slot",
                    ));
                }
                pool.addresses.push(info);
            } else {
                pools.push(AccountAddressPoolEntry {
                    account_type: *owner,
                    pool_type: kind,
                    addresses: vec![info],
                });
            }
        }
        core_pool::apply_pools(tx, &wallet_id, &pools)?;
    }
    tx.execute_batch("DROP TABLE account_address_pools; DROP TABLE core_derived_addresses;")?;
    Ok(())
}
