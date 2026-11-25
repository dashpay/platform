//! Restore persisted address rows into a key-wallet [`AddressPool`].
//!
//! Hosts persisting the wallet through the FFI keep one row per derived
//! address and hand those rows back at load; this module owns how they land in
//! the pool, so the FFI layer only marshals them.

use std::collections::BTreeSet;

use key_wallet::bip32::ChildNumber;
use key_wallet::error::Error as KeyWalletError;
use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
use key_wallet::AddressInfo;

/// The most missing addresses one [`restore_address_rows`] call derives to
/// close gaps below persisted rows, so a damaged store cannot make load derive
/// without bound.
#[cfg(not(test))]
pub(crate) const MAX_RESTORE_GAP_FILL: u32 = 10_000;
/// The same bound, smaller in unit tests so crossing it derives little.
#[cfg(test)]
pub(crate) const MAX_RESTORE_GAP_FILL: u32 = 50;

/// Why [`restore_address_rows`] skipped a row; logged, not returned.
#[derive(Debug, thiserror::Error)]
enum RowRefusal {
    #[error(
        "the {gap} addresses missing below it exceed the {budget} the restore may still derive"
    )]
    GapTooLarge { gap: u32, budget: u32 },
    #[error("the addresses missing below it could not be derived: {0}")]
    GapUnderivable(#[source] KeyWalletError),
    #[error("the address pool refused it: {0}")]
    Refused(#[source] KeyWalletError),
}

/// How [`restore_address_rows`] leaves the pool's addresses at or below its
/// highest persisted row that no persisted row restored: pre-derived ones and
/// those it derives to close a gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GapRows {
    /// Free to hand out, for pools whose usage the chain shows.
    Available,
    /// Marked used, for pools whose usage the chain cannot show: a lost row
    /// may hold a key already handed out.
    Used,
}

/// Outcome of [`restore_address_rows`]: how many persisted rows landed in the
/// pool and how many it refused.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct AddressRowsRestore {
    pub(crate) restored: usize,
    pub(crate) refused: usize,
}

/// Derive every address of `pool` up to and including `index` that it has not
/// derived yet. A no-op when `index` is already derived.
///
/// # Errors
/// Fails when `key_source` cannot derive the pool's addresses, or `index` is
/// not a normal (non-hardened) child index.
pub fn derive_through(
    pool: &mut AddressPool,
    key_source: &KeySource,
    index: u32,
) -> Result<(), KeyWalletError> {
    ChildNumber::from_normal_idx(index).map_err(KeyWalletError::Bip32)?;
    let next = pool.highest_generated().map_or(0, |highest| highest + 1);
    pool.generate_addresses(index.saturating_add(1).saturating_sub(next), key_source)?;
    Ok(())
}

/// Store persisted `rows` in `pool`, in index order. A row overwrites the
/// entry the pool pre-derived at its index.
///
/// A row whose public key is missing (written before rows carried one) keeps
/// the typed key the pool already holds at that index, so pre-derived BLS
/// operator keys survive a legacy store.
///
/// Rows missing below a row's index (a lost persistence round) are derived
/// from `key_source` first, at most [`MAX_RESTORE_GAP_FILL`] per call, so the
/// row is watched and its index is never handed out again. Every address at
/// or below the highest row that no row restored, derived here or before, is
/// left as `gap_rows` says. Hardened pools, which cannot derive, take rows at
/// any index.
///
/// A row is refused, counted and skipped when the gap below it cannot be
/// derived or exceeds the bound, its address is already stored at another
/// index, or its index is not a normal child index.
pub(crate) fn restore_address_rows(
    pool: &mut AddressPool,
    key_source: &KeySource,
    mut rows: Vec<AddressInfo>,
    gap_rows: GapRows,
) -> AddressRowsRestore {
    rows.sort_by_key(|row| row.index);
    let highest_row = rows.last().map(|row| row.index);
    let mut restored = BTreeSet::new();
    let mut outcome = AddressRowsRestore::default();
    let mut first_refusal: Option<(u32, RowRefusal)> = None;
    let mut budget = MAX_RESTORE_GAP_FILL;
    for mut row in rows {
        let derived_from = pool.highest_generated().map_or(0, |highest| highest + 1);
        let index = row.index;
        if let Err(error) = fill_gap_below(pool, key_source, index, &mut budget) {
            outcome.refused += 1;
            // A gap fill that failed part way keeps the addresses it derived.
            if gap_rows == GapRows::Used {
                mark_used_from(pool, derived_from);
            }
            first_refusal.get_or_insert((index, error));
            continue;
        }
        if row.public_key.is_none() {
            if let Some(existing) = pool.addresses().get(&index) {
                row.public_key = existing.public_key.clone();
            }
        }
        match pool.insert(row) {
            Ok(()) => {
                outcome.restored += 1;
                restored.insert(index);
            }
            Err(error) => {
                outcome.refused += 1;
                first_refusal.get_or_insert((index, RowRefusal::Refused(error)));
            }
        }
    }
    if let (GapRows::Used, Some(highest_row)) = (gap_rows, highest_row) {
        let missing: Vec<_> = pool
            .addresses()
            .range(..=highest_row)
            .filter(|(index, _)| !restored.contains(*index))
            .map(|(_, info)| info.address.clone())
            .collect();
        for address in missing {
            pool.mark_used(&address);
        }
    }
    if let Some((address_index, error)) = first_refusal {
        tracing::warn!(
            pool_type = ?pool.pool_type(),
            refused = outcome.refused,
            first_refused_index = address_index,
            %error,
            "load: address pool refused persisted address rows; those addresses are not \
             watched until a rescan finds them"
        );
    }
    outcome
}

fn fill_gap_below(
    pool: &mut AddressPool,
    key_source: &KeySource,
    index: u32,
    budget: &mut u32,
) -> Result<(), RowRefusal> {
    let next = pool.highest_generated().map_or(0, |highest| highest + 1);
    if pool.pool_type() == AddressPoolType::AbsentHardened || index <= next {
        return Ok(());
    }
    let gap = index - next;
    if gap > *budget {
        return Err(RowRefusal::GapTooLarge {
            gap,
            budget: *budget,
        });
    }
    derive_through(pool, key_source, index - 1).map_err(RowRefusal::GapUnderivable)?;
    *budget -= gap;
    Ok(())
}

fn mark_used_from(pool: &mut AddressPool, index: u32) {
    let addresses: Vec<_> = pool
        .addresses()
        .range(index..)
        .map(|(_, info)| info.address.clone())
        .collect();
    for address in addresses {
        pool.mark_used(&address);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::bip32::{ChildNumber, DerivationPath};
    use key_wallet::managed_account::address_pool::{AddressState, PublicKeyType};
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::ManagedWalletInfo;
    use key_wallet::{Network, Wallet};

    const BIP44_0: AccountType = AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    };

    /// The BIP44 account 0 external pool `ManagedWalletInfo::from_wallet`
    /// pre-derives, with the key source that derives it.
    fn external_pool() -> (AddressPool, KeySource) {
        let wallet = Wallet::from_seed_bytes(
            [7u8; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .expect("wallet from seed");
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
        let pool = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .expect("BIP44 account 0")
            .managed_account_type_mut()
            .address_pools_mut()
            .into_iter()
            .find(|pool| pool.pool_type() == AddressPoolType::External)
            .expect("external pool")
            .clone();
        (pool, wallet.key_source_for_account(BIP44_0))
    }

    /// The row a host persisted for `index` of `pool`, marked used.
    fn used_row(pool: &AddressPool, key_source: &KeySource, index: u32) -> AddressInfo {
        let mut probe = pool.clone();
        derive_through(&mut probe, key_source, index).expect("derive the row's address");
        let mut row = probe.addresses()[&index].clone();
        row.state = AddressState::Used;
        row
    }

    fn next_index(pool: &AddressPool) -> u32 {
        pool.highest_generated().map_or(0, |highest| highest + 1)
    }

    #[test]
    fn should_reject_a_hardened_target_before_deriving_addresses() {
        let (mut pool, key_source) = external_pool();
        let generated = pool.highest_generated();
        let addresses = pool.addresses().len();

        for key_source in [KeySource::NoKeySource, key_source] {
            for index in [1 << 31, u32::MAX] {
                let error = derive_through(&mut pool, &key_source, index)
                    .expect_err("hardened targets cannot be derived from an xpub");

                assert!(matches!(error, KeyWalletError::Bip32(_)));
                assert_eq!(pool.highest_generated(), generated);
                assert_eq!(pool.addresses().len(), addresses);
            }
        }
    }

    #[test]
    fn should_restore_a_row_past_the_derived_window_by_deriving_the_gap() {
        let (mut pool, key_source) = external_pool();
        let index = next_index(&pool) + 5;
        let row = used_row(&pool, &key_source, index);

        let outcome = restore_address_rows(
            &mut pool,
            &key_source,
            vec![row.clone()],
            GapRows::Available,
        );

        assert_eq!(
            outcome,
            AddressRowsRestore {
                restored: 1,
                refused: 0
            }
        );
        let restored = &pool.addresses()[&index];
        assert_eq!(restored.address, row.address);
        assert!(restored.is_used());
        assert_eq!(pool.highest_used(), Some(index));
        assert!((0..index).all(|i| pool.addresses().contains_key(&i)));
    }

    #[test]
    fn should_refuse_a_gap_row_the_pool_cannot_derive() {
        let (mut pool, key_source) = external_pool();
        let index = next_index(&pool) + 5;
        let row = used_row(&pool, &key_source, index);
        let generated = pool.highest_generated();

        let outcome = restore_address_rows(
            &mut pool,
            &KeySource::NoKeySource,
            vec![row],
            GapRows::Available,
        );

        assert_eq!(
            outcome,
            AddressRowsRestore {
                restored: 0,
                refused: 1
            }
        );
        assert_eq!(pool.highest_generated(), generated);
        assert_eq!(pool.highest_used(), None);
    }

    #[test]
    fn should_refuse_a_row_whose_gap_exceeds_the_fill_bound() {
        let (mut pool, key_source) = external_pool();
        let generated = pool.highest_generated();
        let mut row = pool.addresses()[&0].clone();
        row.index = next_index(&pool) + MAX_RESTORE_GAP_FILL + 1;

        let outcome = restore_address_rows(&mut pool, &key_source, vec![row], GapRows::Available);

        assert_eq!(
            outcome,
            AddressRowsRestore {
                restored: 0,
                refused: 1
            }
        );
        assert_eq!(pool.highest_generated(), generated);
    }

    #[test]
    fn should_refuse_gap_rows_once_the_pools_fill_budget_is_spent() {
        let (mut pool, key_source) = external_pool();
        let first = next_index(&pool) + MAX_RESTORE_GAP_FILL;
        let rows = vec![
            used_row(&pool, &key_source, first),
            used_row(&pool, &key_source, first + 2),
        ];

        let outcome = restore_address_rows(&mut pool, &key_source, rows, GapRows::Available);

        assert_eq!(
            outcome,
            AddressRowsRestore {
                restored: 1,
                refused: 1
            }
        );
        assert_eq!(pool.highest_used(), Some(first));
    }

    #[test]
    fn should_restore_rows_given_out_of_index_order() {
        let (mut pool, key_source) = external_pool();
        let first = next_index(&pool);
        let rows = vec![
            used_row(&pool, &key_source, first + 1),
            used_row(&pool, &key_source, first),
        ];

        let outcome =
            restore_address_rows(&mut pool, &KeySource::NoKeySource, rows, GapRows::Available);

        assert_eq!(
            outcome,
            AddressRowsRestore {
                restored: 2,
                refused: 0
            }
        );
        assert_eq!(pool.highest_used(), Some(first + 1));
    }

    /// A row persisted before rows carried a key keeps the typed key the pool
    /// pre-derived at its index; at an index with nothing pre-derived it stays
    /// key-less rather than inventing one.
    #[test]
    fn should_keep_the_pre_derived_key_for_a_keyless_legacy_row() {
        let base: DerivationPath = vec![
            ChildNumber::from_hardened_idx(9).expect("hardened 9"),
            ChildNumber::from_hardened_idx(1).expect("hardened 1"),
            ChildNumber::from_hardened_idx(3).expect("hardened 3"),
        ]
        .into();
        let mut pool = AddressPool::new_without_generation(
            base,
            AddressPoolType::AbsentHardened,
            5,
            Network::Testnet,
        );
        let (external, key_source) = external_pool();
        let entry = |index: u32, key: Option<PublicKeyType>| {
            let mut info = external.addresses()[&index].clone();
            info.public_key = key;
            info
        };
        let bls = PublicKeyType::BLS(vec![0xE7; 48]);
        pool.insert(entry(7, Some(bls.clone())))
            .expect("store the pre-derived entry");

        let outcome = restore_address_rows(
            &mut pool,
            &key_source,
            vec![entry(7, None), entry(9, None)],
            GapRows::Available,
        );

        assert_eq!(outcome.refused, 0);
        assert_eq!(pool.addresses()[&7].public_key, Some(bls));
        assert_eq!(pool.addresses()[&9].public_key, None);
    }

    /// A gap fill stops at an address a corrupt row already holds at another
    /// index, after deriving the addresses before it. In a pool whose usage
    /// the chain cannot show those may hold keys already handed out.
    #[test]
    fn should_mark_used_the_addresses_a_failed_gap_fill_derived() {
        let (mut pool, key_source) = external_pool();
        let next = next_index(&pool);
        let mut corrupt = used_row(&pool, &key_source, next + 3);
        corrupt.index = next + 1;
        let rows = vec![corrupt, used_row(&pool, &key_source, next + 5)];

        let outcome = restore_address_rows(&mut pool, &key_source, rows, GapRows::Used);

        assert_eq!(
            outcome,
            AddressRowsRestore {
                restored: 1,
                refused: 1
            }
        );
        assert_eq!(pool.highest_generated(), Some(next + 2));
        assert!(pool.addresses()[&(next + 2)].is_used());
    }
}
