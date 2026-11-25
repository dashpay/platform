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
//!    [`PlatformWalletError::ShieldedDispatchRefused`].
//!
//! # Why after the hand-over
//!
//! Once the record is saved its notes are handed over with nothing awaited
//! in between, so a send dropped while the hook is awaited (cancelled, timed
//! out, shut down) leaves both the notes and the record: it never frees a
//! note under a transition that may still be sent.
//!
//! # What the hook alone does not stop
//!
//! The refusal holds when the hook answers `Err`. A sync pass re-broadcasts
//! every saved spend, so the hook alone does not stop one that:
//!
//! - a sync pass finds while the hook is awaited: the pass may re-broadcast
//!   it before the hook answers. A refusal after that can't retire its
//!   record, and the spend settles as unconfirmed;
//! - is dropped while the hook is awaited, or whose hook panics: its record
//!   stays, and the next pass re-broadcasts it;
//! - was saved before a crash while the hook was awaited, or before the host
//!   committed what the hook registered: a restart restores the record, and
//!   the pass re-broadcasts it.
//!
//! A host that must see every transition before it leaves the wallet runs no
//! sync pass of the network while a hook is awaited and doesn't drop a send
//! mid-hook.
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
/// It is awaited with no shielded store guard and no coordinator lock held,
/// so it may read the store. A running sync pass holds the store's write
/// guard while it scans, so such a read waits for the scan.
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
