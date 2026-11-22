//! Balance change history of the wallet's own platform payment addresses.
//!
//! Platform keeps two zones of per-address balance changes: the recent zone
//! (one entry per block, not yet compacted) and the compacted zone (ranges of
//! blocks merged into one entry per address, expired by Drive after about a
//! week). Both queries are network-wide; this module pages through them,
//! keeps only the changes that touch an address of this wallet, and orders
//! them newest first so a host can render them as they come.

use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::future::Future;

use dash_sdk::platform::query::{
    RecentAddressBalanceChangesQuery, RecentCompactedAddressBalanceChangesQuery,
};
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::{
    BlockAddressBalanceChanges, CompactedBlockAddressBalanceChanges, RecentAddressBalanceChanges,
    RecentCompactedAddressBalanceChanges,
};
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::{BlockAwareCreditOperation, CreditOperation};
use dpp::fee::Credits;
use dpp::prelude::BlockHeight;
use key_wallet::PlatformP2PKHAddress;

use super::saturating_sum_credits;
use super::wallet::{merge_platform_payment_candidate_addresses, PlatformAddressWallet};
use crate::error::PlatformWalletError;

/// How one block changed an address's balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressBalanceChange {
    /// The balance was set to this value.
    Set(Credits),
    /// This amount was added to the balance.
    Added(Credits),
}

/// One balance change of a wallet address in the recent zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressBalanceChangeEntry {
    /// Block whose execution made the change.
    pub block_height: BlockHeight,
    /// The wallet address whose balance changed.
    pub address: PlatformAddress,
    /// What the block did to the balance.
    pub change: AddressBalanceChange,
}

/// How a compacted block range changed an address's balance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompactedAddressBalanceChange {
    /// The range ended with the balance set to this value.
    SetFinal(Credits),
    /// The range only added to the balance: each addition with the block that
    /// made it, newest block first.
    Additions(Vec<(BlockHeight, Credits)>),
}

impl CompactedAddressBalanceChange {
    /// The credits a host shows for the range: the final value for
    /// [`Self::SetFinal`], the sum of the additions for [`Self::Additions`]
    /// (saturating, as for every sum over wallet balances).
    pub fn credits(&self) -> Credits {
        match self {
            Self::SetFinal(credits) => *credits,
            Self::Additions(additions) => {
                saturating_sum_credits(additions.iter().map(|(_, credits)| *credits))
            }
        }
    }
}

/// One compacted balance change of a wallet address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactedAddressBalanceChangeEntry {
    /// First block of the compacted range.
    pub start_block_height: BlockHeight,
    /// Last block of the compacted range.
    pub end_block_height: BlockHeight,
    /// The wallet address whose balance changed.
    pub address: PlatformAddress,
    /// What the range did to the balance.
    pub change: CompactedAddressBalanceChange,
}

impl PlatformAddressWallet {
    /// Balance changes of this wallet's addresses in Platform's recent
    /// (not yet compacted) zone, newest block first, then by address.
    ///
    /// Covers every platform payment account of the wallet. Pages through
    /// the recent zone from height 0 (see `fetch_pages_from_genesis`), so
    /// the result is the whole zone, not one server page.
    pub async fn recent_balance_changes(
        &self,
    ) -> Result<Vec<AddressBalanceChangeEntry>, PlatformWalletError> {
        let wallet_addresses = self.all_accounts_address_set().await?;
        if wallet_addresses.is_empty() {
            return Ok(Vec::new());
        }
        let sdk = self.sdk.as_ref();
        let blocks = fetch_pages_from_genesis(
            |start_height| async move {
                let page = RecentAddressBalanceChanges::fetch(
                    sdk,
                    RecentAddressBalanceChangesQuery::new(start_height),
                )
                .await?;
                Ok(page
                    .map(RecentAddressBalanceChanges::into_inner)
                    .unwrap_or_default())
            },
            |block: &BlockAddressBalanceChanges| block.block_height,
        )
        .await?;
        Ok(wallet_recent_balance_changes(blocks, &wallet_addresses))
    }

    /// Compacted balance changes of this wallet's addresses, newest range
    /// first (by range end, then range start), then by address.
    ///
    /// Covers every platform payment account of the wallet. Pages through
    /// every compacted range Platform still holds, from height 0; Drive
    /// expires compacted ranges after about a week, so the history is
    /// bounded.
    pub async fn compacted_balance_changes(
        &self,
    ) -> Result<Vec<CompactedAddressBalanceChangeEntry>, PlatformWalletError> {
        let wallet_addresses = self.all_accounts_address_set().await?;
        if wallet_addresses.is_empty() {
            return Ok(Vec::new());
        }
        let sdk = self.sdk.as_ref();
        let ranges = fetch_pages_from_genesis(
            |start_block_height| async move {
                let page = RecentCompactedAddressBalanceChanges::fetch(
                    sdk,
                    RecentCompactedAddressBalanceChangesQuery::new(start_block_height),
                )
                .await?;
                Ok(page
                    .map(RecentCompactedAddressBalanceChanges::into_inner)
                    .unwrap_or_default())
            },
            |range: &CompactedBlockAddressBalanceChanges| range.end_block_height,
        )
        .await?;
        Ok(wallet_compacted_balance_changes(ranges, &wallet_addresses))
    }

    /// Every address of every platform payment account on the wallet: each
    /// account's transient derived pool united with its hydrated
    /// `address_balances` keys, the same per-account union
    /// [`Self::candidate_address_set`] builds for one account. No address is
    /// derived here.
    ///
    /// The wallet-manager read lock is released before returning, so the
    /// network round-trips that follow never block a sync.
    pub(crate) async fn all_accounts_address_set(
        &self,
    ) -> Result<BTreeSet<PlatformAddress>, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "Wallet {:?} not found in wallet manager",
                hex::encode(self.wallet_id)
            ))
        })?;

        Ok(info
            .core_wallet
            .all_platform_payment_managed_accounts()
            .into_iter()
            .flat_map(|account| {
                merge_platform_payment_candidate_addresses(
                    account
                        .addresses
                        .addresses
                        .values()
                        .filter_map(|addr_info| {
                            PlatformP2PKHAddress::from_address(&addr_info.address).ok()
                        }),
                    account.address_balances.keys().copied(),
                )
            })
            .map(|p2pkh| PlatformAddress::P2pkh(p2pkh.to_bytes()))
            .collect())
    }
}

/// Fetch every page of a height-keyed query, starting at height 0.
///
/// `fetch_page(start)` returns the items at or after `start`, oldest first,
/// up to the server's page limit. The next page starts one block after the
/// highest `last_height_of` in the page. Paging stops on an empty page, or
/// when a page would not move the cursor forward (a server that ignores the
/// start height, or a height at `u64::MAX`); such a page holds nothing past
/// what was already read and is dropped.
pub(crate) async fn fetch_pages_from_genesis<T, F, Fut>(
    mut fetch_page: F,
    last_height_of: impl Fn(&T) -> BlockHeight,
) -> Result<Vec<T>, PlatformWalletError>
where
    F: FnMut(BlockHeight) -> Fut,
    Fut: Future<Output = Result<Vec<T>, PlatformWalletError>>,
{
    let mut cursor: BlockHeight = 0;
    let mut items = Vec::new();
    loop {
        let page = fetch_page(cursor).await?;
        let Some(page_last_height) = page.iter().map(&last_height_of).max() else {
            break;
        };
        let next_cursor = page_last_height.saturating_add(1);
        if next_cursor <= cursor {
            break;
        }
        items.extend(page);
        cursor = next_cursor;
    }
    Ok(items)
}

/// Keep the recent changes that touch `wallet_addresses`, newest block
/// first, then by address.
pub(crate) fn wallet_recent_balance_changes(
    blocks: Vec<BlockAddressBalanceChanges>,
    wallet_addresses: &BTreeSet<PlatformAddress>,
) -> Vec<AddressBalanceChangeEntry> {
    let mut entries: Vec<AddressBalanceChangeEntry> = blocks
        .into_iter()
        .flat_map(|block| {
            let block_height = block.block_height;
            block
                .changes
                .into_iter()
                .filter(|(address, _)| wallet_addresses.contains(address))
                .map(move |(address, operation)| AddressBalanceChangeEntry {
                    block_height,
                    address,
                    change: match operation {
                        CreditOperation::SetCredits(credits) => AddressBalanceChange::Set(credits),
                        CreditOperation::AddToCredits(credits) => {
                            AddressBalanceChange::Added(credits)
                        }
                    },
                })
        })
        .collect();
    entries.sort_by(|a, b| {
        (Reverse(a.block_height), &a.address).cmp(&(Reverse(b.block_height), &b.address))
    });
    entries
}

/// Keep the compacted changes that touch `wallet_addresses`, newest range
/// first (by range end, then range start), then by address.
pub(crate) fn wallet_compacted_balance_changes(
    ranges: Vec<CompactedBlockAddressBalanceChanges>,
    wallet_addresses: &BTreeSet<PlatformAddress>,
) -> Vec<CompactedAddressBalanceChangeEntry> {
    let mut entries: Vec<CompactedAddressBalanceChangeEntry> = ranges
        .into_iter()
        .flat_map(|range| {
            let start_block_height = range.start_block_height;
            let end_block_height = range.end_block_height;
            range
                .changes
                .into_iter()
                .filter(|(address, _)| wallet_addresses.contains(address))
                .map(
                    move |(address, operation)| CompactedAddressBalanceChangeEntry {
                        start_block_height,
                        end_block_height,
                        address,
                        change: match operation {
                            BlockAwareCreditOperation::SetCredits(credits) => {
                                CompactedAddressBalanceChange::SetFinal(credits)
                            }
                            BlockAwareCreditOperation::AddToCreditsOperations(additions) => {
                                CompactedAddressBalanceChange::Additions(
                                    additions.into_iter().rev().collect(),
                                )
                            }
                        },
                    },
                )
        })
        .collect();
    entries.sort_by(|a, b| {
        (
            Reverse(a.end_block_height),
            Reverse(a.start_block_height),
            &a.address,
        )
            .cmp(&(
                Reverse(b.end_block_height),
                Reverse(b.start_block_height),
                &b.address,
            ))
    });
    entries
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::{BTreeMap, BTreeSet};

    use dash_sdk::query_types::{BlockAddressBalanceChanges, CompactedBlockAddressBalanceChanges};
    use dpp::address_funds::PlatformAddress;
    use dpp::balances::credits::{BlockAwareCreditOperation, CreditOperation};
    use dpp::prelude::BlockHeight;

    use super::{
        fetch_pages_from_genesis, wallet_compacted_balance_changes, wallet_recent_balance_changes,
        AddressBalanceChange, AddressBalanceChangeEntry, CompactedAddressBalanceChange,
        CompactedAddressBalanceChangeEntry,
    };
    use crate::error::PlatformWalletError;

    fn address(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    fn recent_block(
        block_height: BlockHeight,
        changes: impl IntoIterator<Item = (PlatformAddress, CreditOperation)>,
    ) -> BlockAddressBalanceChanges {
        BlockAddressBalanceChanges {
            block_height,
            changes: changes.into_iter().collect(),
        }
    }

    fn compacted_range(
        start_block_height: BlockHeight,
        end_block_height: BlockHeight,
        changes: impl IntoIterator<Item = (PlatformAddress, BlockAwareCreditOperation)>,
    ) -> CompactedBlockAddressBalanceChanges {
        CompactedBlockAddressBalanceChanges {
            start_block_height,
            end_block_height,
            changes: changes.into_iter().collect(),
        }
    }

    #[test]
    fn should_keep_only_wallet_addresses_in_recent_changes() {
        let wallet = BTreeSet::from([address(1)]);
        let blocks = vec![recent_block(
            10,
            [
                (address(1), CreditOperation::AddToCredits(5)),
                (address(2), CreditOperation::SetCredits(7)),
            ],
        )];

        let entries = wallet_recent_balance_changes(blocks, &wallet);

        assert_eq!(
            entries,
            vec![AddressBalanceChangeEntry {
                block_height: 10,
                address: address(1),
                change: AddressBalanceChange::Added(5),
            }]
        );
    }

    #[test]
    fn should_order_recent_changes_newest_block_first_then_by_address() {
        let wallet = BTreeSet::from([address(1), address(2)]);
        let blocks = vec![
            recent_block(10, [(address(1), CreditOperation::SetCredits(100))]),
            recent_block(
                12,
                [
                    (address(2), CreditOperation::AddToCredits(3)),
                    (address(1), CreditOperation::AddToCredits(4)),
                ],
            ),
        ];

        let entries = wallet_recent_balance_changes(blocks, &wallet);

        let order: Vec<(BlockHeight, PlatformAddress)> = entries
            .iter()
            .map(|entry| (entry.block_height, entry.address))
            .collect();
        assert_eq!(
            order,
            vec![(12, address(1)), (12, address(2)), (10, address(1))]
        );
        assert_eq!(entries[2].change, AddressBalanceChange::Set(100));
    }

    #[test]
    fn should_keep_only_wallet_addresses_in_compacted_changes_and_order_newest_range_first() {
        let wallet = BTreeSet::from([address(1), address(3)]);
        let ranges = vec![
            compacted_range(
                1,
                5,
                [
                    (address(1), BlockAwareCreditOperation::SetCredits(50)),
                    (address(2), BlockAwareCreditOperation::SetCredits(60)),
                ],
            ),
            compacted_range(
                6,
                9,
                [(
                    address(3),
                    BlockAwareCreditOperation::AddToCreditsOperations(BTreeMap::from([
                        (6, 10),
                        (8, 20),
                    ])),
                )],
            ),
        ];

        let entries = wallet_compacted_balance_changes(ranges, &wallet);

        assert_eq!(
            entries,
            vec![
                CompactedAddressBalanceChangeEntry {
                    start_block_height: 6,
                    end_block_height: 9,
                    address: address(3),
                    change: CompactedAddressBalanceChange::Additions(vec![(8, 20), (6, 10)]),
                },
                CompactedAddressBalanceChangeEntry {
                    start_block_height: 1,
                    end_block_height: 5,
                    address: address(1),
                    change: CompactedAddressBalanceChange::SetFinal(50),
                },
            ]
        );
        assert_eq!(entries[0].change.credits(), 30);
        assert_eq!(entries[1].change.credits(), 50);
    }

    #[tokio::test]
    async fn should_page_from_genesis_until_a_page_is_empty() {
        let requested = RefCell::new(Vec::new());
        let pages = fetch_pages_from_genesis(
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                async move {
                    Ok::<_, PlatformWalletError>(match start {
                        0 => vec![3u64, 5],
                        6 => vec![9],
                        _ => Vec::new(),
                    })
                }
            },
            |height: &u64| *height,
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(pages, vec![3, 5, 9]);
        assert_eq!(*requested.borrow(), vec![0, 6, 10]);
    }

    #[tokio::test]
    async fn should_stop_paging_when_the_cursor_does_not_advance() {
        let requested = RefCell::new(Vec::new());
        // A server that ignores the start height and always answers with
        // the same page.
        let pages = fetch_pages_from_genesis(
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                async move { Ok::<_, PlatformWalletError>(vec![2u64, 4]) }
            },
            |height: &u64| *height,
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(pages, vec![2, 4]);
        assert_eq!(*requested.borrow(), vec![0, 5]);
    }

    #[tokio::test]
    async fn should_stop_paging_at_the_highest_block_height() {
        let requested = RefCell::new(Vec::new());
        let pages = fetch_pages_from_genesis(
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                async move { Ok::<_, PlatformWalletError>(vec![u64::MAX]) }
            },
            |height: &u64| *height,
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(pages, vec![u64::MAX]);
        assert_eq!(*requested.borrow(), vec![0, u64::MAX]);
    }

    #[tokio::test]
    async fn should_return_the_page_error() {
        let result = fetch_pages_from_genesis(
            |_start: BlockHeight| async move {
                Err::<Vec<u64>, _>(PlatformWalletError::AddressSync("offline".to_string()))
            },
            |height: &u64| *height,
        )
        .await;

        assert!(matches!(result, Err(PlatformWalletError::AddressSync(_))));
    }
}
