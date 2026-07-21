//! Shared lookups for token state-transition methods on
//! [`IdentityWallet`]. All token actions need to (a) fetch the data
//! contract by id and (b) resolve the canonical AUTHENTICATION /
//! CRITICAL / ECDSA_SECP256K1 signing key on the actor identity,
//! so those steps live here rather than being copy-pasted across each
//! action file.
//!
//! `CRITICAL` is non-negotiable: rs-dpp's
//! `combined_security_level_requirement` collapses the allowed key
//! set for *any* batch containing a token transition to
//! `[SecurityLevel::CRITICAL]` only — MASTER, HIGH, and MEDIUM are
//! all rejected by Drive. See
//! `state_transitions/document/batch_transition/methods/v0/mod.rs:133-138`.

use std::sync::Arc;

use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::prelude::Identifier;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::IdentityWallet;

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Resolve the AUTHENTICATION signing key for token state
    /// transitions — used by the `_with_external_signer` variants
    /// where the caller brings the `Signer` and we just need to pick
    /// which on-chain key id to sign with.
    ///
    /// Key selection is a fixed canonical policy:
    /// `Purpose::AUTHENTICATION` × `SecurityLevel::CRITICAL` × `ECDSA_SECP256K1`.
    /// Callers that pass a `signing_key_id` over FFI are advisory —
    /// this Rust path is authoritative and Drive validates the chosen
    /// key matches what the state transition needs.
    pub(super) async fn token_resolve_signing_key(
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
                [SecurityLevel::CRITICAL].into(),
                [KeyType::ECDSA_SECP256K1].into(),
                false,
            )
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "No AUTHENTICATION ECDSA_SECP256K1 key at CRITICAL security level on identity {} — \
                     identities registered before this fix may need an IdentityUpdate to add a CRITICAL key, \
                     or re-registration",
                    identity_id
                ))
            })?
            .clone();

        Ok(signing_key)
    }

    /// Fetch a data contract by id from Platform.
    pub(super) async fn token_fetch_data_contract(
        &self,
        contract_id: Identifier,
    ) -> Result<Arc<DataContract>, PlatformWalletError> {
        use dash_sdk::platform::{ContextProvider, Fetch};

        let contract = DataContract::fetch(&self.sdk, contract_id)
            .await
            .map_err(|e| PlatformWalletError::TokenError(format!("Fetch contract failed: {}", e)))?
            .ok_or_else(|| {
                PlatformWalletError::TokenError(format!(
                    "Data contract {} not found on Platform",
                    contract_id
                ))
            })?;
        let contract = Arc::new(contract);
        // Register the fetched contract with the SDK context provider so the
        // returned-proof verification after a token state-transition broadcast
        // can resolve it. Mirrors document.rs's
        // `register_contract_for_proof_verification`, done here at the shared
        // token fetch point so every write path (mint / burn / transfer /
        // purchase / freeze / …) is covered. The mobile
        // `TrustedHttpContextProvider` never fetches contracts itself, so
        // without this the returned-proof verification fails with "unknown
        // contract" even though the token write already landed on-chain.
        if let Some(provider) = self.sdk.context_provider() {
            provider.register_data_contract(Arc::clone(&contract));
        }
        Ok(contract)
    }
}
