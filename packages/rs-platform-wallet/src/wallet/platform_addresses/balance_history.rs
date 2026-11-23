//! Balance change history of the wallet's own platform payment addresses.
//!
//! Platform keeps two zones of per-address balance changes: the recent zone
//! (one entry per block, not yet compacted) and the compacted zone (ranges of
//! blocks merged into one entry per address, expired by Drive after about a
//! week). Both queries are network-wide; this module pages through them,
//! keeps only the changes that touch an address of this wallet, reconciles
//! the two zones so every block shows exactly once, and orders them newest
//! first so a host can render them as they come.

use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::future::Future;

use dash_sdk::drive::verify::address_funds::{
    COMPACTED_ADDRESS_BALANCE_CHANGES_QUERY_LIMIT, RECENT_ADDRESS_BALANCE_CHANGES_QUERY_LIMIT,
};
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
    /// first, then by address. With compacted history, a block that a
    /// compacted range covers is left out here: the range shows it.
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
    /// so the result is the whole of each zone, not one server page. Every
    /// page must be proved at or above the highest height read so far; a
    /// lagging page is asked for again a bounded number of times, then the
    /// call fails with
    /// [`PlatformWalletError::BalanceHistoryInconsistentHeights`] rather than
    /// return a partial history. With compacted history every compacted
    /// range is kept and a recent block a range covers shows only in that
    /// range (see `wallet_balance_history`). Drive expires compacted ranges
    /// after about a week, so the history is bounded.
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
                let (page, metadata) = RecentAddressBalanceChanges::fetch_with_metadata(
                    sdk,
                    RecentAddressBalanceChangesQuery::new(start_height),
                    None,
                )
                .await?;
                Ok(ProvedPage {
                    items: page
                        .map(RecentAddressBalanceChanges::into_inner)
                        .unwrap_or_default(),
                    proved_height: metadata.height,
                })
            },
            |start_block_height| async move {
                let (page, metadata) = RecentCompactedAddressBalanceChanges::fetch_with_metadata(
                    sdk,
                    RecentCompactedAddressBalanceChangesQuery::new(start_block_height),
                    None,
                )
                .await?;
                Ok(ProvedPage {
                    items: page
                        .map(RecentCompactedAddressBalanceChanges::into_inner)
                        .unwrap_or_default(),
                    proved_height: metadata.height,
                })
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

/// How many times ONE page is requested (at the same cursor) before the
/// read gives up on it being proved at or above the height floor
/// ([`fetch_page_at_floor`]). It counts per page, not per read: each page
/// gets this many attempts, and a page that passes does not spend the next
/// page's attempts. The SDK accepts an answer up to one block behind the
/// newest one it has seen, so a page can come from a node one block behind
/// the floor; asking again usually reaches a node that has caught up (or
/// the same node a block later), so a few attempts suffice while still
/// bounding a read against a lagging network.
const BALANCE_HISTORY_PAGE_ATTEMPTS: usize = 3;

/// One page of a height-keyed query and the block height its proof was
/// verified at.
struct ProvedPage<T> {
    items: Vec<T>,
    proved_height: BlockHeight,
}

/// The wallet rows of both zones, reconciled and sorted:
/// `fetch_recent_page` and `fetch_compacted_page` fetch one network-wide
/// page at a start height, with its proved height.
///
/// One height floor is carried across the whole read: the highest height
/// any page used so far was proved at, never lowered. Every page, empty
/// continuations included, must be proved at or above it before it is used
/// ([`fetch_page_at_floor`]); a page below it is asked for again at the same
/// cursor, and the scan is never restarted. The recent zone is paged first;
/// it is the whole answer unless `include_compacted` is set, in which case
/// the compacted zone is paged next under the same floor.
///
/// Why the floor makes the two zones fit: Drive moves blocks out of the
/// recent zone only by compacting them, in one batch, into a new range that
/// ends at the block being executed (`compact_address_balances_with_current_block_v0`,
/// which also merges that block's own changes straight into the range, so
/// they are never in the recent zone). A block missing from the recent read
/// was compacted at a height no higher than the page that skipped it, hence
/// no higher than the floor, so every compacted page (proved at or above the
/// floor) holds its range. When a compacted page raises the floor past
/// earlier recent pages, any range created in between covers recent blocks
/// that were already read; [`drop_recent_rows_covered_by_ranges`] drops
/// those recent rows and the range shows them, so every block shows once.
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
    RFut: Future<Output = Result<ProvedPage<BlockAddressBalanceChanges>, PlatformWalletError>>,
    CF: FnMut(BlockHeight) -> CFut,
    CFut: Future<
        Output = Result<ProvedPage<CompactedBlockAddressBalanceChanges>, PlatformWalletError>,
    >,
{
    let mut floor: BlockHeight = 0;
    let recent_zone = fetch_pages_from_genesis(
        &mut floor,
        usize::from(RECENT_ADDRESS_BALANCE_CHANGES_QUERY_LIMIT),
        fetch_recent_page,
        |block: &BlockAddressBalanceChanges| (block.block_height, block.block_height),
        |block: BlockAddressBalanceChanges| wallet_rows_of_block(block, wallet_addresses),
    )
    .await?;
    if !include_compacted {
        let mut recent = recent_zone.rows;
        sort_recent_newest_first(&mut recent);
        return Ok((recent, Vec::new()));
    }

    let compacted_zone = fetch_pages_from_genesis(
        &mut floor,
        usize::from(COMPACTED_ADDRESS_BALANCE_CHANGES_QUERY_LIMIT),
        fetch_compacted_page,
        |range: &CompactedBlockAddressBalanceChanges| {
            (range.start_block_height, range.end_block_height)
        },
        |range: CompactedBlockAddressBalanceChanges| wallet_rows_of_range(range, wallet_addresses),
    )
    .await?;
    let mut recent = drop_recent_rows_covered_by_ranges(recent_zone.rows, compacted_zone.spans);
    sort_recent_newest_first(&mut recent);
    let mut compacted = compacted_zone.rows;
    sort_compacted_newest_first(&mut compacted);
    Ok((recent, compacted))
}

/// What a paged zone kept, and what it saw before the wallet filter.
#[derive(Debug, PartialEq, Eq)]
struct PagedZone<R> {
    rows: Vec<R>,
    /// The first and last block height of every item the zone returned,
    /// network-wide (before the wallet filter).
    spans: Vec<(BlockHeight, BlockHeight)>,
}

/// Fetch every page of a height-keyed query, starting at height 0, keeping
/// only what `keep` returns for each item.
///
/// `fetch_page(start)` returns the items at or after `start`, oldest first,
/// up to `page_limit` (the server's page limit), with the height its proof
/// was verified at; `height_span_of` gives an item's first and last block
/// height. Every page is taken through [`fetch_page_at_floor`], so it is
/// proved at or above `floor` and raises `floor` to its own height. Each
/// page is passed through `keep` as it arrives and dropped before the next
/// one is fetched, so memory holds one page plus the kept rows and the
/// items' height spans. The cursor advances from the full page, kept or
/// not: the next page starts one block after the page's highest last
/// height.
///
/// Paging stops on an empty page; on a short page (fewer than `page_limit`
/// items: at its proved height nothing is left past it, so no further
/// request is made); or when a page would not move the cursor forward (a
/// server that ignores the start height, or a height at `u64::MAX`), whose
/// items hold nothing past what was already read and are dropped.
async fn fetch_pages_from_genesis<T, R, F, Fut>(
    floor: &mut BlockHeight,
    page_limit: usize,
    mut fetch_page: F,
    height_span_of: impl Fn(&T) -> (BlockHeight, BlockHeight),
    mut keep: impl FnMut(T) -> Vec<R>,
) -> Result<PagedZone<R>, PlatformWalletError>
where
    F: FnMut(BlockHeight) -> Fut,
    Fut: Future<Output = Result<ProvedPage<T>, PlatformWalletError>>,
{
    let mut cursor: BlockHeight = 0;
    let mut zone = PagedZone {
        rows: Vec::new(),
        spans: Vec::new(),
    };
    loop {
        let page = fetch_page_at_floor(&mut fetch_page, cursor, floor).await?;
        let item_count = page.len();
        let Some(page_highest) = page.iter().map(|item| height_span_of(item).1).max() else {
            break;
        };
        let next_cursor = page_highest.saturating_add(1);
        if next_cursor <= cursor {
            break;
        }
        for item in page {
            zone.spans.push(height_span_of(&item));
            zone.rows.extend(keep(item));
        }
        if item_count < page_limit {
            break;
        }
        cursor = next_cursor;
    }
    Ok(zone)
}

/// Request the page at `cursor` until it is proved at or above `floor`
/// ([`raised_floor`]), at most [`BALANCE_HISTORY_PAGE_ATTEMPTS`] times, and
/// raise `floor` to its proved height. A page below the floor is never
/// used: when every attempt is below it, the read fails with
/// [`PlatformWalletError::BalanceHistoryInconsistentHeights`] rather than
/// return a partial history.
async fn fetch_page_at_floor<T, F, Fut>(
    fetch_page: &mut F,
    cursor: BlockHeight,
    floor: &mut BlockHeight,
) -> Result<Vec<T>, PlatformWalletError>
where
    F: FnMut(BlockHeight) -> Fut,
    Fut: Future<Output = Result<ProvedPage<T>, PlatformWalletError>>,
{
    let mut proved_height = 0;
    for _ in 0..BALANCE_HISTORY_PAGE_ATTEMPTS {
        let page = fetch_page(cursor).await?;
        if let Some(raised) = raised_floor(*floor, page.proved_height) {
            *floor = raised;
            return Ok(page.items);
        }
        proved_height = page.proved_height;
    }
    Err(PlatformWalletError::BalanceHistoryInconsistentHeights {
        attempts: BALANCE_HISTORY_PAGE_ATTEMPTS,
        required_height: *floor,
        proved_height,
    })
}

/// The floor after a page proved at `proved_height`, or `None` when the
/// page is below `floor` and must not be used. A page at or above the floor
/// raises it to its own height; the floor never goes down.
fn raised_floor(floor: BlockHeight, proved_height: BlockHeight) -> Option<BlockHeight> {
    (proved_height >= floor).then_some(proved_height)
}

/// Drop every recent row whose block lies inside one of `range_spans`, the
/// `[start, end]` spans of every compacted range returned (network-wide,
/// whether or not a range holds a wallet row).
///
/// A block inside a compacted range has all its changes merged into that
/// range, so the range already shows the row (merged with the range's other
/// blocks); keeping it in the recent rows too would show it twice. Every
/// compacted range is kept, so no block is lost, including the block whose
/// execution triggered the compaction, which Drive writes only into the
/// range. The cost is per-block detail for superseded recent blocks.
fn drop_recent_rows_covered_by_ranges(
    recent: Vec<AddressBalanceChangeEntry>,
    mut range_spans: Vec<(BlockHeight, BlockHeight)>,
) -> Vec<AddressBalanceChangeEntry> {
    if range_spans.is_empty() {
        return recent;
    }
    range_spans.sort_unstable();
    // Merge overlapping or touching spans so one lookup decides each row.
    let mut coverage: Vec<(BlockHeight, BlockHeight)> = Vec::with_capacity(range_spans.len());
    for (start, end) in range_spans {
        match coverage.last_mut() {
            Some((_, last_end)) if start <= last_end.saturating_add(1) => {
                *last_end = (*last_end).max(end);
            }
            _ => coverage.push((start, end)),
        }
    }
    recent
        .into_iter()
        .filter(|entry| {
            let height = entry.block_height;
            let after = coverage.partition_point(|(start, _)| *start <= height);
            let covering = after.checked_sub(1).and_then(|index| coverage.get(index));
            !matches!(covering, Some((_, end)) if height <= *end)
        })
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
    use std::cell::{Cell, RefCell};
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;

    use dash_sdk::drive::verify::address_funds::COMPACTED_ADDRESS_BALANCE_CHANGES_QUERY_LIMIT;
    use dash_sdk::platform::query::{
        RecentAddressBalanceChangesQuery, RecentCompactedAddressBalanceChangesQuery,
    };
    use dash_sdk::query_types::{
        BlockAddressBalanceChanges, CompactedBlockAddressBalanceChanges,
        RecentAddressBalanceChanges, RecentCompactedAddressBalanceChanges,
    };
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
        drop_recent_rows_covered_by_ranges, fetch_pages_from_genesis, raised_floor,
        sort_compacted_newest_first, sort_recent_newest_first, wallet_balance_history,
        wallet_rows_of_block, wallet_rows_of_range, AddressBalanceChange,
        AddressBalanceChangeEntry, CompactedAddressBalanceChange,
        CompactedAddressBalanceChangeEntry, PagedZone, ProvedPage, BALANCE_HISTORY_PAGE_ATTEMPTS,
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

    /// A wallet (`address(1)`) row of block `block_height` that added
    /// `credits`.
    fn recent_row(block_height: BlockHeight, credits: u64) -> AddressBalanceChangeEntry {
        AddressBalanceChangeEntry {
            block_height,
            address: address(1),
            change: AddressBalanceChange::Added(credits),
        }
    }

    /// A page of `items` proved at `proved_height`.
    fn proved<T>(items: Vec<T>, proved_height: BlockHeight) -> ProvedPage<T> {
        ProvedPage {
            items,
            proved_height,
        }
    }

    /// Fake server proved at `proved_height`: `pages` answers the start
    /// heights it lists and any other start height with an empty page.
    fn page_at<T: Clone>(
        pages: &[(BlockHeight, Vec<T>)],
        start: BlockHeight,
        proved_height: BlockHeight,
    ) -> ProvedPage<T> {
        proved(
            pages
                .iter()
                .find(|(page_start, _)| *page_start == start)
                .map(|(_, page)| page.clone())
                .unwrap_or_default(),
            proved_height,
        )
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
        let mut floor = 0;
        let zone = fetch_pages_from_genesis(
            &mut floor,
            2,
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                async move {
                    Ok::<_, PlatformWalletError>(match start {
                        0 => proved(vec![3u64, 5], 20),
                        6 => proved(vec![8, 9], 21),
                        _ => proved(Vec::new(), 21),
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
                rows: vec![3, 5, 8, 9],
                spans: vec![(3, 3), (5, 5), (8, 8), (9, 9)],
            }
        );
        assert_eq!(*requested.borrow(), vec![0, 6, 10]);
        assert_eq!(floor, 21, "every page raises the floor to its height");
    }

    #[tokio::test]
    async fn should_stop_on_a_short_page_without_another_request() {
        let requested = RefCell::new(Vec::new());
        let mut floor = 0;
        let zone = fetch_pages_from_genesis(
            &mut floor,
            3,
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                async move { Ok::<_, PlatformWalletError>(proved(vec![3u64, 5], 20)) }
            },
            |height: &u64| (*height, *height),
            |height: u64| vec![height],
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(zone.rows, vec![3, 5]);
        assert_eq!(*requested.borrow(), vec![0]);
    }

    #[tokio::test]
    async fn should_advance_past_a_page_with_no_kept_rows() {
        let requested = RefCell::new(Vec::new());
        let zone = fetch_pages_from_genesis(
            &mut 0,
            2,
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                async move {
                    Ok::<_, PlatformWalletError>(match start {
                        0 => proved(vec![3u64, 5], 10),
                        6 => proved(vec![9], 10),
                        _ => proved(Vec::new(), 10),
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
        // The spans count the page whose items were all dropped.
        assert_eq!(zone.spans, vec![(3, 3), (5, 5), (9, 9)]);
        // The second page is short, so it ends the scan.
        assert_eq!(*requested.borrow(), vec![0, 6]);
    }

    #[tokio::test]
    async fn should_stop_paging_when_the_cursor_does_not_advance() {
        let requested = RefCell::new(Vec::new());
        // A server that ignores the start height and always answers with
        // the same full page.
        let zone = fetch_pages_from_genesis(
            &mut 0,
            2,
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                async move { Ok::<_, PlatformWalletError>(proved(vec![2u64, 4], 10)) }
            },
            |height: &u64| (*height, *height),
            |height: u64| vec![height],
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(zone.rows, vec![2, 4]);
        assert_eq!(zone.spans, vec![(2, 2), (4, 4)]);
        assert_eq!(*requested.borrow(), vec![0, 5]);
    }

    #[tokio::test]
    async fn should_stop_paging_at_the_highest_block_height() {
        let requested = RefCell::new(Vec::new());
        let zone = fetch_pages_from_genesis(
            &mut 0,
            1,
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                async move { Ok::<_, PlatformWalletError>(proved(vec![u64::MAX], 10)) }
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
            &mut 0,
            2,
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                async move {
                    Ok::<_, PlatformWalletError>(match start {
                        0 => proved(vec![(1u64, 5u64), (6, 9)], 10),
                        _ => proved(Vec::new(), 10),
                    })
                }
            },
            |range: &(u64, u64)| *range,
            |range: (u64, u64)| vec![range],
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(zone.rows, vec![(1, 5), (6, 9)]);
        assert_eq!(zone.spans, vec![(1, 5), (6, 9)]);
        assert_eq!(*requested.borrow(), vec![0, 10]);
    }

    #[tokio::test]
    async fn should_return_the_page_error() {
        let result = fetch_pages_from_genesis(
            &mut 0,
            2,
            |_start: BlockHeight| async move {
                Err::<ProvedPage<u64>, _>(PlatformWalletError::AddressSync("offline".to_string()))
            },
            |height: &u64| (*height, *height),
            |height: u64| vec![height],
        )
        .await;

        assert!(matches!(result, Err(PlatformWalletError::AddressSync(_))));
    }

    #[tokio::test]
    async fn should_ask_again_at_the_same_cursor_for_a_page_below_the_floor() {
        let requested = RefCell::new(Vec::new());
        let answers = Cell::new(0usize);
        let mut floor = 12;
        let zone = fetch_pages_from_genesis(
            &mut floor,
            2,
            |start: BlockHeight| {
                requested.borrow_mut().push(start);
                answers.set(answers.get() + 1);
                // The first answer comes from a node a block behind.
                let proved_height = if answers.get() == 1 { 11 } else { 13 };
                async move { Ok::<_, PlatformWalletError>(proved(vec![3u64], proved_height)) }
            },
            |height: &u64| (*height, *height),
            |height: u64| vec![height],
        )
        .await
        .expect("the second answer is at the floor");

        assert_eq!(zone.rows, vec![3]);
        assert_eq!(*requested.borrow(), vec![0, 0]);
        assert_eq!(floor, 13);
    }

    #[tokio::test]
    async fn should_fail_a_page_that_stays_below_the_floor() {
        let requested = Cell::new(0usize);
        let mut floor = 12;
        let result = fetch_pages_from_genesis(
            &mut floor,
            2,
            |_start: BlockHeight| {
                requested.set(requested.get() + 1);
                async move { Ok::<_, PlatformWalletError>(proved(vec![3u64], 11)) }
            },
            |height: &u64| (*height, *height),
            |height: u64| vec![height],
        )
        .await;

        assert!(matches!(
            result,
            Err(PlatformWalletError::BalanceHistoryInconsistentHeights {
                attempts: BALANCE_HISTORY_PAGE_ATTEMPTS,
                required_height: 12,
                proved_height: 11,
            })
        ));
        assert_eq!(requested.get(), BALANCE_HISTORY_PAGE_ATTEMPTS);
        assert_eq!(floor, 12, "a refused page never lowers the floor");
    }

    #[test]
    fn should_drop_recent_rows_inside_any_compacted_range_and_keep_the_rest() {
        let recent = vec![
            recent_row(4, 1),
            recent_row(5, 2),
            recent_row(9, 3),
            recent_row(10, 4),
            recent_row(13, 5),
            recent_row(20, 6),
        ];
        // (1, 4) and (5, 9) touch; (10, 12) is separate; 13 and 20 are past
        // every range.
        let spans = vec![(10, 12), (1, 4), (5, 9)];

        let kept = drop_recent_rows_covered_by_ranges(recent, spans);

        assert_eq!(kept, vec![recent_row(13, 5), recent_row(20, 6)]);
    }

    #[test]
    fn should_keep_every_recent_row_without_compacted_ranges() {
        let recent = vec![recent_row(4, 1), recent_row(10, 2)];

        let kept = drop_recent_rows_covered_by_ranges(recent.clone(), Vec::new());

        assert_eq!(kept, recent);
    }

    #[test]
    fn should_raise_the_floor_only_for_pages_at_or_above_it() {
        assert_eq!(raised_floor(12, 12), Some(12));
        assert_eq!(raised_floor(12, 13), Some(13));
        assert_eq!(raised_floor(12, 11), None);
    }

    #[tokio::test]
    async fn should_show_a_top_up_in_the_compaction_block_once_in_its_range() {
        let wallet = BTreeSet::from([address(1)]);
        // The recent read, proved at 11, holds blocks 10 and 11.
        let recent_pages = vec![(
            0,
            vec![
                recent_block(10, [(address(1), CreditOperation::AddToCredits(5))]),
                recent_block(11, [(address(2), CreditOperation::AddToCredits(9))]),
            ],
        )];
        // Block 12 triggered a compaction: Drive merged 10, 11 and 12's own
        // top-up of 7 into range (10, 12) and never wrote 12 to the recent
        // zone. The compacted read is proved at 12.
        let compacted_pages = vec![(
            0,
            vec![compacted_range(
                10,
                12,
                [
                    (
                        address(1),
                        BlockAwareCreditOperation::AddToCreditsOperations(BTreeMap::from([
                            (10, 5),
                            (12, 7),
                        ])),
                    ),
                    (
                        address(2),
                        BlockAwareCreditOperation::AddToCreditsOperations(BTreeMap::from([(
                            11, 9,
                        )])),
                    ),
                ],
            )],
        )];

        let (recent, compacted) = wallet_balance_history(
            &wallet,
            true,
            |start| {
                let page = page_at(&recent_pages, start, 11);
                async move { Ok(page) }
            },
            |start| {
                let page = page_at(&compacted_pages, start, 12);
                async move { Ok(page) }
            },
        )
        .await
        .expect("consistent fake reads");

        // Block 10 is covered by the range, so it shows only there.
        assert!(recent.is_empty());
        assert_eq!(
            compacted,
            vec![CompactedAddressBalanceChangeEntry {
                start_block_height: 10,
                end_block_height: 12,
                address: address(1),
                change: CompactedAddressBalanceChange::Additions(vec![(12, 7), (10, 5)]),
            }]
        );
    }

    #[tokio::test]
    async fn should_keep_a_range_below_the_recent_zone_and_recent_rows_outside_every_range() {
        let wallet = BTreeSet::from([address(1)]);
        let recent_pages = vec![(
            0,
            vec![
                recent_block(10, [(address(1), CreditOperation::AddToCredits(5))]),
                recent_block(12, [(address(1), CreditOperation::AddToCredits(6))]),
            ],
        )];
        let compacted_pages = vec![(
            0,
            vec![compacted_range(
                3,
                9,
                [(address(1), BlockAwareCreditOperation::SetCredits(40))],
            )],
        )];

        let (recent, compacted) = wallet_balance_history(
            &wallet,
            true,
            |start| {
                let page = page_at(&recent_pages, start, 12);
                async move { Ok(page) }
            },
            |start| {
                let page = page_at(&compacted_pages, start, 12);
                async move { Ok(page) }
            },
        )
        .await
        .expect("consistent fake reads");

        assert_eq!(recent, vec![recent_row(12, 6), recent_row(10, 5)]);
        assert_eq!(compacted, vec![compacted_row(3, 9, 40)]);
    }

    #[tokio::test]
    async fn should_restore_a_top_up_from_a_compacted_page_asked_for_again_at_the_floor() {
        let wallet = BTreeSet::from([address(1)]);
        let compacted_requests = RefCell::new(Vec::new());

        let (recent, compacted) = wallet_balance_history(
            &wallet,
            true,
            // Recent is proved at 12, after block 12's compaction moved the
            // top-up out of the recent zone.
            |start| {
                let page = page_at::<BlockAddressBalanceChanges>(&[], start, 12);
                async move { Ok(page) }
            },
            // Older first answers from a node at 11, before the range
            // existed; asked again at the same cursor, it answers at 12 with
            // the range holding the top-up.
            |start| {
                compacted_requests.borrow_mut().push(start);
                let page = if compacted_requests.borrow().len() == 1 {
                    proved(Vec::new(), 11)
                } else {
                    page_at(
                        &[(
                            0,
                            vec![compacted_range(
                                10,
                                12,
                                [(
                                    address(1),
                                    BlockAwareCreditOperation::AddToCreditsOperations(
                                        BTreeMap::from([(12, 7)]),
                                    ),
                                )],
                            )],
                        )],
                        start,
                        12,
                    )
                };
                async move { Ok(page) }
            },
        )
        .await
        .expect("the second compacted answer is at the floor");

        assert!(recent.is_empty());
        assert_eq!(
            compacted,
            vec![CompactedAddressBalanceChangeEntry {
                start_block_height: 10,
                end_block_height: 12,
                address: address(1),
                change: CompactedAddressBalanceChange::Additions(vec![(12, 7)]),
            }]
        );
        assert_eq!(*compacted_requests.borrow(), vec![0, 0]);
    }

    #[tokio::test]
    async fn should_hold_the_floor_instead_of_restarting_when_both_zones_lag() {
        let wallet = BTreeSet::from([address(1)]);
        let recent_requests = RefCell::new(Vec::new());
        let compacted_requests = Cell::new(0usize);

        let result = wallet_balance_history(
            &wallet,
            true,
            // Recent was seen at 12 with a top-up.
            |start| {
                recent_requests.borrow_mut().push(start);
                let page = page_at(
                    &[(
                        0,
                        vec![recent_block(
                            12,
                            [(address(1), CreditOperation::AddToCredits(6))],
                        )],
                    )],
                    start,
                    12,
                );
                async move { Ok(page) }
            },
            // Every compacted answer comes from a node at 11.
            |start| {
                compacted_requests.set(compacted_requests.get() + 1);
                let page = page_at::<CompactedBlockAddressBalanceChanges>(&[], start, 11);
                async move { Ok(page) }
            },
        )
        .await;

        assert!(matches!(
            result,
            Err(PlatformWalletError::BalanceHistoryInconsistentHeights {
                attempts: BALANCE_HISTORY_PAGE_ATTEMPTS,
                required_height: 12,
                proved_height: 11,
            })
        ));
        // The recent zone is read once: the scan is never restarted.
        assert_eq!(*recent_requests.borrow(), vec![0]);
        assert_eq!(compacted_requests.get(), BALANCE_HISTORY_PAGE_ATTEMPTS);
    }

    #[tokio::test]
    async fn should_check_the_height_of_an_empty_continuation_after_a_full_page() {
        let wallet = BTreeSet::from([address(1)]);
        let page_limit = BlockHeight::from(COMPACTED_ADDRESS_BALANCE_CHANGES_QUERY_LIMIT);
        // A full first page of single-block ranges of another wallet.
        let full_page: Vec<CompactedBlockAddressBalanceChanges> = (1..=page_limit)
            .map(|height| {
                compacted_range(
                    height,
                    height,
                    [(address(2), BlockAwareCreditOperation::SetCredits(1))],
                )
            })
            .collect();
        let continuation = page_limit + 1;
        let new_range = compacted_range(
            continuation,
            continuation + 4,
            [(address(1), BlockAwareCreditOperation::SetCredits(40))],
        );
        let compacted_requests = RefCell::new(Vec::new());

        let (_, compacted) = wallet_balance_history(
            &wallet,
            true,
            |start| {
                let page = page_at::<BlockAddressBalanceChanges>(&[], start, 100);
                async move { Ok(page) }
            },
            |start| {
                compacted_requests.borrow_mut().push(start);
                let page = if start == 0 {
                    proved(full_page.clone(), 100)
                } else if compacted_requests.borrow().len() == 2 {
                    // An empty continuation from a node a block behind would
                    // hide the range created at 100.
                    proved(Vec::new(), 99)
                } else {
                    proved(vec![new_range.clone()], 100)
                };
                async move { Ok(page) }
            },
        )
        .await
        .expect("the continuation asked again is at the floor");

        assert_eq!(
            compacted,
            vec![compacted_row(continuation, continuation + 4, 40)]
        );
        assert_eq!(
            *compacted_requests.borrow(),
            vec![0, continuation, continuation]
        );
    }

    #[tokio::test]
    async fn should_end_the_compacted_scan_on_a_short_page_at_the_floor() {
        let wallet = BTreeSet::from([address(1)]);
        let compacted_requests = RefCell::new(Vec::new());

        let (_, compacted) = wallet_balance_history(
            &wallet,
            true,
            |start| {
                let page = page_at::<BlockAddressBalanceChanges>(&[], start, 12);
                async move { Ok(page) }
            },
            |start| {
                compacted_requests.borrow_mut().push(start);
                let page = page_at(
                    &[(
                        0,
                        vec![compacted_range(
                            3,
                            9,
                            [(address(1), BlockAwareCreditOperation::SetCredits(40))],
                        )],
                    )],
                    start,
                    12,
                );
                async move { Ok(page) }
            },
        )
        .await
        .expect("consistent fake reads");

        assert_eq!(compacted, vec![compacted_row(3, 9, 40)]);
        assert_eq!(*compacted_requests.borrow(), vec![0]);
    }

    /// A platform address wallet whose only address is `address(byte)` (a
    /// hydrated balance on platform payment account 0), on `sdk`.
    fn address_wallet_with_address(sdk: dash_sdk::Sdk, byte: u8) -> PlatformAddressWallet {
        let (mut wm, wallet_id) = wallet_manager_with_empty_wallet();
        {
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            let mut account = ManagedPlatformAccount::new(0, 0, empty_pool(0), false);
            account.set_address_credit_balance(PlatformP2PKHAddress::new([byte; 20]), 1_000, None);
            info.core_wallet.accounts.insert_platform_account(account);
        }
        platform_address_wallet_in(Arc::new(sdk), Arc::new(RwLock::new(wm)), wallet_id)
    }

    /// A mock SDK answering the first page of each zone: recent with a
    /// top-up of `address(byte)` at block 12 proved at `recent_height`, and
    /// compacted with a range (3, 9) setting it to 40 proved at
    /// `compacted_height`. Both pages are short, so nothing else is asked.
    async fn mock_sdk_with_both_zones(
        byte: u8,
        recent_height: BlockHeight,
        compacted_height: BlockHeight,
    ) -> dash_sdk::Sdk {
        let mut sdk = dash_sdk::Sdk::new_mock();
        sdk.mock()
            .expect_fetch(
                RecentAddressBalanceChangesQuery::new(0),
                Some(RecentAddressBalanceChanges(vec![recent_block(
                    12,
                    [(address(byte), CreditOperation::AddToCredits(500))],
                )])),
            )
            .await
            .expect("register the recent page");
        sdk.mock()
            .expect_fetch(
                RecentCompactedAddressBalanceChangesQuery::new(0),
                Some(RecentCompactedAddressBalanceChanges(vec![compacted_range(
                    3,
                    9,
                    [(address(byte), BlockAwareCreditOperation::SetCredits(40))],
                )])),
            )
            .await
            .expect("register the compacted page");
        sdk.mock()
            .set_response_height_for::<RecentAddressBalanceChanges, _>(
                RecentAddressBalanceChangesQuery::new(0),
                recent_height,
            )
            .set_response_height_for::<RecentCompactedAddressBalanceChanges, _>(
                RecentCompactedAddressBalanceChangesQuery::new(0),
                compacted_height,
            );
        sdk
    }

    #[tokio::test]
    async fn should_read_both_zones_through_the_sdk_with_their_proved_heights() {
        let byte = 0x42;
        let sdk = mock_sdk_with_both_zones(byte, 12, 13).await;
        let wallet = address_wallet_with_address(sdk, byte);

        let history = wallet
            .balance_history(true)
            .await
            .expect("the compacted page is proved above the recent one");

        assert_eq!(
            history.recent,
            vec![AddressBalanceChangeEntry {
                block_height: 12,
                address: address(byte),
                change: AddressBalanceChange::Added(500),
            }]
        );
        assert_eq!(
            history.compacted,
            vec![CompactedAddressBalanceChangeEntry {
                start_block_height: 3,
                end_block_height: 9,
                address: address(byte),
                change: CompactedAddressBalanceChange::SetFinal(40),
            }]
        );
    }

    #[tokio::test]
    async fn should_refuse_through_the_sdk_a_compacted_page_proved_below_the_recent_one() {
        let byte = 0x42;
        let sdk = mock_sdk_with_both_zones(byte, 12, 11).await;
        let wallet = address_wallet_with_address(sdk, byte);

        let result = wallet.balance_history(true).await;

        assert!(
            matches!(
                result,
                Err(PlatformWalletError::BalanceHistoryInconsistentHeights {
                    attempts: BALANCE_HISTORY_PAGE_ATTEMPTS,
                    required_height: 12,
                    proved_height: 11,
                })
            ),
            "{result:?}"
        );
    }

    #[tokio::test]
    async fn should_cover_recent_rows_by_every_range_including_ranges_without_wallet_rows() {
        let wallet = BTreeSet::from([address(1)]);
        let recent_pages = vec![(
            0,
            vec![recent_block(
                12,
                [(address(1), CreditOperation::AddToCredits(6))],
            )],
        )];
        // The covering range holds no change of this wallet: coverage is
        // taken from every range returned, not from the wallet's rows.
        let compacted_pages = vec![(
            0,
            vec![compacted_range(
                10,
                12,
                [(address(2), BlockAwareCreditOperation::SetCredits(3))],
            )],
        )];

        let (recent, compacted) = wallet_balance_history(
            &wallet,
            true,
            |start| {
                let page = page_at(&recent_pages, start, 12);
                async move { Ok(page) }
            },
            |start| {
                let page = page_at(&compacted_pages, start, 12);
                async move { Ok(page) }
            },
        )
        .await
        .expect("consistent fake reads");

        assert!(recent.is_empty());
        assert!(compacted.is_empty());
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
                async move { Ok(proved(Vec::new(), 10)) }
            },
            |start| {
                calls.borrow_mut().push(("compacted", start));
                async move { Ok(proved(Vec::new(), 10)) }
            },
        )
        .await
        .expect("fake pages never fail");

        assert_eq!(*calls.borrow(), vec![("recent", 0), ("compacted", 0)]);
    }

    #[tokio::test]
    async fn should_not_fetch_the_compacted_zone_unless_asked() {
        let wallet = BTreeSet::from([address(1)]);
        let compacted_fetched = Cell::new(false);
        let recent_pages = vec![(
            0,
            vec![recent_block(
                12,
                [(address(1), CreditOperation::AddToCredits(6))],
            )],
        )];

        let (recent, compacted) = wallet_balance_history(
            &wallet,
            false,
            |start| {
                let page = page_at(&recent_pages, start, 12);
                async move { Ok(page) }
            },
            |_start| {
                compacted_fetched.set(true);
                async move { Ok(proved(Vec::new(), 12)) }
            },
        )
        .await
        .expect("fake pages never fail");

        assert!(!compacted_fetched.get());
        assert_eq!(recent, vec![recent_row(12, 6)]);
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
