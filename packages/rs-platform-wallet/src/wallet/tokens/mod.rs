//! Token-contract group-action discovery helpers.
//!
//! Per-(identity, token) balance bookkeeping lives on
//! [`crate::manager::identity_sync::IdentitySyncManager`] — there is
//! no `TokenWallet` anymore. Token *actions* (transfer / mint / burn
//! / freeze / unfreeze / claim / purchase / set-price / pause /
//! resume / destroy-frozen-funds / update-config) live on
//! [`IdentityWallet`](crate::wallet::identity::network::IdentityWallet)
//! since they're identity-as-actor operations.
//!
//! What stays in this module is the read-only group-action discovery
//! surface (free functions taking `&Sdk`).

mod group_queries;

pub use group_queries::{
    group_action_signers_external, pending_group_actions_external, GroupActionEntry,
    GroupActionParams, GroupActionSignerEntry,
};
