//! Token update-config — applies a single
//! `TokenConfigurationChangeItem` mutation to the token's on-contract
//! config. Group-gateable. The variant enum stays open-ended so this
//! entry point doesn't have to grow a new method per setting.

use std::sync::Arc;

use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::IdentityWallet;

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Update config with a caller-supplied signer + key.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_update_config_with_signer<S: Signer<IdentityPublicKey>>(
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

    /// Update config with an external signer. Contract fetched
    /// internally; `config_change` is the full enum so this entry
    /// point stays open-ended for variants the FFI doesn't yet
    /// surface.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_update_config_with_external_signer<S: Signer<IdentityPublicKey>>(
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
        let data_contract = self.token_fetch_data_contract(token_contract_id).await?;
        let signing_key = self.token_resolve_signing_key(&identity_id).await?;

        self.token_update_config_with_signer(
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
        .map_err(|e| {
            // Preserve a structured key-unavailable signer failure so the FFI
            // boundary can still restore code 31; only genuine operation
            // failures get stringified into `TokenError`.
            crate::error::preserve_signer_key_unavailable_or(e, |e| {
                PlatformWalletError::TokenError(format!("Token config update failed: {}", e))
            })
        })
    }
}
