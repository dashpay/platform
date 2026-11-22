//! Balance change history of the wallet's own platform payment addresses.
//!
//! Platform keeps two zones of per-address balance changes: the recent zone
//! (one entry per block, not yet compacted) and the compacted zone (ranges of
//! blocks merged into one entry per address, expired by Drive after about a
//! week). Both queries are network-wide; this module pages through them,
//! keeps only the changes that touch an address of this wallet, reconciles
//! the two zones so no block shows in both, and orders them newest first so
//! a host can render them as they come.

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
use key_wallet::Network;

use super::saturating_sum_credits;
use super::wallet::{platform_payment_account_candidate_addresses, PlatformAddressWallet};
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

/// The balance change history of a wallet's platform addresses, from one
/// [`PlatformAddressWallet::balance_history`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressBalanceHistory {
    /// The wallet's network: the one every address here is displayed on.
    pub network: Network,
    /// Changes in Platform's recent (not yet compacted) zone, newest block
    /// first, then by address.
    pub recent: Vec<AddressBalanceChangeEntry>,
    /// Compacted changes, newest range first (by range end, then range
    /// start), then by address. Empty unless compacted history was asked for.
    /// No block here is also in `recent`.
    pub compacted: Vec<CompactedAddressBalanceChangeEntry>,
}

impl PlatformAddressWallet {
    /// Balance changes of this wallet's addresses (every platform payment
    /// account): the recent zone, and the compacted zone too when
    /// `include_compacted` is set.
    ///
    /// Both zones are paged from height 0 (see `fetch_pages_from_genesis`),
    /// so the result is the whole of each zone, not one server page. The
    /// recent zone is fetched first and the compacted zone is reconciled
    /// against it (see `drop_ranges_compacted_after_recent_fetch`), so a
    /// block compacted between the two fetches shows once, in `recent`.
    /// Drive expires compacted ranges after about a week, so the history is
    /// bounded.
    pub async fn balance_history(
        &self,
        include_compacted: bool,
    ) -> Result<AddressBalanceHistory, PlatformWalletError> {
        let network = self.network();
        let wallet_addresses = self.all_accounts_address_set().await?;
        if wallet_addresses.is_empty() {
            return Ok(AddressBalanceHistory {
                network,
                recent: Vec::new(),
                compacted: Vec::new(),
            });
        }
        let sdk = self.sdk.as_ref();
        let (recent, compacted) = wallet_balance_history(
            &wallet_addresses,
            include_compacted,
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
        )
        .await?;
        Ok(AddressBalanceHistory {
            network,
            recent,
            compacted,
        })
    }

    /// Every address of every platform payment account on the wallet: each
    /// account's transient derived pool united with its hydrated
    /// `address_balances` keys, the same per-account set
    /// [`Self::candidate_address_set`] returns for one account. No address is
    /// derived here.
    ///
    /// The wallet-manager read lock is released before returning, so the
    /// network round-trips that follow never block a sync.
    async fn all_accounts_address_set(
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
            .flat_map(platform_payment_account_candidate_addresses)
            .collect())
    }
}

/// The wallet rows of both zones, sorted: `fetch_recent_page` and
/// `fetch_compacted_page` fetch one network-wide page at a start height.
///
/// The recent zone is paged to the end FIRST; the compacted zone is paged
/// only when `include_compacted` is set, and then reconciled against the
/// lowest height the recent fetch saw.
async fn wallet_balance_history<RF, RFut, CF, CFut>(
    wallet_addresses: &BTreeSet<PlatformAddress>,
    include_compacted: bool,
    fetch_recent_page: RF,
    fetch_compacted_page: CF,
) -> Result<
    (
        Vec<AddressBalanceChangeEntry>,
        Vec<CompactedAddressBalanceChangeEntry>,
    ),
    PlatformWalletError,
>
where
    RF: FnMut(BlockHeight) -> RFut,
    RFut: Future<Output = Result<Vec<BlockAddressBalanceChanges>, PlatformWalletError>>,
    CF: FnMut(BlockHeight) -> CFut,
    CFut: Future<Output = Result<Vec<CompactedBlockAddressBalanceChanges>, PlatformWalletError>>,
{
    let recent_zone = fetch_pages_from_genesis(
        fetch_recent_page,
        |block: &BlockAddressBalanceChanges| (block.block_height, block.block_height),
        |block: BlockAddressBalanceChanges| wallet_rows_of_block(block, wallet_addresses),
    )
    .await?;
    let mut recent = recent_zone.rows;
    sort_recent_newest_first(&mut recent);

    let mut compacted = Vec::new();
    if include_compacted {
        let compacted_zone = fetch_pages_from_genesis(
            fetch_compacted_page,
            |range: &CompactedBlockAddressBalanceChanges| {
                (range.start_block_height, range.end_block_height)
            },
            |range: CompactedBlockAddressBalanceChanges| {
                wallet_rows_of_range(range, wallet_addresses)
            },
        )
        .await?;
        compacted = drop_ranges_compacted_after_recent_fetch(
            compacted_zone.rows,
            recent_zone.lowest_height,
        );
        sort_compacted_newest_first(&mut compacted);
    }
    Ok((recent, compacted))
}

/// The rows a paged zone kept, and the lowest height of every item the zone
/// returned before any row was filtered out.
#[derive(Debug, PartialEq, Eq)]
struct PagedZone<R> {
    rows: Vec<R>,
    /// `None` when the zone returned nothing.
    lowest_height: Option<BlockHeight>,
}

/// Fetch every page of a height-keyed query, starting at height 0, keeping
/// only what `keep` returns for each item.
///
/// `fetch_page(start)` returns the items at or after `start`, oldest first,
/// up to the server's page limit; `height_span_of` gives an item's first and
/// last block height. Each page is passed through `keep` as it arrives and
/// dropped before the next one is fetched, so memory holds one page plus the
/// kept rows. The cursor advances from the full page, kept or not: the next
/// page starts one block after the page's highest last height. Paging stops
/// on an empty page, or when a page would not move the cursor forward (a
/// server that ignores the start height, or a height at `u64::MAX`); such a
/// page holds nothing past what was already read and is dropped.
async fn fetch_pages_from_genesis<T, R, F, Fut>(
    mut fetch_page: F,
    height_span_of: impl Fn(&T) -> (BlockHeight, BlockHeight),
    mut keep: impl FnMut(T) -> Vec<R>,
) -> Result<PagedZone<R>, PlatformWalletError>
where
    F: FnMut(BlockHeight) -> Fut,
    Fut: Future<Output = Result<Vec<T>, PlatformWalletError>>,
{
    let mut cursor: BlockHeight = 0;
    let mut zone = PagedZone {
        rows: Vec::new(),
        lowest_height: None,
    };
    loop {
        let page = fetch_page(cursor).await?;
        let Some((page_lowest, page_highest)) = page.iter().map(&height_span_of).fold(
            None,
            |span: Option<(BlockHeight, BlockHeight)>, (first, last)| {
                Some(match span {
                    None => (first, last),
                    Some((lowest, highest)) => (lowest.min(first), highest.max(last)),
                })
            },
        ) else {
            break;
        };
        let next_cursor = page_highest.saturating_add(1);
        if next_cursor <= cursor {
            break;
        }
        zone.lowest_height = Some(
            zone.lowest_height
                .map_or(page_lowest, |lowest| lowest.min(page_lowest)),
        );
        for item in page {
            zone.rows.extend(keep(item));
        }
        cursor = next_cursor;
    }
    Ok(zone)
}

/// Drop every compacted row whose range starts at or after
/// `lowest_recent_height`, the lowest block height the recent fetch returned
/// network-wide (`None` when the recent zone was empty: every row is kept).
///
/// Drive compacts by draining the WHOLE recent zone, together with the
/// block being executed, into one new range in one batch
/// (`compact_address_balances_with_current_block_v0`), so a new range
/// starts at the lowest height the recent zone held at that moment. Hence:
/// - a range created before the recent fetch ends below every block that
///   fetch saw (those blocks were stored after it), so it starts below
///   `lowest_recent_height` and is kept;
/// - a range created after the recent fetch drained the blocks that fetch
///   returned (it starts at `lowest_recent_height`, or above for a later
///   compaction), so its changes are already in the recent rows and it is
///   dropped.
///
/// Because the recent zone is fetched first, a block compacted after the
/// recent fetch was in that fetch, so dropping these ranges leaves no gap.
/// The recent fetch is only atomic per page: a compaction between two of
/// its pages drains blocks stored after the earlier page, which are then in
/// neither zone. Drive compacts before the recent zone reaches
/// `max_blocks_before_compaction` (64) blocks and a recent page holds up to
/// 100, so the fetch is one page plus the empty page that ends it, and that
/// window is the one round trip between them.
fn drop_ranges_compacted_after_recent_fetch(
    compacted: Vec<CompactedAddressBalanceChangeEntry>,
    lowest_recent_height: Option<BlockHeight>,
) -> Vec<CompactedAddressBalanceChangeEntry> {
    let Some(lowest_recent_height) = lowest_recent_height else {
        return compacted;
    };
    compacted
        .into_iter()
        .filter(|entry| entry.start_block_height < lowest_recent_height)
        .collect()
}

/// The changes of one recent-zone block that touch `wallet_addresses`.
fn wallet_rows_of_block(
    block: BlockAddressBalanceChanges,
    wallet_addresses: &BTreeSet<PlatformAddress>,
) -> Vec<AddressBalanceChangeEntry> {
    let block_height = block.block_height;
    block
        .changes
        .into_iter()
        .filter(|(address, _)| wallet_addresses.contains(address))
        .map(|(address, operation)| AddressBalanceChangeEntry {
            block_height,
            address,
            change: match operation {
                CreditOperation::SetCredits(credits) => AddressBalanceChange::Set(credits),
                CreditOperation::AddToCredits(credits) => AddressBalanceChange::Added(credits),
            },
        })
        .collect()
}

/// The changes of one compacted range that touch `wallet_addresses`.
fn wallet_rows_of_range(
    range: CompactedBlockAddressBalanceChanges,
    wallet_addresses: &BTreeSet<PlatformAddress>,
) -> Vec<CompactedAddressBalanceChangeEntry> {
    let start_block_height = range.start_block_height;
    let end_block_height = range.end_block_height;
    range
        .changes
        .into_iter()
        .filter(|(address, _)| wallet_addresses.contains(address))
        .map(|(address, operation)| CompactedAddressBalanceChangeEntry {
            start_block_height,
            end_block_height,
            address,
            change: match operation {
                BlockAwareCreditOperation::SetCredits(credits) => {
                    CompactedAddressBalanceChange::SetFinal(credits)
                }
                BlockAwareCreditOperation::AddToCreditsOperations(additions) => {
                    CompactedAddressBalanceChange::Additions(additions.into_iter().rev().collect())
                }
            },
        })
        .collect()
}

/// Newest block first, then by address.
fn sort_recent_newest_first(entries: &mut [AddressBalanceChangeEntry]) {
    entries.sort_by(|a, b| {
        (Reverse(a.block_height), &a.address).cmp(&(Reverse(b.block_height), &b.address))
    });
}

/// Newest range first (by range end, then range start), then by address.
fn sort_compacted_newest_first(entries: &mut [CompactedAddressBalanceChangeEntry]) {
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
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;

    use dash_sdk::query_types::{BlockAddressBalanceChanges, CompactedBlockAddressBalanceChanges};
    use dpp::address_funds::PlatformAddress;
    use dpp::balances::credits::{BlockAwareCreditOperation, CreditOperation};
    use dpp::prelude::BlockHeight;
    use key_wallet::bip32::{ChildNumber, DerivationPath};
    use key_wallet::managed_account::address_pool::{
        AddressInfo as PoolAddressInfo, AddressPool, AddressPoolType,
    };
    use key_wallet::managed_account::managed_platform_account::ManagedPlatformAccount;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::{Network, PlatformP2PKHAddress};
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use super::{
        drop_ranges_compacted_after_recent_fetch, fetch_pages_from_genesis,
        sort_compacted_newest_first, sort_recent_newest_first, wallet_balance_history,
        wallet_rows_of_block, wallet_rows_of_range, AddressBalanceChange,
        AddressBalanceChangeEntry, CompactedAddressBalanceChange,
        CompactedAddressBalanceChangeEntry, PagedZone,
    };
    use crate::error::PlatformWalletError;
    use crate::test_support::platform_address_wallet_in;
    use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
    use crate::wallet::PlatformAddressWallet;

    fn address(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    /// The DIP-17 base path of platform payment account `account`.
    fn platform_account_path(account: u32) -> DerivationPath {
        DerivationPath::from(vec![
            ChildNumber::from_hardened_idx(9).expect("purpose"),
            ChildNumber::from_hardened_idx(1).expect("coin type"),
            ChildNumber::from_hardened_idx(17).expect("feature"),
            ChildNumber::from_hardened_idx(0).expect("subfeature"),
            ChildNumber::from_hardened_idx(account).expect("account"),
        ])
    }

    /// An empty testnet address pool for platform payment account `account`.
    fn empty_pool(account: u32) -> AddressPool {
        AddressPool::new_without_generation(
            platform_account_path(account),
            AddressPoolType::Absent,
            20,
            Network::Testnet,
        )
    }

    /// A derived-pool entry at `index` of `account` for the P2PKH hash
    /// `[byte; 20]`, built from its script (no private key needed).
    fn pool_entry(account: u32, byte: u8, index: u32) -> PoolAddressInfo {
        let address = PlatformP2PKHAddress::new([byte; 20]).to_address(Network::Testnet);
        PoolAddressInfo::new_from_script_pubkey_p2pkh(
            address.script_pubkey(),
            index,
            platform_account_path(account),
            Network::Testnet,
        )
        .expect("known testnet P2PKH script")
    }

    /// A testnet wallet manager holding one wallet with no accounts, and that
    /// wallet's id.
    fn wallet_manager_with_empty_wallet() -> (WalletManager<PlatformWalletInfo>, WalletId) {
        let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
        let wallet_id = wm
            .create_wallet_with_random_mnemonic(WalletAccountCreationOptions::None)
            .expect("create wallet");
        (wm, wallet_id)
    }

    /// A `PlatformAddressWallet` over `wm`'s wallet `wallet_id` on a mock SDK
    /// with no expectations, so any network fetch fails.
    fn address_wallet_without_network(
        wm: WalletManager<PlatformWalletInfo>,
        wallet_id: WalletId,
    ) -> PlatformAddressWallet {
        let sdk = Arc::new(dash_sdk::Sdk::new_mock());
        platform_address_wallet_in(sdk, Arc::new(RwLock::new(wm)), wallet_id)
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

    /// A wallet row of a compacted range that set the balance to `credits`.
    fn compacted_row(
        start_block_height: BlockHeight,
        end_block_height: BlockHeight,
        credits: u64,
    ) -> CompactedAddressBalanceChangeEntry {
        CompactedAddressBalanceChangeEntry {
            start_block_height,
            end_block_height,
            address: address(1),
            change: CompactedAddressBalanceChange::SetFinal(credits),
        }
    }

    /// Fake one-page server: `pages` answers the start heights it lists and
    /// any other start height with an empty page.
    fn page_at<T: Clone>(pages: &[(BlockHeight, Vec<T>)], start: BlockHeight) -> Vec<T> {
        pages
            .iter()
            .find(|(page_start, _)| *page_start == start)
            .map(|(_, page)| page.clone())
            .unwrap_or_default()
    }

    #[test]
    fn should_keep_only_wallet_addresses_in_recent_changes() {
        let wallet = BTreeSet::from([address(1)]);
        let block = recent_block(
            10,
            [
                (address(1), CreditOperation::AddToCredits(5)),
                (address(2), CreditOperation::SetCredits(7)),
            ],
        );

        let entries = wallet_rows_of_block(block, &wallet);

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

        let mut entries: Vec<AddressBalanceChangeEntry> = blocks
            .into_iter()
            .flat_map(|block| wallet_rows_of_block(block, &wallet))
            .collect();
        sort_recent_newest_first(&mut entries);

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

        let mut entries: Vec<CompactedAddressBalanceChangeEntry> = ranges
            .into_iter()
            .flat_map(|range| wallet_rows_of_range(range, &wallet))
            .collect();
        sort_compacted_newest_first(&mut entries);

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
        let zone = fetch_pages_from_genesis(
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
            |height: &u64| (*height, *height),
            |height: u64| vec![height],
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(
            zone,
            PagedZone {
                rows: vec![3, 5, 9],
                lowest_height: Some(3),
            }
        );
        assert_eq!(*requested.borrow(), vec![0, 6, 10]);
    }

    #[tokio::test]
    async fn should_advance_past_a_page_with_no_kept_rows() {
        let requested = RefCell::new(Vec::new());
        let zone = fetch_pages_from_genesis(
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
            |height: &u64| (*height, *height),
            // Keep only the second page's item.
            |height: u64| {
                if height == 9 {
                    vec![height]
                } else {
                    Vec::new()
                }
            },
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(zone.rows, vec![9]);
        // The lowest height counts the page whose items were all dropped.
        assert_eq!(zone.lowest_height, Some(3));
        assert_eq!(*requested.borrow(), vec![0, 6, 10]);
    }

    #[tokio::test]
    async fn should_report_no_lowest_height_for_an_empty_zone() {
        let zone = fetch_pages_from_genesis(
            |_start: BlockHeight| async move { Ok::<_, PlatformWalletError>(Vec::<u64>::new()) },
            |height: &u64| (*height, *height),
            |height: u64| vec![height],
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(
            zone,
            PagedZone {
                rows: Vec::new(),
                lowest_height: None,
            }
        );
    }

    #[tokio::test]
    async fn should_stop_paging_when_the_cursor_does_not_advance() {
        let requested = RefCell::new(Vec::new());
        // A server that ignores the start height and always answers with
        // the same page.
        let zone = fetch_pages_from_genesis(
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                async move { Ok::<_, PlatformWalletError>(vec![2u64, 4]) }
            },
            |height: &u64| (*height, *height),
            |height: u64| vec![height],
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(zone.rows, vec![2, 4]);
        assert_eq!(zone.lowest_height, Some(2));
        assert_eq!(*requested.borrow(), vec![0, 5]);
    }

    #[tokio::test]
    async fn should_stop_paging_at_the_highest_block_height() {
        let requested = RefCell::new(Vec::new());
        let zone = fetch_pages_from_genesis(
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                async move { Ok::<_, PlatformWalletError>(vec![u64::MAX]) }
            },
            |height: &u64| (*height, *height),
            |height: u64| vec![height],
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(zone.rows, vec![u64::MAX]);
        assert_eq!(*requested.borrow(), vec![0, u64::MAX]);
    }

    #[tokio::test]
    async fn should_advance_the_cursor_past_a_range_end() {
        let requested = RefCell::new(Vec::new());
        let zone = fetch_pages_from_genesis(
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                async move {
                    Ok::<_, PlatformWalletError>(match start {
                        0 => vec![(1u64, 5u64), (6, 9)],
                        _ => Vec::new(),
                    })
                }
            },
            |range: &(u64, u64)| *range,
            |range: (u64, u64)| vec![range],
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(zone.rows, vec![(1, 5), (6, 9)]);
        assert_eq!(zone.lowest_height, Some(1));
        assert_eq!(*requested.borrow(), vec![0, 10]);
    }

    #[tokio::test]
    async fn should_return_the_page_error() {
        let result = fetch_pages_from_genesis(
            |_start: BlockHeight| async move {
                Err::<Vec<u64>, _>(PlatformWalletError::AddressSync("offline".to_string()))
            },
            |height: &u64| (*height, *height),
            |height: u64| vec![height],
        )
        .await;

        assert!(matches!(result, Err(PlatformWalletError::AddressSync(_))));
    }

    #[test]
    fn should_drop_a_compacted_range_starting_at_or_after_the_lowest_recent_height() {
        let compacted = vec![compacted_row(20, 30, 1), compacted_row(25, 40, 2)];

        let kept = drop_ranges_compacted_after_recent_fetch(compacted, Some(20));

        assert!(kept.is_empty());
    }

    #[test]
    fn should_keep_a_compacted_range_entirely_before_the_lowest_recent_height() {
        let compacted = vec![compacted_row(1, 19, 1), compacted_row(20, 30, 2)];

        let kept = drop_ranges_compacted_after_recent_fetch(compacted, Some(20));

        assert_eq!(kept, vec![compacted_row(1, 19, 1)]);
    }

    #[test]
    fn should_keep_every_compacted_range_when_the_recent_zone_is_empty() {
        let compacted = vec![compacted_row(1, 19, 1), compacted_row(20, 30, 2)];

        let kept = drop_ranges_compacted_after_recent_fetch(compacted.clone(), None);

        assert_eq!(kept, compacted);
    }

    #[tokio::test]
    async fn should_drop_compacted_ranges_by_network_wide_recent_heights() {
        let wallet = BTreeSet::from([address(1)]);
        // The recent zone's lowest block (10) has no change of this wallet.
        let recent_pages = vec![(
            0,
            vec![
                recent_block(10, [(address(2), CreditOperation::AddToCredits(7))]),
                recent_block(12, [(address(1), CreditOperation::AddToCredits(5))]),
            ],
        )];
        // (3, 9) was compacted before the recent fetch; (10, 12) after it,
        // from the very blocks the recent fetch returned.
        let compacted_pages = vec![(
            0,
            vec![
                compacted_range(
                    3,
                    9,
                    [(address(1), BlockAwareCreditOperation::SetCredits(40))],
                ),
                compacted_range(
                    10,
                    12,
                    [(
                        address(1),
                        BlockAwareCreditOperation::AddToCreditsOperations(BTreeMap::from([(
                            12, 5,
                        )])),
                    )],
                ),
            ],
        )];

        let (recent, compacted) = wallet_balance_history(
            &wallet,
            true,
            |start| {
                let page = page_at(&recent_pages, start);
                async move { Ok(page) }
            },
            |start| {
                let page = page_at(&compacted_pages, start);
                async move { Ok(page) }
            },
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(
            recent,
            vec![AddressBalanceChangeEntry {
                block_height: 12,
                address: address(1),
                change: AddressBalanceChange::Added(5),
            }]
        );
        assert_eq!(compacted, vec![compacted_row(3, 9, 40)]);
    }

    #[tokio::test]
    async fn should_fetch_the_recent_zone_before_the_compacted_zone() {
        let wallet = BTreeSet::from([address(1)]);
        let calls = RefCell::new(Vec::new());

        wallet_balance_history(
            &wallet,
            true,
            |start| {
                calls.borrow_mut().push(("recent", start));
                async move { Ok(Vec::new()) }
            },
            |start| {
                calls.borrow_mut().push(("compacted", start));
                async move { Ok(Vec::new()) }
            },
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(*calls.borrow(), vec![("recent", 0), ("compacted", 0)]);
    }

    #[tokio::test]
    async fn should_not_fetch_the_compacted_zone_unless_asked() {
        let wallet = BTreeSet::from([address(1)]);
        let compacted_fetched = RefCell::new(false);

        let (_, compacted) = wallet_balance_history(
            &wallet,
            false,
            |_start| async move { Ok(Vec::new()) },
            |_start| {
                *compacted_fetched.borrow_mut() = true;
                async move { Ok(Vec::new()) }
            },
        )
        .await
        .expect("fake pages never fail");

        assert!(!*compacted_fetched.borrow());
        assert!(compacted.is_empty());
    }

    #[tokio::test]
    async fn should_collect_derived_and_hydrated_addresses_of_every_platform_payment_account() {
        let derived_only_byte = 0x41u8;
        let hydrated_only_byte = 0x42u8;
        let (mut wm, wallet_id) = wallet_manager_with_empty_wallet();
        {
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");

            // Account 0: an address only in the derived pool, no balance.
            let mut pool = empty_pool(0);
            pool.addresses
                .insert(0, pool_entry(0, derived_only_byte, 0));
            info.core_wallet
                .accounts
                .insert_platform_account(ManagedPlatformAccount::new(0, 0, pool, false));

            // Account 1: an empty pool and one address only in the hydrated
            // balance map, as `initialize_from_persisted` leaves it.
            let mut hydrated_account = ManagedPlatformAccount::new(1, 0, empty_pool(1), false);
            hydrated_account.set_address_credit_balance(
                PlatformP2PKHAddress::new([hydrated_only_byte; 20]),
                1_000,
                None,
            );
            info.core_wallet
                .accounts
                .insert_platform_account(hydrated_account);
        }
        let wallet = address_wallet_without_network(wm, wallet_id);

        let addresses = wallet
            .all_accounts_address_set()
            .await
            .expect("the wallet is in the manager");

        assert_eq!(
            addresses,
            BTreeSet::from([address(derived_only_byte), address(hydrated_only_byte)])
        );
    }

    #[tokio::test]
    async fn should_return_empty_history_without_fetching_when_the_wallet_has_no_platform_payment_accounts(
    ) {
        let (wm, wallet_id) = wallet_manager_with_empty_wallet();
        assert_eq!(
            wm.get_wallet_info(&wallet_id)
                .expect("wallet info")
                .core_wallet
                .platform_payment_account_count(),
            0,
            "test setup: the wallet must have no platform payment account"
        );
        // The mock SDK has no expectations: any fetch would return an error.
        let wallet = address_wallet_without_network(wm, wallet_id);

        for include_compacted in [false, true] {
            let history = wallet
                .balance_history(include_compacted)
                .await
                .expect("no fetch is made for a wallet without addresses");

            assert_eq!(history.network, wallet.network());
            assert!(history.recent.is_empty());
            assert!(history.compacted.is_empty());
        }
    }
}
