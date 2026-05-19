//! Token transfer — moves `amount` of `(contract, token_position)`
//! from one identity to another. Not group-gated.

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
    /// Transfer tokens using a caller-supplied signer + key. Returns
    /// the SDK's `TransferResult` so the caller can inspect the
    /// proof-verified outcome.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_transfer_with_signer<S: Signer<IdentityPublicKey>>(
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

    /// Transfer tokens using an external signer. The contract is
    /// fetched internally and the signing key is selected via the
    /// shared `token_resolve_signing_key` helper.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_transfer_with_external_signer<S: Signer<IdentityPublicKey>>(
        &self,
        from_identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        to_identity_id: Identifier,
        amount: TokenAmount,
        public_note: Option<String>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::TransferResult, PlatformWalletError> {
        let data_contract = self.token_fetch_data_contract(token_contract_id).await?;
        let signing_key = self.token_resolve_signing_key(&from_identity_id).await?;

        self.token_transfer_with_signer(
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
}
