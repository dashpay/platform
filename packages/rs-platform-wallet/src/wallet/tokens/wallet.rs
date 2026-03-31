//! Token wallet with per-identity registry-based balance tracking.
//!
//! Consumers register which tokens to watch per identity via
//! [`watch`](TokenWallet::watch). [`sync`](TokenWallet::sync) queries Platform
//! for balances of all watched identity+token pairs.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use dpp::balances::credits::TokenAmount;
use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use tokio::sync::RwLock;

use dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalancesQuery;
use dash_sdk::platform::FetchMany;

use crate::error::PlatformWalletError;
use crate::wallet::identity::IdentityManager;
use crate::wallet::signer::IdentitySigner;

/// Key for the balance cache and watch registry: (identity_id, token_id).
type IdentityTokenKey = (Identifier, Identifier);

/// Token wallet providing per-identity token balance tracking and operations.
///
/// Tokens are watched per-identity via [`watch`](Self::watch) because Platform
/// has no "list all tokens for an identity" query — the caller must know which
/// token IDs each identity cares about.
#[derive(Clone)]
pub struct TokenWallet {
    pub(crate) sdk: dash_sdk::Sdk,
    pub(crate) wallet: Arc<RwLock<Wallet>>,
    pub(crate) identity_manager: Arc<RwLock<IdentityManager>>,
    pub(crate) network: Network,
    /// Per-identity set of watched token IDs.
    watched: Arc<RwLock<BTreeMap<Identifier, BTreeSet<Identifier>>>>,
    /// Cached balances keyed by (identity_id, token_id).
    balances: Arc<RwLock<BTreeMap<IdentityTokenKey, TokenAmount>>>,
}

impl TokenWallet {
    /// Create a new TokenWallet.
    pub(crate) fn new(
        sdk: dash_sdk::Sdk,
        wallet: Arc<RwLock<Wallet>>,
        identity_manager: Arc<RwLock<IdentityManager>>,
        network: Network,
    ) -> Self {
        Self {
            sdk,
            wallet,
            identity_manager,
            network,
            watched: Arc::new(RwLock::new(BTreeMap::new())),
            balances: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

// ---------------------------------------------------------------------------
// Token registry (per-identity)
// ---------------------------------------------------------------------------

impl TokenWallet {
    /// Register a token for balance tracking on a specific identity.
    pub async fn watch(&self, identity_id: Identifier, token_id: Identifier) {
        let mut watched = self.watched.write().await;
        watched.entry(identity_id).or_default().insert(token_id);
    }

    /// Unregister a token from a specific identity and clear its cached balance.
    pub async fn unwatch(&self, identity_id: &Identifier, token_id: &Identifier) {
        let mut watched = self.watched.write().await;
        if let Some(tokens) = watched.get_mut(identity_id) {
            tokens.remove(token_id);
            if tokens.is_empty() {
                watched.remove(identity_id);
            }
        }
        drop(watched);

        let mut balances = self.balances.write().await;
        balances.remove(&(*identity_id, *token_id));
    }

    /// Unregister all tokens for a specific identity and clear cached balances.
    pub async fn unwatch_identity(&self, identity_id: &Identifier) {
        let mut watched = self.watched.write().await;
        watched.remove(identity_id);
        drop(watched);

        let mut balances = self.balances.write().await;
        balances.retain(|(iid, _), _| iid != identity_id);
    }

    /// Get the watched token IDs for a specific identity.
    pub async fn watched_for(&self, identity_id: &Identifier) -> Vec<Identifier> {
        let watched = self.watched.read().await;
        watched
            .get(identity_id)
            .map(|tokens| tokens.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get all watched (identity_id, token_id) pairs.
    pub async fn watched(&self) -> Vec<IdentityTokenKey> {
        let watched = self.watched.read().await;
        watched
            .iter()
            .flat_map(|(iid, tokens)| tokens.iter().map(move |tid| (*iid, *tid)))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

impl TokenWallet {
    /// Sync balances for all watched identity+token pairs.
    ///
    /// Queries Platform per identity, fetching only the tokens that identity
    /// is watching. Updates the local cache.
    pub async fn sync(&self) -> Result<(), PlatformWalletError> {
        let snapshot: BTreeMap<Identifier, Vec<Identifier>> = {
            let w = self.watched.read().await;
            w.iter()
                .map(|(iid, tokens)| (*iid, tokens.iter().copied().collect()))
                .collect()
        };

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

            let result: dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalances =
                TokenAmount::fetch_many(&self.sdk, query).await.map_err(|e| {
                    PlatformWalletError::TokenError(format!(
                        "Failed to fetch token balances for identity {}: {}",
                        identity_id, e
                    ))
                })?;

            let mut balances = self.balances.write().await;
            for (token_id, maybe_balance) in result.iter() {
                let key = (*identity_id, *token_id);
                match maybe_balance {
                    Some(amount) => {
                        balances.insert(key, *amount);
                    }
                    None => {
                        balances.remove(&key);
                    }
                }
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
        let balances = self.balances.read().await;
        balances.get(&(*identity_id, *token_id)).copied()
    }

    /// Get all cached token balances for an identity.
    pub async fn balances_for_identity(
        &self,
        identity_id: &Identifier,
    ) -> BTreeMap<Identifier, TokenAmount> {
        let balances = self.balances.read().await;
        balances
            .iter()
            .filter(|((iid, _), _)| iid == identity_id)
            .map(|((_, tid), &amount)| (*tid, amount))
            .collect()
    }

    /// Get all cached balances as (identity_id, token_id) -> amount.
    pub async fn all_balances(&self) -> BTreeMap<IdentityTokenKey, TokenAmount> {
        let balances = self.balances.read().await;
        balances.clone()
    }
}

// ---------------------------------------------------------------------------
// Token operations
// ---------------------------------------------------------------------------

impl TokenWallet {
    /// Resolve an identity + signer + signing key for token operations.
    async fn resolve_identity_and_signer(
        &self,
        identity_id: &Identifier,
    ) -> Result<
        (
            dpp::identity::Identity,
            IdentitySigner,
            IdentityPublicKey,
        ),
        PlatformWalletError,
    > {
        let manager = self.identity_manager.read().await;

        let identity = manager
            .identity(identity_id)
            .cloned()
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let identity_index = manager.identity_index(identity_id).ok_or(
            PlatformWalletError::IdentityIndexNotSet(*identity_id),
        )?;

        let signer = IdentitySigner::new(self.wallet.clone(), self.network, identity_index);

        let signing_key = identity
            .get_first_public_key_matching(
                Purpose::AUTHENTICATION,
                [SecurityLevel::MASTER, SecurityLevel::HIGH].into(),
                [KeyType::ECDSA_SECP256K1].into(),
                false,
            )
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "No authentication key found on identity".to_string(),
                )
            })?
            .clone();

        Ok((identity, signer, signing_key))
    }

    /// Transfer tokens from one identity to another.
    pub async fn transfer(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        from_identity_id: &Identifier,
        to_identity_id: Identifier,
        amount: TokenAmount,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::transfer::TokenTransferTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.resolve_identity_and_signer(from_identity_id).await?;

        let builder = TokenTransferTransitionBuilder::new(
            data_contract,
            token_position,
            *from_identity_id,
            to_identity_id,
            amount,
        );

        self.sdk
            .token_transfer(builder, &signing_key, &signer)
            .await
            .map_err(|e| {
                PlatformWalletError::TokenError(format!("Token transfer failed: {}", e))
            })?;

        Ok(())
    }

    /// Mint tokens (admin operation).
    pub async fn mint(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: &Identifier,
        amount: TokenAmount,
        recipient_id: Option<Identifier>,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::mint::TokenMintTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.resolve_identity_and_signer(identity_id).await?;

        let mut builder = TokenMintTransitionBuilder::new(
            data_contract,
            token_position,
            *identity_id,
            amount,
        );

        if let Some(recipient) = recipient_id {
            builder.recipient_id = Some(recipient);
        }

        self.sdk
            .token_mint(builder, &signing_key, &signer)
            .await
            .map_err(|e| {
                PlatformWalletError::TokenError(format!("Token mint failed: {}", e))
            })?;

        Ok(())
    }

    /// Burn tokens (admin operation).
    pub async fn burn(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: &Identifier,
        amount: TokenAmount,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::burn::TokenBurnTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.resolve_identity_and_signer(identity_id).await?;

        let builder = TokenBurnTransitionBuilder::new(
            data_contract,
            token_position,
            *identity_id,
            amount,
        );

        self.sdk
            .token_burn(builder, &signing_key, &signer)
            .await
            .map_err(|e| {
                PlatformWalletError::TokenError(format!("Token burn failed: {}", e))
            })?;

        Ok(())
    }

    /// Freeze an identity's token balance (admin operation).
    pub async fn freeze(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: &Identifier,
        target_identity_id: Identifier,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::freeze::TokenFreezeTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.resolve_identity_and_signer(identity_id).await?;

        let builder = TokenFreezeTransitionBuilder::new(
            data_contract,
            token_position,
            *identity_id,
            target_identity_id,
        );

        self.sdk
            .token_freeze(builder, &signing_key, &signer)
            .await
            .map_err(|e| {
                PlatformWalletError::TokenError(format!("Token freeze failed: {}", e))
            })?;

        Ok(())
    }

    /// Unfreeze an identity's token balance (admin operation).
    pub async fn unfreeze(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: &Identifier,
        target_identity_id: Identifier,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::unfreeze::TokenUnfreezeTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.resolve_identity_and_signer(identity_id).await?;

        let builder = TokenUnfreezeTransitionBuilder::new(
            data_contract,
            token_position,
            *identity_id,
            target_identity_id,
        );

        self.sdk
            .token_unfreeze_identity(builder, &signing_key, &signer)
            .await
            .map_err(|e| {
                PlatformWalletError::TokenError(format!("Token unfreeze failed: {}", e))
            })?;

        Ok(())
    }

    /// Set the direct purchase price for a token (admin operation).
    pub async fn set_price(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: &Identifier,
        price: dpp::fee::Credits,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::set_price::TokenChangeDirectPurchasePriceTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.resolve_identity_and_signer(identity_id).await?;

        let builder = TokenChangeDirectPurchasePriceTransitionBuilder::new(
            data_contract,
            token_position,
            *identity_id,
        )
        .with_single_price(price);

        self.sdk
            .token_set_price_for_direct_purchase(builder, &signing_key, &signer)
            .await
            .map_err(|e| {
                PlatformWalletError::TokenError(format!("Token set price failed: {}", e))
            })?;

        Ok(())
    }

    /// Purchase tokens directly at the set price.
    pub async fn purchase(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: &Identifier,
        amount: TokenAmount,
        total_agreed_price: dpp::fee::Credits,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::purchase::TokenDirectPurchaseTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.resolve_identity_and_signer(identity_id).await?;

        let builder = TokenDirectPurchaseTransitionBuilder::new(
            data_contract,
            token_position,
            *identity_id,
            amount,
            total_agreed_price,
        );

        self.sdk
            .token_purchase(builder, &signing_key, &signer)
            .await
            .map_err(|e| {
                PlatformWalletError::TokenError(format!("Token purchase failed: {}", e))
            })?;

        Ok(())
    }

    /// Claim token distribution rewards.
    pub async fn claim(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: &Identifier,
        distribution_type: dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::claim::TokenClaimTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.resolve_identity_and_signer(identity_id).await?;

        let builder = TokenClaimTransitionBuilder::new(
            data_contract,
            token_position,
            *identity_id,
            distribution_type,
        );

        self.sdk
            .token_claim(builder, &signing_key, &signer)
            .await
            .map_err(|e| {
                PlatformWalletError::TokenError(format!("Token claim failed: {}", e))
            })?;

        Ok(())
    }
}

impl std::fmt::Debug for TokenWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenWallet")
            .field("network", &self.network)
            .finish()
    }
}
