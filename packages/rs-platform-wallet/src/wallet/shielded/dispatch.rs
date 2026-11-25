//! The host's dispatch hook for shielded note spends.
//!
//! A host that sets a [`ShieldedDispatch`] on a network's coordinator
//! ([`NetworkShieldedCoordinator::set_shielded_dispatch`]) sees every note
//! spend made through a [`PlatformWallet`] of that network before it is
//! broadcast, and can refuse it:
//!
//! 1. the library saves the spend's recovery record durably and hands its
//!    notes over to it, as it does without a hook;
//! 2. it awaits [`ShieldedDispatch::dispatch`] with no store guard held;
//! 3. `Ok` lets the broadcast go ahead. A refusal sends nothing: the record
//!    is retired and the notes are released, as for any spend refused before
//!    delivery, and the call returns
//!    [`PlatformWalletError::ShieldedDispatchRefused`]. If the record can't be
//!    retired (the store's write fails), or a sync pass claimed it while the
//!    hook was awaited, the call reports the spend as unconfirmed instead, and
//!    the record and its notes are kept;
//! 4. every later re-broadcast of a saved note spend by a sync pass first
//!    awaits [`ShieldedDispatch::admit`]. A refusal skips it for that pass:
//!    the record stays, its attempt count is unchanged and its notes stay
//!    reserved.
//!
//! # Why after the hand-over
//!
//! Once the record is saved its notes are handed over with nothing awaited
//! in between, so a send dropped while the hook is awaited (cancelled, timed
//! out, shut down) leaves both the notes and the record: it never frees a
//! note under a transition that may still be sent.
//!
//! # A pass, a drop or a crash during the hook
//!
//! A saved spend that the hook has not accepted can reach a sync pass in
//! three ways: the pass runs while the hook is awaited; the send is dropped
//! or its hook panics; or the process crashes, or the send is cancelled,
//! between the record's save and the host's commit of what the hook
//! registered. In each case the pass asks [`ShieldedDispatch::admit`] before
//! re-broadcasting it, so a host that admits only transitions it accepted
//! from [`ShieldedDispatch::dispatch`] never sees one leave the wallet first.
//! A host that admits a transition it refused lets the pass send it, and the
//! refusal then can't retire its record: the spend is reported unconfirmed
//! and may execute.
//!
//! The hook must be set before the first wallet is bound and before shielded
//! sync starts, not only before the first spend: a sync pass that runs with
//! no hook set re-broadcasts every saved record without asking.
//!
//! Such a record keeps its notes reserved. Its reservation carries its
//! anchor, so the sync pass's prune releases them once that anchor is pruned
//! from Platform's recorded set, or its nullifiers land. The hook runs before
//! the broadcast, so a record the host never accepted was never sent by this
//! call.
//!
//! [`NetworkShieldedCoordinator::set_shielded_dispatch`]:
//!     super::NetworkShieldedCoordinator::set_shielded_dispatch
//! [`PlatformWallet`]: crate::wallet::platform_wallet::PlatformWallet
//! [`PlatformWalletError::ShieldedDispatchRefused`]:
//!     crate::error::PlatformWalletError::ShieldedDispatchRefused

use async_trait::async_trait;
use dpp::state_transition::StateTransition;

use crate::wallet::platform_wallet::WalletId;

/// The host's hook for shielded note spends: transfer, unshield, withdrawal,
/// identity top-up and identity creation from the pool, and private document
/// creation paid from the pool. See the [module docs](self).
///
/// # Locks
///
/// Both methods are awaited with no shielded store guard and no coordinator
/// lock held, so they may read the store. A running sync pass holds the
/// store's write guard while it scans, so such a read waits for the scan.
///
/// [`Self::admit`] runs inside a sync pass, so it should answer promptly. It
/// must not wait for a [`Self::dispatch`] still awaited (a pass can overlap
/// one), clear the shielded state, remove or tear down a wallet, or shut the
/// manager down: each of those waits for the pass. A stop of the pass drops
/// an `admit` still awaited.
///
/// An identity top-up runs with its wallet's shield-class guard held, so a
/// hook must not start a shield, top-up or other shield-class operation on
/// the same wallet: it would wait for itself. A private contact request
/// payment runs with its wallet's payment guard read-held, so a hook must
/// not remove or tear down that wallet, nor start another such payment while
/// a teardown may be waiting. Nor may it await a spend of the same
/// coordinator.
#[async_trait]
pub trait ShieldedDispatch: Send + Sync {
    /// See `payload`'s spend before its first broadcast. Its recovery record
    /// is already durable when this runs. `Ok` lets the broadcast go ahead;
    /// `Err` refuses it, and this call sends nothing.
    async fn dispatch(&self, payload: ShieldedDispatchPayload) -> Result<(), DispatchRefused>;

    /// Allow a sync pass to re-broadcast the saved note spend of transition
    /// `st_hash` (the SHA-256 of its bytes, as in
    /// [`ShieldedDispatchPayload::st_hash`]), recorded under `activity_id` in
    /// `wallet_id`'s account `account_index`. `Err` skips it for this pass.
    ///
    /// It is asked about every saved note spend a pass would re-broadcast
    /// while the hook is set, including ones saved before it was set or in
    /// an earlier run, and ones whose [`Self::dispatch`] never answered `Ok`.
    /// A host should admit only transitions it has accepted, and refuse any it
    /// has no record of. A record that is never admitted keeps its notes
    /// reserved until its anchor is pruned or its nullifiers land, so a host
    /// that starts using the hook holds the records saved before it until
    /// then. Identity-funded shields spend no notes and are not put to it.
    async fn admit(
        &self,
        wallet_id: WalletId,
        account_index: u32,
        activity_id: [u8; 32],
        st_hash: [u8; 32],
    ) -> Result<(), DispatchRefused>;
}

/// A refusal from a [`ShieldedDispatch`] hook.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{reason}")]
pub struct DispatchRefused {
    /// Why the hook refused.
    pub reason: String,
}

impl DispatchRefused {
    /// A refusal for `reason`.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

/// Which operation a dispatched spend is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShieldedDispatchKind {
    /// A shielded transfer.
    Transfer,
    /// An unshield to a platform address.
    Unshield,
    /// A withdrawal to a Core address.
    Withdraw,
    /// A top-up of `identity_id` from the pool.
    IdentityTopUp { identity_id: [u8; 32] },
    /// The creation of `identity_id` from the pool.
    IdentityCreate { identity_id: [u8; 32] },
    /// A private document create paid from the pool.
    PrivateDocumentCreate,
}

impl ShieldedDispatchKind {
    /// The kind of the note spend `st`, or `None` for a transition that
    /// spends no notes.
    pub(crate) fn of(st: &StateTransition) -> Option<Self> {
        use dpp::state_transition::identity_top_up_from_shielded_pool_transition::accessors::IdentityTopUpFromShieldedPoolTransitionAccessorsV0;
        use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;

        Some(match st {
            StateTransition::ShieldedTransfer(_) => Self::Transfer,
            StateTransition::Unshield(_) => Self::Unshield,
            StateTransition::ShieldedWithdrawal(_) => Self::Withdraw,
            StateTransition::IdentityTopUpFromShieldedPool(t) => Self::IdentityTopUp {
                identity_id: t.identity_id().to_buffer(),
            },
            StateTransition::IdentityCreateFromShieldedPool(t) => Self::IdentityCreate {
                identity_id: t.identity_id().to_buffer(),
            },
            StateTransition::PrivateDocumentCreateFromShieldedPool(_) => {
                Self::PrivateDocumentCreate
            }
            _ => return None,
        })
    }
}

/// One of the wallet's own notes a dispatched spend consumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchedNote {
    pub nullifier: [u8; 32],
    pub cmx: [u8; 32],
    pub value: u64,
    pub position: u64,
}

/// Everything a [`ShieldedDispatch::dispatch`] call is shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShieldedDispatchPayload {
    pub kind: ShieldedDispatchKind,
    pub wallet_id: WalletId,
    pub account_index: u32,
    /// The spend's activity entry id, which keys its recovery record.
    pub activity_id: [u8; 32],
    /// The SHA-256 of [`Self::st_bytes`]: the transition's id on Platform.
    pub st_hash: [u8; 32],
    /// The signed transition exactly as it will be broadcast.
    pub st_bytes: Vec<u8>,
    /// The commitment-tree anchor the spend was built against.
    pub anchor: [u8; 32],
    /// The wallet's own notes the spend consumes: exactly the nullifiers its
    /// recovery record reserves. Padding actions are not among them.
    pub selected_notes: Vec<DispatchedNote>,
    /// The nullifier of every action of the transition, in action order,
    /// padding included. A one-note spend is padded to two actions, so this
    /// can hold nullifiers that match no note of the wallet.
    pub published_nullifiers: Vec<[u8; 32]>,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::error::PlatformWalletError;
    use crate::wallet::shielded::{FileBackedShieldedStore, NetworkShieldedCoordinator};

    struct Accepting;

    #[async_trait]
    impl ShieldedDispatch for Accepting {
        async fn dispatch(&self, _: ShieldedDispatchPayload) -> Result<(), DispatchRefused> {
            Ok(())
        }

        async fn admit(
            &self,
            _: WalletId,
            _: u32,
            _: [u8; 32],
            _: [u8; 32],
        ) -> Result<(), DispatchRefused> {
            Ok(())
        }
    }

    /// A coordinator's hook is set once, and a second one is refused and
    /// changes nothing.
    #[test]
    fn should_set_the_hook_once() {
        let dir = tempfile::tempdir().expect("a directory");
        let path = dir.path().join("tree.sqlite");
        let coordinator = NetworkShieldedCoordinator::new(
            Arc::new(dash_sdk::Sdk::new_mock()),
            dashcore::Network::Testnet,
            path.clone(),
            FileBackedShieldedStore::open_path(&path, 100).expect("the store"),
        );
        assert!(coordinator.shielded_dispatch().is_none());
        let hook: Arc<dyn ShieldedDispatch> = Arc::new(Accepting);
        coordinator
            .set_shielded_dispatch(Arc::clone(&hook))
            .expect("set");
        assert!(matches!(
            coordinator.set_shielded_dispatch(Arc::new(Accepting)),
            Err(PlatformWalletError::InvalidParameter(_))
        ));
        let set = coordinator.shielded_dispatch().expect("the hook is set");
        assert!(std::ptr::addr_eq(set, Arc::as_ptr(&hook)));
    }
}
