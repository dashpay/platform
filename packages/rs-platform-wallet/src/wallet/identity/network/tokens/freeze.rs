//! Token freeze — freezes a target identity's full balance for a
//! given `(contract, token_position)`. Group-gateable.

use std::sync::Arc;

use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::IdentityWallet;

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Freeze with a caller-supplied signer + key.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_freeze_with_signer<S: Signer<IdentityPublicKey>>(
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

    /// Freeze with an external signer. Contract fetched internally.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_freeze_with_external_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        frozen_identity_id: Identifier,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::FreezeResult, PlatformWalletError> {
        let data_contract = self.token_fetch_data_contract(token_contract_id).await?;
        let signing_key = self.token_resolve_signing_key(&identity_id).await?;

        self.token_freeze_with_signer(
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
}
