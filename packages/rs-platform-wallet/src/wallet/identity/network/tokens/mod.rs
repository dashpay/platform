//! Token state-transition operations on `IdentityWallet`.
//!
//! Each action — transfer, mint, burn, freeze, unfreeze, claim,
//! destroy_frozen_funds, pause, resume, set_price, purchase, update_config —
//! is an identity-as-actor operation, so it lives here next to the rest of
//! the identity-lifecycle and DashPay surface (same precedent as the
//! DashPay merge described in the parent `mod.rs`). Token *bookkeeping*
//! (watch / sync / balance) stays on [`crate::wallet::tokens::TokenWallet`]
//! because it's wallet-scoped, not identity-scoped.

mod burn;
mod claim;
mod destroy_frozen_funds;
mod freeze;
mod helpers;
mod mint;
mod pause;
mod purchase;
mod resume;
mod set_price;
mod transfer;
mod unfreeze;
mod update_config;
