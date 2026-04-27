//! Token mint — issues `amount` of `(contract, token_position)`.
//! `recipient_id == None` mints to `identity_id`. Group-gateable.

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
    /// Mint tokens using the wallet's internal signer.
    pub async fn token_mint(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: &Identifier,
        amount: TokenAmount,
        recipient_id: Option<Identifier>,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::tokens::builders::mint::TokenMintTransitionBuilder;

        let (_identity, signer, signing_key) =
            self.token_resolve_identity_and_signer(identity_id).await?;

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

    /// Mint tokens using a caller-supplied signer + key.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_mint_with_signer<S: Signer<IdentityPublicKey>>(
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

    /// Mint tokens with an external signer. Contract fetched internally.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_mint_with_external_signer<S: Signer<IdentityPublicKey>>(
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
        let data_contract = self.token_fetch_data_contract(token_contract_id).await?;
        let signing_key = self.token_resolve_signing_key(&identity_id).await?;

        self.token_mint_with_signer(
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
}
