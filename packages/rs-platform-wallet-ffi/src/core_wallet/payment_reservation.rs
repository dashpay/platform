//! ABI-side handle table for the raw-payment path's key-wallet reservation
//! tokens.
//!
//! # Why a table and not just the token
//!
//! [`core_wallet_release_payment_reservation`](super::core_wallet_release_payment_reservation)
//! must be *owner-guarded*: it may only free inputs the abandoned build still
//! owns, or it can free a reservation key-wallet's TTL swept and a concurrent
//! build re-took — a double-spend window (dashpay/platform#4247 review).
//! The proof of ownership is key-wallet's `ReservationToken`, and that type
//! **cannot cross the C ABI**: it is deliberately opaque, with a private counter
//! and no public constructor, precisely so a caller cannot forge one and release
//! another build's inputs. Serializing it to a `u64` and rebuilding it on the way
//! back in is therefore not available — and would defeat the point if it were.
//!
//! So the token stays in Rust and an opaque, table-allocated
//! [`PaymentReservationHandle`] crosses the boundary in its place. This is the
//! same indirection `SignedPaymentRegistry` uses for the deferred-broadcast path
//! (its `ReservationToken` payment handle is a `u64` over the ABI while the
//! key-wallet funding token it guards stays Rust-side); the two token spaces are
//! deliberately separate types so a handle from one can never be presented to the
//! other.
//!
//! Forgery-resistance is preserved by construction: an unknown handle resolves to
//! nothing, so a fabricated `u64` releases nothing rather than acting on some
//! other build's token.
//!
//! # Lifetime and bounding
//!
//! Entries are **not** consumed by a release. Two reasons: the release stays
//! idempotent (a second release re-resolves the same token, which by then owns
//! nothing — a no-op inside key-wallet rather than an "unknown handle" error),
//! and the happy path never releases at all. That second point is what forces a
//! bound: this primitive does not broadcast — dashj does — so the SDK is never
//! told that a build went out on the wire, and a build → broadcast payment simply
//! leaves its entry behind.
//!
//! The table is therefore a fixed-capacity FIFO ring of [`MAX_ENTRIES`]: minting
//! entry N evicts entry N − [`MAX_ENTRIES`]. Reaching eviction takes
//! [`MAX_ENTRIES`] builds that were never released, i.e. builds that were
//! broadcast — whose reservations are backed by genuinely spent coins and whose
//! handles are of no further use. A build that is going to be abandoned is
//! abandoned within seconds of being made, so eviction cannot realistically
//! overtake one.
//!
//! An evicted or unknown handle is **refused**, never silently downgraded to the
//! unguarded release. Downgrading is exactly the behavior this change removes.
//!
//! In-memory only, like every reservation in this stack: a crash drops the table
//! and the underlying `ReservationSet` together, so nothing leaks across a
//! restart.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use key_wallet::ReservationToken as FundingReservationToken;
use once_cell::sync::Lazy;

/// Capacity of the FIFO ring. Sized so that reaching eviction requires thousands
/// of un-released builds in one process lifetime — several orders of magnitude
/// past any real send volume between app launches — while costing a fixed few
/// tens of kilobytes.
const MAX_ENTRIES: usize = 4096;

/// Opaque, process-unique handle standing in for a key-wallet
/// [`FundingReservationToken`] across the C ABI.
///
/// `0` is reserved and never minted: it is the "this build reserved nothing"
/// sentinel, matching the FFI's null-handle convention. A distinct newtype rather
/// than a bare `u64` so it can never be confused with the deferred path's
/// `platform_wallet::ReservationToken` payment handle, which is a different token
/// space entirely.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PaymentReservationHandle(u64);

impl PaymentReservationHandle {
    /// The sentinel meaning "the build took no reservation".
    pub(crate) const NONE: Self = Self(0);

    /// The raw wire value handed across the ABI.
    pub(crate) const fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for PaymentReservationHandle {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

/// A handle the table does not (or no longer) know: forged, already evicted, or
/// minted by a previous process. Releasing against it is refused.
#[derive(Debug)]
pub(crate) struct UnknownReservationHandle(pub(crate) PaymentReservationHandle);

/// The interior state: the live tokens plus the insertion order that drives
/// eviction.
struct Entries<T> {
    tokens: HashMap<PaymentReservationHandle, T>,
    /// Handles in mint order. The front is the oldest and is evicted first.
    order: VecDeque<PaymentReservationHandle>,
}

/// Bounded FIFO table of the reservation tokens currently addressable from the
/// host.
///
/// Generic over the stored token purely so the bounding/eviction/forgery logic
/// can be unit-tested: a real [`FundingReservationToken`] has no public
/// constructor (by design), so a test cannot fabricate one. The production
/// instantiation is [`PAYMENT_RESERVATIONS`].
pub(crate) struct PaymentReservationTable<T> {
    next_handle: AtomicU64,
    entries: Mutex<Entries<T>>,
}

impl<T: Copy> PaymentReservationTable<T> {
    /// A fresh, empty table. Not `const` — `HashMap::new` is not a const fn — so
    /// the process-global below is a `Lazy` rather than a plain `static`, exactly
    /// like `SIGNED_PAYMENT_REGISTRY`.
    pub(crate) fn new() -> Self {
        Self {
            // Start at 1 so `PaymentReservationHandle::NONE` is never minted.
            next_handle: AtomicU64::new(1),
            entries: Mutex::new(Entries {
                tokens: HashMap::new(),
                order: VecDeque::new(),
            }),
        }
    }

    /// Lock the table, recovering from a poisoned mutex rather than panicking.
    /// This is one process-global, so propagating a poison would permanently
    /// disable the abandon path for every wallet; the guarded state is a map plus
    /// a queue with no invariant a partial write could break. Mirrors
    /// `SignedPaymentRegistry::lock` and key-wallet's `ReservationSet::lock`.
    fn lock(&self) -> MutexGuard<'_, Entries<T>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Record `token` and return the handle that addresses it, evicting the
    /// oldest entry if the table is full. `None` mints nothing and yields
    /// [`PaymentReservationHandle::NONE`].
    pub(crate) fn stash(&self, token: Option<T>) -> PaymentReservationHandle {
        let Some(token) = token else {
            return PaymentReservationHandle::NONE;
        };
        let handle = PaymentReservationHandle(self.next_handle.fetch_add(1, Ordering::SeqCst));

        let mut entries = self.lock();
        entries.tokens.insert(handle, token);
        entries.order.push_back(handle);
        while entries.order.len() > MAX_ENTRIES {
            if let Some(oldest) = entries.order.pop_front() {
                entries.tokens.remove(&oldest);
            }
        }
        handle
    }

    /// Resolve `handle` to the token it addresses, WITHOUT consuming it — see the
    /// module docs on why the release must stay idempotent.
    ///
    /// [`PaymentReservationHandle::NONE`] resolves to `Ok(None)`: the build took
    /// no reservation, so there is nothing to guard and nothing to free. Any
    /// other unrecognised handle is an error rather than a silent `None`, because
    /// a `None` here would downgrade the caller to the unguarded release this
    /// change exists to remove.
    pub(crate) fn resolve(
        &self,
        handle: PaymentReservationHandle,
    ) -> Result<Option<T>, UnknownReservationHandle> {
        if handle == PaymentReservationHandle::NONE {
            return Ok(None);
        }
        self.lock()
            .tokens
            .get(&handle)
            .copied()
            .map(Some)
            .ok_or(UnknownReservationHandle(handle))
    }

    /// Number of live entries. Test-only introspection.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.lock().tokens.len()
    }
}

/// Process-global table backing `core_wallet_build_signed_payment` /
/// `core_wallet_release_payment_reservation`.
pub(crate) static PAYMENT_RESERVATIONS: Lazy<PaymentReservationTable<FundingReservationToken>> =
    Lazy::new(PaymentReservationTable::new);

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh table per test — the process-global one is shared with every other
    /// test in the binary and cannot support exact-count assertions. `u64` stands
    /// in for the real token, whose constructor is private by design.
    fn table() -> PaymentReservationTable<u64> {
        PaymentReservationTable::new()
    }

    #[test]
    fn the_none_sentinel_round_trips_without_minting() {
        let t = table();
        let handle = t.stash(None);
        assert_eq!(handle, PaymentReservationHandle::NONE);
        assert_eq!(handle.as_u64(), 0);
        assert_eq!(t.len(), 0, "the sentinel must not occupy an entry");
        assert!(
            matches!(t.resolve(handle), Ok(None)),
            "the sentinel resolves to 'no token', not an error"
        );
    }

    /// A fabricated handle must resolve to an error, never to some other build's
    /// token and never to a silent `None` — a `None` would downgrade the caller
    /// to the unguarded release that reopens the double-spend window.
    #[test]
    fn an_unknown_handle_is_refused() {
        let t = table();
        t.stash(Some(11));
        for forged in [2u64, 99, u64::MAX] {
            assert!(
                matches!(
                    t.resolve(PaymentReservationHandle::from(forged)),
                    Err(UnknownReservationHandle(_))
                ),
                "handle {forged} was never minted and must be refused"
            );
        }
    }

    /// Handles are unique and never collide with the `NONE` sentinel, so a stale
    /// handle can always be told apart from a live one.
    #[test]
    fn minted_handles_are_unique_and_never_zero() {
        let t = table();
        let mut seen = std::collections::HashSet::new();
        for i in 0..1_000u64 {
            let h = t.stash(Some(i));
            assert_ne!(h, PaymentReservationHandle::NONE);
            assert!(seen.insert(h), "handle {h:?} was minted twice");
        }
    }

    /// The ring is bounded: past `MAX_ENTRIES` the oldest handles are evicted and
    /// then refused, while every newer one stays resolvable.
    #[test]
    fn the_table_is_bounded_and_evicts_oldest_first() {
        let t = table();
        let first: Vec<_> = (0..MAX_ENTRIES as u64).map(|i| t.stash(Some(i))).collect();
        assert_eq!(t.len(), MAX_ENTRIES);
        assert!(
            t.resolve(first[0]).is_ok(),
            "nothing is evicted while the table is merely full"
        );

        // One more mint evicts exactly one — the oldest.
        let newest = t.stash(Some(9_999));
        assert_eq!(t.len(), MAX_ENTRIES, "capacity must stay fixed");
        assert!(
            matches!(t.resolve(first[0]), Err(UnknownReservationHandle(_))),
            "the oldest handle must have been evicted"
        );
        assert!(
            t.resolve(first[1]).is_ok() && t.resolve(newest).is_ok(),
            "every other handle must still resolve"
        );
    }

    /// Resolving does NOT consume, which is what keeps the release idempotent.
    #[test]
    fn resolving_does_not_consume_the_handle() {
        let t = table();
        let handle = t.stash(Some(42));
        for attempt in 0..5 {
            assert!(
                matches!(t.resolve(handle), Ok(Some(42))),
                "resolve attempt {attempt} must still find the token"
            );
        }
        assert_eq!(t.len(), 1);
    }

    /// Each stashed token is addressed by its own handle — no aliasing, which is
    /// what stops one build's release from presenting another build's token.
    #[test]
    fn handles_address_their_own_token() {
        let t = table();
        let handles: Vec<_> = (0..16u64).map(|i| t.stash(Some(i * 7))).collect();
        for (i, handle) in handles.iter().enumerate() {
            assert!(
                matches!(t.resolve(*handle), Ok(Some(v)) if v == i as u64 * 7),
                "handle {i} resolved to the wrong token"
            );
        }
    }
}
