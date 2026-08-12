//! Token pause — emergency action that halts all transitions on a
//! `(contract, token_position)`. Group-gateable. No target identity —
//! the action targets the token itself.

use std::sync::Arc;

use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::IdentityWallet;

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Pause with a caller-supplied signer + key.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_pause_with_signer<S: Signer<IdentityPublicKey>>(
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

    /// Pause with an external signer. Contract fetched internally.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_pause_with_external_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::EmergencyActionResult, PlatformWalletError>
    {
        let data_contract = self.token_fetch_data_contract(token_contract_id).await?;
        let signing_key = self.token_resolve_signing_key(&identity_id).await?;

        self.token_pause_with_signer(
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
        .map_err(|e| {
            // Preserve a structured key-unavailable signer failure so the FFI
            // boundary can still restore code 31; only genuine operation
            // failures get stringified into `TokenError`
            // (dashpay/platform#4183 review).
            crate::error::preserve_signer_key_unavailable_or(e, |e| {
                PlatformWalletError::TokenError(format!("Token pause failed: {}", e))
            })
        })
    }
}
