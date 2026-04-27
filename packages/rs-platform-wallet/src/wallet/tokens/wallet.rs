//! Token wallet with per-identity registry-based balance tracking.
//!
//! Consumers register which tokens to watch per identity via
//! [`watch`](TokenWallet::watch). [`sync`](TokenWallet::sync) queries Platform
//! for balances of all watched identity+token pairs.

use std::collections::BTreeMap;
use std::sync::Arc;

use dpp::balances::credits::TokenAmount;
use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use tokio::sync::RwLock;

use dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalancesQuery;
use dash_sdk::platform::FetchMany;

use crate::changeset::{Merge, TokenBalanceChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::signer::IdentitySigner;
use key_wallet_manager::WalletManager;

/// Key for the balance cache and watch registry: (identity_id, token_id).
type IdentityTokenKey = (Identifier, Identifier);

/// Token wallet providing per-identity token balance tracking and operations.
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

// ---------------------------------------------------------------------------
// Token operations
// ---------------------------------------------------------------------------

impl TokenWallet {
    /// Resolve an identity + signer + signing key for token operations.
    async fn resolve_identity_and_signer(
        &self,
        identity_id: &Identifier,
    ) -> Result<(dpp::identity::Identity, IdentitySigner, IdentityPublicKey), PlatformWalletError>
    {
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

        let identity = info
            .identity_manager
            .identity(identity_id)
            .map(|m| m.identity.clone())
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let identity_index = info
            .identity_manager
            .identity_index(identity_id)
            .ok_or(PlatformWalletError::IdentityIndexNotSet(*identity_id))?;

        let signer = IdentitySigner::new(
            self.wallet_manager.clone(),
            self.wallet_id,
            self.sdk.network,
            identity_index,
        );

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

        let mut builder =
            TokenMintTransitionBuilder::new(data_contract, token_position, *identity_id, amount);

        if let Some(recipient) = recipient_id {
            builder.recipient_id = Some(recipient);
        }

        self.sdk
            .token_mint(builder, &signing_key, &signer)
            .await
            .map_err(|e| PlatformWalletError::TokenError(format!("Token mint failed: {}", e)))?;

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

        let builder =
            TokenBurnTransitionBuilder::new(data_contract, token_position, *identity_id, amount);

        self.sdk
            .token_burn(builder, &signing_key, &signer)
            .await
            .map_err(|e| PlatformWalletError::TokenError(format!("Token burn failed: {}", e)))?;

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
            .map_err(|e| PlatformWalletError::TokenError(format!("Token freeze failed: {}", e)))?;

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
            .map_err(|e| PlatformWalletError::TokenError(format!("Token claim failed: {}", e)))?;

        Ok(())
    }

    /// Destroy frozen funds for a target identity (admin operation).
    pub async fn destroy_frozen_funds(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: &Identifier,
        frozen_identity_id: Identifier,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::destroy::TokenDestroyFrozenFundsTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.resolve_identity_and_signer(identity_id).await?;

        let builder = TokenDestroyFrozenFundsTransitionBuilder::new(
            data_contract,
            token_position,
            *identity_id,
            frozen_identity_id,
        );

        self.sdk
            .token_destroy_frozen_funds(builder, &signing_key, &signer)
            .await
            .map_err(|e| {
                PlatformWalletError::TokenError(format!("Destroy frozen funds failed: {}", e))
            })?;

        Ok(())
    }

    /// Pause a token (emergency action, admin operation).
    pub async fn pause(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: &Identifier,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::emergency_action::TokenEmergencyActionTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.resolve_identity_and_signer(identity_id).await?;

        let builder = TokenEmergencyActionTransitionBuilder::pause(
            data_contract,
            token_position,
            *identity_id,
        );

        self.sdk
            .token_emergency_action(builder, &signing_key, &signer)
            .await
            .map_err(|e| PlatformWalletError::TokenError(format!("Token pause failed: {}", e)))?;

        Ok(())
    }

    /// Resume a paused token (emergency action, admin operation).
    pub async fn resume(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: &Identifier,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::emergency_action::TokenEmergencyActionTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.resolve_identity_and_signer(identity_id).await?;

        let builder = TokenEmergencyActionTransitionBuilder::resume(
            data_contract,
            token_position,
            *identity_id,
        );

        self.sdk
            .token_emergency_action(builder, &signing_key, &signer)
            .await
            .map_err(|e| PlatformWalletError::TokenError(format!("Token resume failed: {}", e)))?;

        Ok(())
    }

    /// Update token configuration (admin operation).
    pub async fn update_config(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: &Identifier,
        config_change: dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::config_update::TokenConfigUpdateTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.resolve_identity_and_signer(identity_id).await?;

        let builder = TokenConfigUpdateTransitionBuilder::new(
            data_contract,
            token_position,
            *identity_id,
            config_change,
        );

        self.sdk
            .token_update_contract_token_configuration(builder, &signing_key, &signer)
            .await
            .map_err(|e| {
                PlatformWalletError::TokenError(format!("Token config update failed: {}", e))
            })?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Extended token operations (external signer, full result types)
// ---------------------------------------------------------------------------
//
// These methods accept an external `Signer` and `IdentityPublicKey` from the
// caller, along with optional builder options (public note, group info,
// state transition creation options). They return the SDK's detailed result
// types so callers can inspect proof-verified outcomes (e.g. updated balances).

impl TokenWallet {
    /// Transfer tokens using an external signer. Returns the SDK result.
    #[allow(clippy::too_many_arguments)]
    pub async fn transfer_with_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        from_identity_id: Identifier,
        to_identity_id: Identifier,
        amount: TokenAmount,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::TransferResult, dash_sdk::Error> {
        use dash_sdk::platform::tokens::builders::transfer::TokenTransferTransitionBuilder;

        let mut builder = TokenTransferTransitionBuilder::new(
            data_contract,
            token_position,
            from_identity_id,
            to_identity_id,
            amount,
        );

        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }

        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk.token_transfer(builder, signing_key, signer).await
    }

    /// Mint tokens using an external signer. Returns the SDK result.
    #[allow(clippy::too_many_arguments)]
    pub async fn mint_with_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        amount: TokenAmount,
        recipient_id: Option<Identifier>,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::MintResult, dash_sdk::Error> {
        use dash_sdk::platform::tokens::builders::mint::TokenMintTransitionBuilder;

        let builder =
            TokenMintTransitionBuilder::new(data_contract, token_position, identity_id, amount);

        let mut builder = if let Some(recipient) = recipient_id {
            builder.issued_to_identity_id(recipient)
        } else {
            builder
        };

        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }

        if let Some(gi) = group_info {
            builder = builder.with_using_group_info(gi);
        }

        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk.token_mint(builder, signing_key, signer).await
    }

    /// Burn tokens using an external signer. Returns the SDK result.
    #[allow(clippy::too_many_arguments)]
    pub async fn burn_with_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        amount: TokenAmount,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::BurnResult, dash_sdk::Error> {
        use dash_sdk::platform::tokens::builders::burn::TokenBurnTransitionBuilder;

        let mut builder =
            TokenBurnTransitionBuilder::new(data_contract, token_position, identity_id, amount);

        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }

        if let Some(gi) = group_info {
            builder = builder.with_using_group_info(gi);
        }

        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk.token_burn(builder, signing_key, signer).await
    }

    /// Freeze tokens using an external signer. Returns the SDK result.
    #[allow(clippy::too_many_arguments)]
    pub async fn freeze_with_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        target_identity_id: Identifier,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::FreezeResult, dash_sdk::Error> {
        use dash_sdk::platform::tokens::builders::freeze::TokenFreezeTransitionBuilder;

        let mut builder = TokenFreezeTransitionBuilder::new(
            data_contract,
            token_position,
            identity_id,
            target_identity_id,
        );

        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }

        if let Some(gi) = group_info {
            builder = builder.with_using_group_info(gi);
        }

        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk.token_freeze(builder, signing_key, signer).await
    }

    /// Unfreeze tokens using an external signer. Returns the SDK result.
    #[allow(clippy::too_many_arguments)]
    pub async fn unfreeze_with_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        target_identity_id: Identifier,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::UnfreezeResult, dash_sdk::Error> {
        use dash_sdk::platform::tokens::builders::unfreeze::TokenUnfreezeTransitionBuilder;

        let mut builder = TokenUnfreezeTransitionBuilder::new(
            data_contract,
            token_position,
            identity_id,
            target_identity_id,
        );

        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }

        if let Some(gi) = group_info {
            builder = builder.with_using_group_info(gi);
        }

        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk
            .token_unfreeze_identity(builder, signing_key, signer)
            .await
    }

    /// Set direct purchase price using an external signer. Returns the SDK result.
    #[allow(clippy::too_many_arguments)]
    pub async fn set_price_with_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        token_pricing_schedule: Option<dpp::tokens::token_pricing_schedule::TokenPricingSchedule>,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::SetPriceResult, dash_sdk::Error> {
        use dash_sdk::platform::tokens::builders::set_price::TokenChangeDirectPurchasePriceTransitionBuilder;

        let mut builder = TokenChangeDirectPurchasePriceTransitionBuilder::new(
            data_contract,
            token_position,
            identity_id,
        );

        if let Some(pricing_schedule) = token_pricing_schedule {
            builder = builder.with_token_pricing_schedule(pricing_schedule);
        }

        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }

        if let Some(gi) = group_info {
            builder = builder.with_using_group_info(gi);
        }

        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk
            .token_set_price_for_direct_purchase(builder, signing_key, signer)
            .await
    }

    /// Purchase tokens using an external signer. Returns the SDK result.
    #[allow(clippy::too_many_arguments)]
    pub async fn purchase_with_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        amount: TokenAmount,
        total_agreed_price: dpp::fee::Credits,
        signing_key: &IdentityPublicKey,
        signer: &S,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::DirectPurchaseResult, dash_sdk::Error>
    {
        use dash_sdk::platform::tokens::builders::purchase::TokenDirectPurchaseTransitionBuilder;

        let mut builder = TokenDirectPurchaseTransitionBuilder::new(
            data_contract,
            token_position,
            identity_id,
            amount,
            total_agreed_price,
        );

        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk.token_purchase(builder, signing_key, signer).await
    }

    /// Claim tokens using an external signer. Returns the SDK result.
    #[allow(clippy::too_many_arguments)]
    pub async fn claim_with_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        distribution_type: dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::ClaimResult, dash_sdk::Error> {
        use dash_sdk::platform::tokens::builders::claim::TokenClaimTransitionBuilder;

        let mut builder = TokenClaimTransitionBuilder::new(
            data_contract,
            token_position,
            identity_id,
            distribution_type,
        );

        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }

        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk.token_claim(builder, signing_key, signer).await
    }

    /// Destroy frozen funds using an external signer.
    #[allow(clippy::too_many_arguments)]
    pub async fn destroy_frozen_funds_with_signer<
        S: dpp::identity::signer::Signer<IdentityPublicKey>,
    >(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        frozen_identity_id: Identifier,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::DestroyFrozenFundsResult, dash_sdk::Error>
    {
        use dash_sdk::platform::tokens::builders::destroy::TokenDestroyFrozenFundsTransitionBuilder;

        let mut builder = TokenDestroyFrozenFundsTransitionBuilder::new(
            data_contract,
            token_position,
            identity_id,
            frozen_identity_id,
        );

        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }
        if let Some(gi) = group_info {
            builder = builder.with_using_group_info(gi);
        }
        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk
            .token_destroy_frozen_funds(builder, signing_key, signer)
            .await
    }

    /// Pause token using an external signer.
    #[allow(clippy::too_many_arguments)]
    pub async fn pause_with_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::EmergencyActionResult, dash_sdk::Error>
    {
        use dash_sdk::platform::tokens::builders::emergency_action::TokenEmergencyActionTransitionBuilder;

        let mut builder = TokenEmergencyActionTransitionBuilder::pause(
            data_contract,
            token_position,
            identity_id,
        );

        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }
        if let Some(gi) = group_info {
            builder = builder.with_using_group_info(gi);
        }
        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk
            .token_emergency_action(builder, signing_key, signer)
            .await
    }

    /// Resume token using an external signer.
    #[allow(clippy::too_many_arguments)]
    pub async fn resume_with_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::EmergencyActionResult, dash_sdk::Error>
    {
        use dash_sdk::platform::tokens::builders::emergency_action::TokenEmergencyActionTransitionBuilder;

        let mut builder = TokenEmergencyActionTransitionBuilder::resume(
            data_contract,
            token_position,
            identity_id,
        );

        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }
        if let Some(gi) = group_info {
            builder = builder.with_using_group_info(gi);
        }
        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk
            .token_emergency_action(builder, signing_key, signer)
            .await
    }

    /// Update token config using an external signer.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_config_with_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        config_change: dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::ConfigUpdateResult, dash_sdk::Error> {
        use dash_sdk::platform::tokens::builders::config_update::TokenConfigUpdateTransitionBuilder;

        let mut builder = TokenConfigUpdateTransitionBuilder::new(
            data_contract,
            token_position,
            identity_id,
            config_change,
        );

        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }
        if let Some(gi) = group_info {
            builder = builder.with_using_group_info(gi);
        }
        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk
            .token_update_contract_token_configuration(builder, signing_key, signer)
            .await
    }
}

// ---------------------------------------------------------------------------
// External-signer helpers (FFI-facing)
// ---------------------------------------------------------------------------
//
// These wrap the `_with_signer` methods above with the contract fetch and
// the identity / signing-key lookup so the FFI layer stays a thin shim.
// Each helper:
//   1. Fetches the data contract by id (no caller-supplied contract).
//   2. Resolves the signing key on the identity (AUTHENTICATION purpose,
//      MASTER or HIGH security, ECDSA_SECP256K1).
//   3. Delegates to the existing `_with_signer` method using the
//      caller-provided `Signer` implementation.
//
// Keeping the orchestration here rather than in the FFI honours the
// `swift-sdk/CLAUDE.md` "high-level operations go through platform-wallet"
// rule.

impl TokenWallet {
    /// Resolve the AUTHENTICATION signing key on an identity for token
    /// state-transition operations. Mirrors the lookup in
    /// `resolve_identity_and_signer` but only returns the key — callers
    /// that bring their own `Signer` skip the `IdentitySigner`
    /// construction.
    async fn resolve_signing_key(
        &self,
        identity_id: &Identifier,
    ) -> Result<IdentityPublicKey, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

        let identity = info
            .identity_manager
            .identity(identity_id)
            .map(|m| m.identity.clone())
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

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

        Ok(signing_key)
    }

    /// Fetch a data contract by id from Platform.
    async fn fetch_data_contract(
        &self,
        contract_id: Identifier,
    ) -> Result<Arc<DataContract>, PlatformWalletError> {
        use dash_sdk::platform::Fetch;

        let contract = DataContract::fetch(&*self.sdk, contract_id)
            .await
            .map_err(|e| PlatformWalletError::TokenError(format!("Fetch contract failed: {}", e)))?
            .ok_or_else(|| {
                PlatformWalletError::TokenError(format!(
                    "Data contract {} not found on Platform",
                    contract_id
                ))
            })?;
        Ok(Arc::new(contract))
    }

    /// Transfer tokens with an external signer. The contract is fetched
    /// internally; the signing key is selected via [`Self::resolve_signing_key`].
    #[allow(clippy::too_many_arguments)]
    pub async fn transfer_external_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        from_identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        to_identity_id: Identifier,
        amount: TokenAmount,
        public_note: Option<String>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::TransferResult, PlatformWalletError> {
        let data_contract = self.fetch_data_contract(token_contract_id).await?;
        let signing_key = self.resolve_signing_key(&from_identity_id).await?;

        self.transfer_with_signer(
            data_contract,
            token_position,
            from_identity_id,
            to_identity_id,
            amount,
            &signing_key,
            signer,
            public_note,
            None,
        )
        .await
        .map_err(|e| PlatformWalletError::TokenError(format!("Token transfer failed: {}", e)))
    }

    /// Burn tokens with an external signer. Contract fetched internally.
    #[allow(clippy::too_many_arguments)]
    pub async fn burn_external_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        amount: TokenAmount,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::BurnResult, PlatformWalletError> {
        let data_contract = self.fetch_data_contract(token_contract_id).await?;
        let signing_key = self.resolve_signing_key(&identity_id).await?;

        self.burn_with_signer(
            data_contract,
            token_position,
            identity_id,
            amount,
            &signing_key,
            signer,
            public_note,
            group_info,
            None,
        )
        .await
        .map_err(|e| PlatformWalletError::TokenError(format!("Token burn failed: {}", e)))
    }

    /// Mint tokens with an external signer. Contract fetched internally.
    /// `recipient_id == None` mints to `identity_id`.
    #[allow(clippy::too_many_arguments)]
    pub async fn mint_external_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        recipient_id: Option<Identifier>,
        amount: TokenAmount,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::MintResult, PlatformWalletError> {
        let data_contract = self.fetch_data_contract(token_contract_id).await?;
        let signing_key = self.resolve_signing_key(&identity_id).await?;

        self.mint_with_signer(
            data_contract,
            token_position,
            identity_id,
            amount,
            recipient_id,
            &signing_key,
            signer,
            public_note,
            group_info,
            None,
        )
        .await
        .map_err(|e| PlatformWalletError::TokenError(format!("Token mint failed: {}", e)))
    }

    /// Claim a distribution with an external signer. Contract fetched internally.
    /// Claim is not group-gated.
    #[allow(clippy::too_many_arguments)]
    pub async fn claim_external_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        distribution_type: dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType,
        public_note: Option<String>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::ClaimResult, PlatformWalletError> {
        let data_contract = self.fetch_data_contract(token_contract_id).await?;
        let signing_key = self.resolve_signing_key(&identity_id).await?;

        self.claim_with_signer(
            data_contract,
            token_position,
            identity_id,
            distribution_type,
            &signing_key,
            signer,
            public_note,
            None,
        )
        .await
        .map_err(|e| PlatformWalletError::TokenError(format!("Token claim failed: {}", e)))
    }

    /// Freeze the entire balance of `frozen_identity_id` for the token at
    /// `token_position` on `token_contract_id`. Contract fetched internally.
    #[allow(clippy::too_many_arguments)]
    pub async fn freeze_external_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        frozen_identity_id: Identifier,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::FreezeResult, PlatformWalletError> {
        let data_contract = self.fetch_data_contract(token_contract_id).await?;
        let signing_key = self.resolve_signing_key(&identity_id).await?;

        self.freeze_with_signer(
            data_contract,
            token_position,
            identity_id,
            frozen_identity_id,
            &signing_key,
            signer,
            public_note,
            group_info,
            None,
        )
        .await
        .map_err(|e| PlatformWalletError::TokenError(format!("Token freeze failed: {}", e)))
    }

    /// Unfreeze the entire frozen balance of `frozen_identity_id` for the
    /// token at `token_position` on `token_contract_id`. Contract fetched
    /// internally.
    #[allow(clippy::too_many_arguments)]
    pub async fn unfreeze_external_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        frozen_identity_id: Identifier,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::UnfreezeResult, PlatformWalletError> {
        let data_contract = self.fetch_data_contract(token_contract_id).await?;
        let signing_key = self.resolve_signing_key(&identity_id).await?;

        self.unfreeze_with_signer(
            data_contract,
            token_position,
            identity_id,
            frozen_identity_id,
            &signing_key,
            signer,
            public_note,
            group_info,
            None,
        )
        .await
        .map_err(|e| PlatformWalletError::TokenError(format!("Token unfreeze failed: {}", e)))
    }

    /// Pause all token operations for the token at `token_position` on
    /// `token_contract_id`. Contract fetched internally. Pause is an
    /// emergency action and targets the token itself — no target identity.
    #[allow(clippy::too_many_arguments)]
    pub async fn pause_external_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::EmergencyActionResult, PlatformWalletError>
    {
        let data_contract = self.fetch_data_contract(token_contract_id).await?;
        let signing_key = self.resolve_signing_key(&identity_id).await?;

        self.pause_with_signer(
            data_contract,
            token_position,
            identity_id,
            &signing_key,
            signer,
            public_note,
            group_info,
            None,
        )
        .await
        .map_err(|e| PlatformWalletError::TokenError(format!("Token pause failed: {}", e)))
    }

    /// Resume all token operations for the token at `token_position` on
    /// `token_contract_id`. Contract fetched internally. Resume is an
    /// emergency action and targets the token itself — no target identity.
    #[allow(clippy::too_many_arguments)]
    pub async fn resume_external_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::EmergencyActionResult, PlatformWalletError>
    {
        let data_contract = self.fetch_data_contract(token_contract_id).await?;
        let signing_key = self.resolve_signing_key(&identity_id).await?;

        self.resume_with_signer(
            data_contract,
            token_position,
            identity_id,
            &signing_key,
            signer,
            public_note,
            group_info,
            None,
        )
        .await
        .map_err(|e| PlatformWalletError::TokenError(format!("Token resume failed: {}", e)))
    }

    /// Destroy the entire frozen balance of `frozen_identity_id` for the
    /// token at `token_position` on `token_contract_id`. Contract fetched
    /// internally. The Rust builder takes no `amount` — the action always
    /// destroys the full frozen balance.
    #[allow(clippy::too_many_arguments)]
    pub async fn destroy_frozen_funds_external_signer<
        S: dpp::identity::signer::Signer<IdentityPublicKey>,
    >(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        frozen_identity_id: Identifier,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<
        dash_sdk::platform::tokens::transitions::DestroyFrozenFundsResult,
        PlatformWalletError,
    > {
        let data_contract = self.fetch_data_contract(token_contract_id).await?;
        let signing_key = self.resolve_signing_key(&identity_id).await?;

        self.destroy_frozen_funds_with_signer(
            data_contract,
            token_position,
            identity_id,
            frozen_identity_id,
            &signing_key,
            signer,
            public_note,
            group_info,
            None,
        )
        .await
        .map_err(|e| {
            PlatformWalletError::TokenError(format!("Token destroy frozen funds failed: {}", e))
        })
    }

    /// Configure the direct-purchase price for the token at
    /// `token_position` on `token_contract_id`. Contract fetched
    /// internally. The simple-form FFI takes a single
    /// `price_per_token`: a non-zero value is lifted into
    /// `TokenPricingSchedule::SinglePrice(price)`, while `0` clears
    /// the schedule (`None` — disables direct purchase).
    ///
    /// The richer `TokenPricingSchedule::SetPrices(BTreeMap<...>)`
    /// (tiered / volume-discount pricing) is intentionally not
    /// surfaced through this entry point; a future helper can add it
    /// without changing the flat-price path.
    #[allow(clippy::too_many_arguments)]
    pub async fn set_price_external_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        price_per_token: u64,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::SetPriceResult, PlatformWalletError> {
        use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

        let data_contract = self.fetch_data_contract(token_contract_id).await?;
        let signing_key = self.resolve_signing_key(&identity_id).await?;

        let pricing_schedule = if price_per_token == 0 {
            None
        } else {
            Some(TokenPricingSchedule::SinglePrice(price_per_token))
        };

        self.set_price_with_signer(
            data_contract,
            token_position,
            identity_id,
            pricing_schedule,
            &signing_key,
            signer,
            public_note,
            group_info,
            None,
        )
        .await
        .map_err(|e| PlatformWalletError::TokenError(format!("Token set price failed: {}", e)))
    }

    /// Purchase `amount` of the token at `token_position` on
    /// `token_contract_id`. Contract fetched internally. Purchase is
    /// not group-gated; the buyer's identity is the only actor.
    ///
    /// `total_agreed_price` is the credits the caller agrees to pay in
    /// total for `amount` tokens — Platform rejects the transition if
    /// the on-chain pricing schedule disagrees, protecting the buyer
    /// from price-change races between fetch and submit.
    #[allow(clippy::too_many_arguments)]
    pub async fn purchase_external_signer<S: dpp::identity::signer::Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        amount: TokenAmount,
        total_agreed_price: dpp::fee::Credits,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::DirectPurchaseResult, PlatformWalletError>
    {
        let data_contract = self.fetch_data_contract(token_contract_id).await?;
        let signing_key = self.resolve_signing_key(&identity_id).await?;

        self.purchase_with_signer(
            data_contract,
            token_position,
            identity_id,
            amount,
            total_agreed_price,
            &signing_key,
            signer,
            None,
        )
        .await
        .map_err(|e| PlatformWalletError::TokenError(format!("Token purchase failed: {}", e)))
    }

    /// Update token configuration with an external signer. Contract is
    /// fetched internally; the signing key is selected via
    /// [`Self::resolve_signing_key`]. `config_change` is the full
    /// `TokenConfigurationChangeItem` enum so this entry point stays
    /// open-ended for the variants the FFI doesn't yet surface.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_config_external_signer<
        S: dpp::identity::signer::Signer<IdentityPublicKey>,
    >(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        config_change: dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::ConfigUpdateResult, PlatformWalletError>
    {
        let data_contract = self.fetch_data_contract(token_contract_id).await?;
        let signing_key = self.resolve_signing_key(&identity_id).await?;

        self.update_config_with_signer(
            data_contract,
            token_position,
            identity_id,
            config_change,
            &signing_key,
            signer,
            public_note,
            group_info,
            None,
        )
        .await
        .map_err(|e| PlatformWalletError::TokenError(format!("Token config update failed: {}", e)))
    }
}

impl std::fmt::Debug for TokenWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenWallet")
            .field("network", &self.sdk.network)
            .finish()
    }
}
