//! Token mint — issues `amount` of `(contract, token_position)`.
//! `recipient_id == None` defers to the contract's
//! `newTokensDestinationIdentity`. When the contract sets
//! `minting_allow_choosing_destination = false`, this helper drops any
//! non-`None` `recipient_id` to keep co-sign replays from tripping the
//! chain-side `ChoosingTokenMintRecipientNotAllowed` validator.
//! Group-gateable.

use std::sync::Arc;

use dpp::balances::credits::TokenAmount;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::IdentityWallet;

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
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

        // Contracts with `minting_allow_choosing_destination = false`
        // reject any mint where `issued_to_identity_id.is_some()` at
        // execution time (rs-drive's TokenMintTransitionTransformer
        // rule). The proposer-mode submission is stored as a pending
        // action and bypasses this check, but the chain stores
        // `TokenEvent::Mint` with the resolved
        // `newTokensDestinationIdentity` baked in. The co-sign path
        // then surfaces that resolved id back to the caller as a
        // non-optional recipient and replays it, tripping the
        // validator. Normalize here so neither caller has to think
        // about it: when the rule forbids choosing, drop the
        // recipient and let the chain resolve to
        // `newTokensDestinationIdentity` (or surface
        // `DestinationIdentityForTokenMintingNotSet` if that's also
        // unset).
        let recipient_id = if !data_contract
            .expected_token_configuration(token_position)
            .map_err(dash_sdk::Error::Protocol)?
            .distribution_rules()
            .minting_allow_choosing_destination()
        {
            // Surface the silent override in logs so downstream
            // debugging doesn't have to reverse-engineer "the helper
            // ignored my recipient" from a successful mint to
            // `newTokensDestinationIdentity`. Only emit when we
            // actually changed something — a pre-existing `None`
            // is a no-op.
            if let Some(supplied) = recipient_id.as_ref() {
                tracing::debug!(
                    supplied = %bs58::encode(supplied.to_buffer()).into_string(),
                    token_position,
                    "token_mint normalizing caller-supplied recipient_id to None: contract forbids choosing destination"
                );
            }
            None
        } else {
            recipient_id
        };

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
