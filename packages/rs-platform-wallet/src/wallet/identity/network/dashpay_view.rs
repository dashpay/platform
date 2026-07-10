//! Zero-cost DashPay namespace over [`IdentityWallet`].
//!
//! [`DashPayView`] is a borrowing view — NOT a second facade. The
//! previous owned `DashPayWallet<B>` was merged into `IdentityWallet`
//! because two owned handles over the same state meant two FFI handles,
//! two clones per op, and split impls for ops that straddle the
//! identity/DashPay line. The view keeps that single-handle design and
//! adds only call-site namespacing: every DashPay-contract operation
//! (contact requests, contacts, contactInfo, payments, profiles) is
//! defined on `DashPayView`, reached as `wallet.identity().dashpay()`.
//!
//! The view [`Deref`]s to `IdentityWallet`, so DashPay ops keep direct
//! access to the shared plumbing they straddle into (signer derivation,
//! DPNS resolution, the SDK handle, the wallet manager).

use std::ops::Deref;

use crate::broadcaster::{SpvBroadcaster, TransactionBroadcaster};

use super::identity_handle::IdentityWallet;

/// Borrowing namespace for the DashPay-contract operations of an
/// [`IdentityWallet`]. Obtain via [`IdentityWallet::dashpay`].
pub struct DashPayView<'a, B: TransactionBroadcaster + ?Sized = SpvBroadcaster>(
    &'a IdentityWallet<B>,
);

// Manual Clone/Copy: a derive would bound `B: Clone`/`B: Copy`, but the
// view only holds a shared reference.
impl<B: TransactionBroadcaster + ?Sized> Clone for DashPayView<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<B: TransactionBroadcaster + ?Sized> Copy for DashPayView<'_, B> {}

impl<B: TransactionBroadcaster + ?Sized> Deref for DashPayView<'_, B> {
    type Target = IdentityWallet<B>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// The DashPay-contract operations of this wallet handle: contact
    /// requests, contacts, contactInfo, payments, and profiles.
    pub fn dashpay(&self) -> DashPayView<'_, B> {
        DashPayView(self)
    }
}
