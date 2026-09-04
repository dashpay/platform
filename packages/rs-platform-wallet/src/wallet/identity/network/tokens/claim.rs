//! Token claim — claims a distribution payout for the actor identity.
//! Not group-gated.

use std::sync::Arc;

use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::IdentityWallet;

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Claim a token distribution using a caller-supplied signer + key.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_claim_with_signer<S: Signer<IdentityPublicKey>>(
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

    /// Claim a token distribution with an external signer. Contract
    /// fetched internally. Claim is not group-gated.
    pub async fn token_claim_with_external_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        distribution_type: dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType,
        public_note: Option<String>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::ClaimResult, PlatformWalletError> {
        let data_contract = self.token_fetch_data_contract(token_contract_id).await?;
        let signing_key = self.token_resolve_signing_key(&identity_id).await?;

        self.token_claim_with_signer(
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
        .map_err(|e| {
            // Preserve a structured key-unavailable signer failure so the FFI
            // boundary can still restore code 31; only genuine operation
            // failures get stringified into `TokenError`.
            crate::error::preserve_signer_key_unavailable_or(e, |e| {
                PlatformWalletError::TokenError(format!("Token claim failed: {}", e))
            })
        })
    }
}
