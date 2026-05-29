//! Platform-local "Reserved" bridge for HD address hand-out.
//!
//! Serves every pool whose hand-out suffers the two-state race: the
//! DIP-17 platform-payment receive pool and the core BIP-44 external
//! (receive) and internal (change) pools. Each pool's index space is
//! kept disjoint by the [`PoolKind`] discriminant in the table key.
//!
//! # Why this module exists
//!
//! The upstream `key-wallet` [`AddressPool`] models only a two-state
//! lifecycle per derivation index: *unused* → *used*. The `used` flag
//! flips only when a positive-balance sync proves the address received
//! funds, so [`AddressPool::next_unused`] returns the first index whose
//! `used == false`.
//!
//! That two-state model has a hand-out race: two concurrent callers each
//! ask for "the next unused receive address", both observe the same
//! index as `used == false` (no payment has landed yet, and one won't
//! until the user actually receives funds), and both are handed the
//! **same** address.
//!
//! The correct fix is a tri-state lifecycle *unused* → *reserved* →
//! *used* inside the pool itself. That belongs upstream in `key-wallet`.
//! Until it lands, this module supplies the missing *reserved* layer as
//! a thin, platform-local side table that sits on top of the existing
//! pool, consulted and updated atomically while the caller holds the
//! wallet write lock.
//!
//! # BRIDGE
//!
//! This whole module is a deliberate stopgap. The proper home for the
//! Reserved state is inside `key-wallet`'s `AddressPool` upstream —
//! tracked at <https://github.com/dashpay/rust-dashcore/issues/791>.
//! When `key-wallet` gains a native `Reserved` state, replace the body of
//! [`next_unused_and_reserve`] with a single delegation:
//!
//! ```ignore
//! pool.next_unused_and_reserve(key_source, add_to_state)
//! ```
//!
//! The signature here is intentionally identical in shape to that future
//! upstream method, so the swap is a one-liner and **no call site
//! changes**. The only extra parameters — `wallet_id` and
//! `account_index` — exist solely to key the platform-side reservation
//! table; upstream would not need them because the pool would own its
//! reserved set internally.
//!
//! # Ephemerality
//!
//! The reserved set is **never persisted**. It lives in a process-global
//! [`Mutex`]-guarded table and is rebuilt empty on every process start.
//! A reserved-but-never-paid index must free itself on restart rather
//! than pin gap-limit headroom forever, so the in-memory-only choice is
//! deliberate. It does not appear in any serde/bincode form.
//!
//! # Atomicity
//!
//! [`next_unused_receive_address`](super::wallet::PlatformAddressWallet::next_unused_receive_address)
//! holds `wallet_manager.write().await` across the entire pick-and-
//! reserve. The reservation table additionally guards its own state with
//! a [`Mutex`], so the on-use clear path ([`release_reservation`]) — run
//! under the same wallet write lock from the balance-sync callback —
//! never tears against an in-flight reserve. Pick-and-reserve is a single
//! critical section: there is no TOCTOU gap between "find unused" and
//! "mark reserved".

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use key_wallet::error::Error as KeyWalletError;
use key_wallet::managed_account::address_pool::{AddressInfo, AddressPool, KeySource};
use key_wallet::Address;

use crate::wallet::platform_wallet::WalletId;

/// Which HD pool a reservation belongs to. A single `(wallet_id, account)`
/// owns several independent index spaces — the DIP-17 platform-payment
/// pool, plus the BIP-44 external (receive) and internal (change) pools —
/// and an index reserved in one must never steer hand-out in another.
/// Including the pool kind in the table key keeps those spaces disjoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PoolKind {
    /// DIP-17 platform-payment receive pool.
    PlatformReceive,
    /// BIP-44 external (receive) pool.
    CoreReceive,
    /// BIP-44 internal (change) pool.
    CoreChange,
}

/// Account-scoped key for the reservation table: a slot is identified by
/// which wallet, which pool, and which account it belongs to.
type AccountKey = (WalletId, PoolKind, u32);

/// One reserved index plus the instant it was reserved, so the TTL sweep
/// can release stale entries without a separate timestamp map.
#[derive(Debug, Clone, Copy)]
struct ReservedAt {
    reserved_at: Instant,
}

/// Process-global reservation table. Keyed by `(wallet_id, account)`,
/// each entry maps a reserved derivation index to when it was reserved.
/// Guarded by a single [`Mutex`] so reserve and release commit
/// atomically.
///
/// Owned as a `static` rather than a struct field on purpose: it must be
/// reachable from both the hand-out path and the balance-sync clear path
/// without threading a new type through either, and it must be in-memory
/// only. A `static` is the platform-local stand-in for "state the pool
/// will own once upstream gains the `Reserved` lifecycle".
#[derive(Default)]
struct ReservationTable {
    by_account: HashMap<AccountKey, HashMap<u32, ReservedAt>>,
}

// TODO(upstream): this process-global reserved table is a deliberate
// BRIDGE/stopgap. The proper home for the Reserved tri-state
// (Unused → Reserved → Used) is inside `key-wallet`'s `AddressPool`
// (rust-dashcore). Remove this static — and collapse
// `next_unused_and_reserve` to a one-line delegation — once upstream
// lands native support. Tracked at:
// https://github.com/dashpay/rust-dashcore/issues/791
fn table() -> &'static Mutex<ReservationTable> {
    static TABLE: OnceLock<Mutex<ReservationTable>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(ReservationTable::default()))
}

fn with_table<R>(f: impl FnOnce(&mut ReservationTable) -> R) -> R {
    let mut guard = table().lock().unwrap_or_else(|p| p.into_inner());
    f(&mut guard)
}

/// First index that is neither `used` in the pool nor present in
/// `reserved`. Mirrors the `0..` scan in [`AddressPool::next_unused`] but
/// additionally skips reserved-but-unused indices, so two concurrent
/// callers can never be handed the same slot.
///
/// Pure over its inputs — the unit tests exercise it directly without a
/// live pool or key derivation. Walks the contiguous `0..` index space
/// the pool uses, returning the first index that is absent or unused, and
/// in either case not reserved.
fn first_unreserved_unused_index(
    addresses: &BTreeMap<u32, AddressInfo>,
    reserved: &HashSet<u32>,
) -> u32 {
    let mut index = 0u32;
    loop {
        match addresses.get(&index) {
            Some(info) if info.used => index += 1,
            _ if reserved.contains(&index) => index += 1,
            // Absent or present-and-unused, and not reserved: this is it.
            _ => return index,
        }
    }
}

/// The atomic pick-and-reserve critical section: under the table lock,
/// pick the first index that is neither `used` (in `addresses`) nor
/// already reserved for `key`, insert it into the reserved set, and
/// return it. Because the whole operation runs inside one `with_table`
/// acquisition there is no TOCTOU window — two concurrent callers can
/// never observe the same index as free.
fn reserve_first_free(key: AccountKey, addresses: &BTreeMap<u32, AddressInfo>) -> u32 {
    with_table(|t| {
        let account = t.by_account.entry(key).or_default();
        let reserved: HashSet<u32> = account.keys().copied().collect();
        let index = first_unreserved_unused_index(addresses, &reserved);
        account.insert(
            index,
            ReservedAt {
                reserved_at: Instant::now(),
            },
        );
        index
    })
}

/// Hand out the next receive address that is neither used nor already
/// reserved, atomically marking it reserved before returning.
///
/// BRIDGE: mirrors the intended upstream `AddressPool::next_unused_and_reserve`.
/// When `key-wallet` gains a native `Reserved` state, replace this body
/// with `pool.next_unused_and_reserve(key_source, add_to_state)` — the
/// signature is intentionally identical apart from the `wallet_id` /
/// `account_index` reservation key, which the upstream pool would not
/// need because it would own its reserved set internally.
///
/// Atomic with respect to the pool because the caller holds the wallet
/// write lock; atomic with respect to the clear path because the
/// reservation table is [`Mutex`]-guarded.
///
/// The chosen index is materialized into `pool.addresses` whenever
/// `add_to_state` is set, which is what makes the reserved index count
/// toward the gap-limit scan window: every reserved slot becomes a real
/// pending address the BLAST sync covers.
///
/// Materialization uses only the pool's public API. The picked index is
/// always either already materialized (return it via
/// [`AddressPool::address_at_index`]) or exactly `highest_generated + 1`
/// (the contiguous next slot), so a single
/// [`AddressPool::generate_addresses`]`(1, ..)` — which generates from
/// `highest_generated + 1` — materializes precisely that index. This
/// mirrors what the future upstream `next_unused_and_reserve` would do
/// internally; the swap stays a one-liner.
pub(crate) fn next_unused_and_reserve(
    pool: &mut AddressPool,
    wallet_id: WalletId,
    pool_kind: PoolKind,
    account_index: u32,
    key_source: &KeySource,
    add_to_state: bool,
) -> Result<Address, KeyWalletError> {
    let index = reserve_first_free((wallet_id, pool_kind, account_index), &pool.addresses);

    let result = match pool.address_at_index(index) {
        Some(address) => Ok(address),
        None if !add_to_state => {
            // Caller wants the address without mutating pool state; derive
            // it through the pool's discovery path and take the matching
            // index. `next_unused` returns the first unused address, which
            // — given our reservation steered the pick — is this index.
            pool.next_unused(key_source, false)
        }
        None => match pool.generate_addresses(1, key_source, add_to_state) {
            Ok(mut addrs) => addrs.pop().ok_or(KeyWalletError::NoKeySource),
            Err(e) => Err(e),
        },
    };

    if result.is_err() {
        // Derivation failed — give the index back so it isn't pinned by a
        // hand-out that produced nothing.
        release_reservation(wallet_id, pool_kind, account_index, index);
    }
    result
}

/// Release a single reservation, called from the balance-sync
/// `on_address_found` path once an address is proven used. Idempotent —
/// releasing an index that was never reserved (or already released) is a
/// no-op.
pub(crate) fn release_reservation(
    wallet_id: WalletId,
    pool_kind: PoolKind,
    account_index: u32,
    index: u32,
) {
    let key = (wallet_id, pool_kind, account_index);
    with_table(|t| {
        if let Some(m) = t.by_account.get_mut(&key) {
            m.remove(&index);
            if m.is_empty() {
                t.by_account.remove(&key);
            }
        }
    });
}

/// Highest currently-reserved index for an account, or `None` if the
/// account holds no reservations. Used by tests asserting that the
/// reserved frontier advances on hand-out while the pool's `used`
/// frontier does not advance until a sync hit.
#[cfg(test)]
pub(crate) fn highest_reserved(
    wallet_id: WalletId,
    pool_kind: PoolKind,
    account_index: u32,
) -> Option<u32> {
    with_table(|t| {
        t.by_account
            .get(&(wallet_id, pool_kind, account_index))
            .and_then(|m| m.keys().copied().max())
    })
}

/// Release every reservation older than `ttl`, returning the number of
/// indices reclaimed. A long-lived process calls this periodically so a
/// caller that reserved an address but never received a payment (the user
/// closed the screen, the request timed out) eventually frees the slot
/// instead of leaking it for the process lifetime.
pub(crate) fn sweep_expired(ttl: Duration) -> usize {
    let now = Instant::now();
    with_table(|t| {
        let mut reclaimed = 0;
        t.by_account.retain(|_, m| {
            m.retain(|_, r| {
                let keep = now.saturating_duration_since(r.reserved_at) < ttl;
                if !keep {
                    reclaimed += 1;
                }
                keep
            });
            !m.is_empty()
        });
        reclaimed
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Each test runs against a distinct `(wallet_id, account)` key so
    /// the process-global table never aliases across tests, even under
    /// the default multi-threaded test runner. A monotonic counter in
    /// the high bytes of the wallet id guarantees uniqueness.
    fn unique_account() -> AccountKey {
        static NEXT: AtomicU32 = AtomicU32::new(1);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut wid = [0u8; 32];
        wid[..4].copy_from_slice(&n.to_be_bytes());
        (wid, PoolKind::PlatformReceive, 0)
    }

    fn info(index: u32, used: bool) -> AddressInfo {
        // The bridge selection logic never inspects the address bytes,
        // only the `used` flag, so one fixed valid P2PKH address serves
        // every index. Using a constant key (rather than deriving one per
        // index) avoids both the per-index secp cost and the all-zero
        // invalid-key edge at index 255, and keeps the helper independent
        // of the large `AddressInfo` field set via its public constructor.
        use dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
        use key_wallet::bip32::DerivationPath;
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[7u8; 32]).expect("sk");
        let pk = PublicKey::from_secret_key(&secp, &sk);
        let cpk = dashcore::PublicKey::new(pk);
        let address = Address::p2pkh(&cpk, key_wallet::Network::Testnet);
        let mut info = AddressInfo::new_from_script_pubkey_p2pkh(
            address.script_pubkey(),
            index,
            DerivationPath::default(),
            key_wallet::Network::Testnet,
        )
        .expect("p2pkh info");
        info.used = used;
        info
    }

    fn pool_map(entries: &[(u32, bool)]) -> BTreeMap<u32, AddressInfo> {
        entries
            .iter()
            .map(|&(i, used)| (i, info(i, used)))
            .collect()
    }

    // ----- pure selection logic -----

    #[test]
    fn picks_first_unused_when_nothing_reserved() {
        let addrs = pool_map(&[(0, true), (1, true), (2, false), (3, false)]);
        let reserved = HashSet::new();
        assert_eq!(first_unreserved_unused_index(&addrs, &reserved), 2);
    }

    #[test]
    fn skips_reserved_index() {
        let addrs = pool_map(&[(0, true), (1, false), (2, false)]);
        let reserved: HashSet<u32> = [1].into_iter().collect();
        assert_eq!(first_unreserved_unused_index(&addrs, &reserved), 2);
    }

    #[test]
    fn skips_both_used_and_reserved() {
        let addrs = pool_map(&[(0, true), (1, false), (2, false), (3, false)]);
        let reserved: HashSet<u32> = [1, 2].into_iter().collect();
        assert_eq!(first_unreserved_unused_index(&addrs, &reserved), 3);
    }

    #[test]
    fn walks_past_reserved_tail_beyond_materialized_map() {
        // Three materialized addresses, all used; indices 3 and 4 already
        // reserved past the end. Next fresh slot is 5.
        let addrs = pool_map(&[(0, true), (1, true), (2, true)]);
        let reserved: HashSet<u32> = [3, 4].into_iter().collect();
        assert_eq!(first_unreserved_unused_index(&addrs, &reserved), 5);
    }

    /// Persisted-form invariant: the first hand-out from a fresh account
    /// picks exactly the index the upstream two-state pool would have, so
    /// the address materialized into `pool.addresses` — the only thing
    /// that reaches the serialized wallet — is bit-identical to the
    /// pre-bridge `next_unused(.., true)` path. The reservation layer
    /// changes *subsequent concurrent* hand-outs, never the serialized
    /// pool for a single first call.
    #[test]
    fn first_handout_matches_upstream_next_unused_scan() {
        let addrs = pool_map(&[(0, true), (1, true), (2, false), (3, false)]);
        // upstream `next_unused` walks 0.. for the first non-used index.
        let upstream = {
            let mut i = 0u32;
            while matches!(addrs.get(&i), Some(info) if info.used) {
                i += 1;
            }
            i
        };
        let key = unique_account();
        assert_eq!(reserve_first_free(key, &addrs), upstream);
        assert_eq!(upstream, 2);
    }

    // ----- Found-026 adapted: reserve via the real critical section -----

    /// Back-to-back hand-outs against a fixed pool return distinct
    /// indices, because the first reservation makes the second skip it.
    #[test]
    fn back_to_back_handouts_are_distinct() {
        let key = unique_account();
        let addrs = pool_map(&[(0, false), (1, false), (2, false)]);
        let a = reserve_first_free(key, &addrs);
        let b = reserve_first_free(key, &addrs);
        let c = reserve_first_free(key, &addrs);
        assert_eq!(
            (a, b, c),
            (0, 1, 2),
            "each hand-out skips the prior reservation"
        );
    }

    /// An index already reserved is skipped even though the pool still
    /// reports it `used == false`.
    #[test]
    fn reserved_index_is_skipped_while_pool_reports_unused() {
        let key = unique_account();
        let addrs = pool_map(&[(0, false), (1, false)]);
        let first = reserve_first_free(key, &addrs);
        assert_eq!(first, 0);
        // Pool is untouched (no sync hit) so index 0 is still unused, yet
        // the reservation must steer the next hand-out to index 1.
        assert!(!addrs[&0].used);
        assert_eq!(reserve_first_free(key, &addrs), 1);
    }

    /// On confirmed use, releasing the reservation clears it; the pool's
    /// own `used` flag (flipped by the sync) is what then keeps the index
    /// out of future hand-outs.
    #[test]
    fn release_on_use_clears_reservation() {
        let (wid, pk, acct) = unique_account();
        let mut addrs = pool_map(&[(0, false), (1, false)]);
        let idx = reserve_first_free((wid, pk, acct), &addrs);
        assert_eq!(idx, 0);
        assert_eq!(highest_reserved(wid, pk, acct), Some(0));

        // Sync proves index 0 used: provider flips the flag and releases.
        addrs.get_mut(&0).unwrap().used = true;
        release_reservation(wid, pk, acct, 0);
        assert_eq!(highest_reserved(wid, pk, acct), None);

        // Next hand-out skips the now-used 0 via the pool flag, lands on 1.
        assert_eq!(reserve_first_free((wid, pk, acct), &addrs), 1);
    }

    /// Reservations in different pools of the SAME `(wallet, account)` are
    /// independent: reserving index 0 in `CoreReceive` must not steer the
    /// first `CoreChange` hand-out off index 0. Guards the `PoolKind`
    /// discriminant in the table key.
    #[test]
    fn distinct_pool_kinds_do_not_collide() {
        let (wid, _pk, acct) = unique_account();
        let addrs = pool_map(&[(0, false), (1, false)]);

        let recv = reserve_first_free((wid, PoolKind::CoreReceive, acct), &addrs);
        let change = reserve_first_free((wid, PoolKind::CoreChange, acct), &addrs);
        let platform = reserve_first_free((wid, PoolKind::PlatformReceive, acct), &addrs);

        // Each pool reserves from its own empty index space, so all three
        // land on index 0 despite sharing the wallet and account.
        assert_eq!((recv, change, platform), (0, 0, 0));
        assert_eq!(highest_reserved(wid, PoolKind::CoreReceive, acct), Some(0));
        assert_eq!(highest_reserved(wid, PoolKind::CoreChange, acct), Some(0));
        assert_eq!(
            highest_reserved(wid, PoolKind::PlatformReceive, acct),
            Some(0)
        );

        // A second CoreReceive hand-out skips its own index 0 but the other
        // pools are untouched.
        assert_eq!(
            reserve_first_free((wid, PoolKind::CoreReceive, acct), &addrs),
            1
        );
        assert_eq!(highest_reserved(wid, PoolKind::CoreChange, acct), Some(0));
    }

    /// Behavioral shift vs the old `mark_index_used` idiom: hand-out
    /// advances the *reserved* frontier, while the pool's *used* frontier
    /// does NOT advance until a sync hit flips `used`.
    #[test]
    fn handout_advances_reserved_not_used() {
        let (wid, pk, acct) = unique_account();
        let addrs = pool_map(&[(0, false), (1, false), (2, false)]);

        assert_eq!(highest_reserved(wid, pk, acct), None);
        reserve_first_free((wid, pk, acct), &addrs);
        assert_eq!(highest_reserved(wid, pk, acct), Some(0));
        reserve_first_free((wid, pk, acct), &addrs);
        assert_eq!(highest_reserved(wid, pk, acct), Some(1));

        // No sync ran, so every pool entry is still unused — the `used`
        // frontier has not moved at all.
        assert!(addrs.values().all(|i| !i.used));
    }

    // ----- reclaim / TTL -----

    #[test]
    fn sweep_expired_releases_old_reservations() {
        let (wid, pk, acct) = unique_account();
        let addrs = pool_map(&[(0, false), (1, false)]);
        reserve_first_free((wid, pk, acct), &addrs);
        reserve_first_free((wid, pk, acct), &addrs);
        assert_eq!(highest_reserved(wid, pk, acct), Some(1));

        std::thread::sleep(Duration::from_millis(2));
        let reclaimed = sweep_expired(Duration::from_millis(1));
        assert!(
            reclaimed >= 2,
            "both reservations reclaimed, got {reclaimed}"
        );
        assert_eq!(highest_reserved(wid, pk, acct), None);
    }

    #[test]
    fn sweep_keeps_fresh_reservations() {
        let (wid, pk, acct) = unique_account();
        let addrs = pool_map(&[(0, false)]);
        reserve_first_free((wid, pk, acct), &addrs);
        // Huge TTL — nothing is stale.
        let reclaimed = sweep_expired(Duration::from_secs(3600));
        assert_eq!(reclaimed, 0);
        assert_eq!(highest_reserved(wid, pk, acct), Some(0));
    }

    // ----- mandatory concurrency stress test -----

    /// `STRESS_TASKS` concurrent tasks each reserve one index against a
    /// single shared account. The atomic critical section must hand every
    /// task a distinct index: zero duplicates, count == task count.
    const STRESS_TASKS: u32 = 1_000;

    fn run_stress(tasks: u32) {
        use std::sync::Arc;
        let key = unique_account();
        // All indices start unused; the reservation table alone must
        // serialize the hand-outs.
        let addrs: Arc<BTreeMap<u32, AddressInfo>> =
            Arc::new((0..tasks).map(|i| (i, info(i, false))).collect());

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(8)
            .build()
            .expect("runtime");

        let handed_out = rt.block_on(async move {
            let mut handles = Vec::with_capacity(tasks as usize);
            for _ in 0..tasks {
                let addrs = Arc::clone(&addrs);
                handles.push(tokio::spawn(async move { reserve_first_free(key, &addrs) }));
            }
            let mut out = Vec::with_capacity(tasks as usize);
            for h in handles {
                out.push(h.await.expect("task"));
            }
            out
        });

        let distinct: HashSet<u32> = handed_out.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            tasks as usize,
            "duplicates handed out: {} distinct of {} tasks",
            distinct.len(),
            tasks
        );
        assert_eq!(handed_out.len(), tasks as usize);
    }

    /// Always-on variant — proves the atomic reserve under parallel load
    /// in CI without the cost of the full-scale run.
    #[test]
    fn concurrent_reserve_no_duplicates() {
        run_stress(STRESS_TASKS);
    }

    /// Full-scale 10_000-task variant. Gated behind `#[ignore]` so it
    /// doesn't slow CI; run with `cargo test -- --ignored`.
    #[test]
    #[ignore = "heavy: 10k concurrent tasks; run explicitly with --ignored"]
    fn concurrent_reserve_no_duplicates_10k() {
        run_stress(10_000);
    }
}
