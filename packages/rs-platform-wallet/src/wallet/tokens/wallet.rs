//! Token wallet with per-identity registry-based balance tracking.
//!
//! Consumers register which tokens to watch per identity via
//! [`watch`](TokenWallet::watch). [`sync`](TokenWallet::sync) queries Platform
//! for balances of all watched identity+token pairs.
//!
//! Token *actions* (transfer / mint / burn / freeze / unfreeze / claim /
//! purchase / set-price / pause / resume / destroy-frozen-funds /
//! update-config) are identity-as-actor operations and live on
//! [`IdentityWallet`](crate::wallet::identity::network::IdentityWallet)
//! alongside the rest of the identity-lifecycle and DashPay surface.
//! What stays here is wallet-scoped bookkeeping only: the watch
//! registry, the per-identity balance cache, and the `sync` driver
//! that refreshes those balances from Platform.

use std::collections::BTreeMap;
use std::sync::Arc;

use dpp::balances::credits::TokenAmount;
use dpp::prelude::Identifier;
use tokio::sync::RwLock;

use dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalancesQuery;
use dash_sdk::platform::FetchMany;

use crate::changeset::{Merge, TokenBalanceChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use key_wallet_manager::WalletManager;

/// Key for the balance cache and watch registry: (identity_id, token_id).
type IdentityTokenKey = (Identifier, Identifier);

/// Token wallet providing per-identity token balance tracking.
///
/// Tokens are watched per-identity via [`watch`](Self::watch) because Platform
/// has no "list all tokens for an identity" query — the caller must know which
/// token IDs each identity cares about.
#[derive(Clone)]
pub struct TokenWallet {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    /// The shared wallet manager lock for all mutable wallet state.
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifies which wallet within the manager this sub-wallet operates on.
    pub(crate) wallet_id: WalletId,
    /// Per-wallet persistence handle for queuing changesets.
    pub(crate) persister: crate::wallet::persister::WalletPersister,
}

impl TokenWallet {
    /// Create a new TokenWallet.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        persister: crate::wallet::persister::WalletPersister,
    ) -> Self {
        Self {
            sdk,
            wallet_manager,
            wallet_id,
            persister,
        }
    }
}

// ---------------------------------------------------------------------------
// Token registry (per-identity)
// ---------------------------------------------------------------------------

impl TokenWallet {
    /// Register a token for balance tracking on a specific identity.
    ///
    /// Persists the resulting changeset internally and returns `()`.
    pub async fn watch(&self, identity_id: Identifier, token_id: Identifier) {
        let mut wm = self.wallet_manager.write().await;
        let mut cs = TokenBalanceChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            info.token_watched
                .entry(identity_id)
                .or_default()
                .insert(token_id);
        }
        cs.watched.entry(identity_id).or_default().insert(token_id);
        if let Err(e) = self.persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Unregister a token from a specific identity and clear its cached balance.
    ///
    /// Persists the resulting changeset internally and returns `()`.
    pub async fn unwatch(&self, identity_id: &Identifier, token_id: &Identifier) {
        let mut wm = self.wallet_manager.write().await;
        let mut cs = TokenBalanceChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            if let Some(tokens) = info.token_watched.get_mut(identity_id) {
                tokens.remove(token_id);
                if tokens.is_empty() {
                    info.token_watched.remove(identity_id);
                }
            }
            info.token_balances.remove(&(*identity_id, *token_id));
        }
        cs.unwatched
            .entry(*identity_id)
            .or_default()
            .insert(*token_id);
        cs.removed_balances.insert((*identity_id, *token_id));
        if let Err(e) = self.persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Unregister all tokens for a specific identity and clear cached balances.
    ///
    /// Persists the resulting changeset internally and returns `()`.
    pub async fn unwatch_identity(&self, identity_id: &Identifier) {
        let mut wm = self.wallet_manager.write().await;
        let mut cs = TokenBalanceChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            if let Some(tokens) = info.token_watched.remove(identity_id) {
                cs.unwatched.insert(*identity_id, tokens);
            }
            let to_remove: Vec<_> = info
                .token_balances
                .keys()
                .filter(|(iid, _)| iid == identity_id)
                .copied()
                .collect();
            for key in &to_remove {
                info.token_balances.remove(key);
                cs.removed_balances.insert(*key);
            }
        }
        if let Err(e) = self.persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Get the watched token IDs for a specific identity.
    pub async fn watched_for(&self, identity_id: &Identifier) -> Vec<Identifier> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .and_then(|info| info.token_watched.get(identity_id))
            .map(|tokens| tokens.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get all watched (identity_id, token_id) pairs.
    pub async fn watched(&self) -> Vec<IdentityTokenKey> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| {
                info.token_watched
                    .iter()
                    .flat_map(|(iid, tokens)| tokens.iter().map(move |tid| (*iid, *tid)))
                    .collect()
            })
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

impl TokenWallet {
    /// Sync balances for all watched identity+token pairs.
    ///
    /// Queries Platform per identity, fetching only the tokens that identity
    /// is watching. Updates the local cache and persists the resulting
    /// changeset internally. Returns `()` on success.
    pub async fn sync(&self) -> Result<(), PlatformWalletError> {
        // Snapshot the watched tokens while holding the lock briefly.
        let snapshot: BTreeMap<Identifier, Vec<Identifier>> = {
            let wm = self.wallet_manager.read().await;
            wm.get_wallet_info(&self.wallet_id)
                .map(|info| {
                    info.token_watched
                        .iter()
                        .map(|(iid, tokens)| (*iid, tokens.iter().copied().collect()))
                        .collect()
                })
                .unwrap_or_default()
        };

        let mut cs = TokenBalanceChangeSet::default();
        if snapshot.is_empty() {
            return Ok(());
        }

        for (identity_id, token_ids) in &snapshot {
            if token_ids.is_empty() {
                continue;
            }

            let query = IdentityTokenBalancesQuery {
                identity_id: *identity_id,
                token_ids: token_ids.clone(),
            };

            // No locks held during the network call.
            let result: dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalances =
                TokenAmount::fetch_many(&self.sdk, query)
                    .await
                    .map_err(|e| {
                        PlatformWalletError::TokenError(format!(
                            "Failed to fetch token balances for identity {}: {}",
                            identity_id, e
                        ))
                    })?;

            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                for (token_id, maybe_balance) in result.iter() {
                    let key = (*identity_id, *token_id);
                    match maybe_balance {
                        Some(amount) => {
                            info.token_balances.insert(key, *amount);
                            cs.balances.insert(key, *amount);
                        }
                        None => {
                            info.token_balances.remove(&key);
                            cs.removed_balances.insert(key);
                        }
                    }
                }
            }
        }

        if !cs.is_empty() {
            if let Err(e) = self.persister.store(cs.into()) {
                tracing::error!("Failed to persist changeset: {}", e);
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Balance queries (from cache)
// ---------------------------------------------------------------------------

impl TokenWallet {
    /// Get the cached balance for a specific identity and token.
    pub async fn balance(
        &self,
        identity_id: &Identifier,
        token_id: &Identifier,
    ) -> Option<TokenAmount> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .and_then(|info| info.token_balances.get(&(*identity_id, *token_id)).copied())
    }

    /// Get all cached token balances for an identity.
    pub async fn balances_for_identity(
        &self,
        identity_id: &Identifier,
    ) -> BTreeMap<Identifier, TokenAmount> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| {
                info.token_balances
                    .iter()
                    .filter(|((iid, _), _)| iid == identity_id)
                    .map(|((_, tid), &amount)| (*tid, amount))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all cached balances as (identity_id, token_id) -> amount.
    pub async fn all_balances(&self) -> BTreeMap<IdentityTokenKey, TokenAmount> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| info.token_balances.clone())
            .unwrap_or_default()
    }
}

impl std::fmt::Debug for TokenWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenWallet")
            .field("network", &self.sdk.network)
            .finish()
    }
}
