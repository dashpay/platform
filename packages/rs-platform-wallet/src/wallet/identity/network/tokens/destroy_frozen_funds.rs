//! Token destroy-frozen-funds — destroys the entire frozen balance of
//! a target identity for `(contract, token_position)`. Group-gateable.
//! The Rust builder takes no `amount` — the action always destroys the
//! full frozen balance.

use std::sync::Arc;

use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::IdentityWallet;

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Destroy frozen funds with a caller-supplied signer + key.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_destroy_frozen_funds_with_signer<S: Signer<IdentityPublicKey>>(
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

    /// Destroy frozen funds with an external signer. Contract fetched
    /// internally.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_destroy_frozen_funds_with_external_signer<S: Signer<IdentityPublicKey>>(
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
        let data_contract = self.token_fetch_data_contract(token_contract_id).await?;
        let signing_key = self.token_resolve_signing_key(&identity_id).await?;

        self.token_destroy_frozen_funds_with_signer(
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
            // Preserve a structured key-unavailable signer failure so the FFI
            // boundary can still restore code 31; only genuine operation
            // failures get stringified into `TokenError`.
            crate::error::preserve_signer_key_unavailable_or(e, |e| {
                PlatformWalletError::TokenError(format!("Token destroy frozen funds failed: {}", e))
            })
        })
    }
}
