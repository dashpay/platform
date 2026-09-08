//! Token purchase — buy `amount` of `(contract, token_position)` at
//! the on-chain pricing schedule. Not group-gated; the buyer's
//! identity is the only actor.
//!
//! `total_agreed_price` is the credits the caller agrees to pay in
//! total for `amount` tokens — Platform rejects the transition if the
//! on-chain pricing schedule disagrees, protecting the buyer from
//! price-change races between fetch and submit.

use std::sync::Arc;

use dpp::balances::credits::TokenAmount;
use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::IdentityWallet;

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Purchase with a caller-supplied signer + key.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_purchase_with_signer<S: Signer<IdentityPublicKey>>(
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

    /// Purchase with an external signer. Contract fetched internally.
    pub async fn token_purchase_with_external_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        amount: TokenAmount,
        total_agreed_price: dpp::fee::Credits,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::DirectPurchaseResult, PlatformWalletError>
    {
        let data_contract = self.token_fetch_data_contract(token_contract_id).await?;
        let signing_key = self.token_resolve_signing_key(&identity_id).await?;

        self.token_purchase_with_signer(
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
        .map_err(|e| {
            // Preserve a structured key-unavailable signer failure so the FFI
            // boundary can still restore code 31; only genuine operation
            // failures get stringified into `TokenError`.
            crate::error::preserve_signer_key_unavailable_or(e, |e| {
                PlatformWalletError::TokenError(format!("Token purchase failed: {}", e))
            })
        })
    }
}
