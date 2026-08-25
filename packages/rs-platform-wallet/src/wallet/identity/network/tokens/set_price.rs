//! Token set-price — configures the direct-purchase pricing schedule
//! for `(contract, token_position)`. Group-gateable.
//!
//! The simple-form FFI takes a single `price_per_token`: a non-zero
//! value is lifted into `TokenPricingSchedule::SinglePrice(price)`,
//! while `0` clears the schedule (`None` — disables direct purchase).
//! The richer `TokenPricingSchedule::SetPrices` variant (tiered /
//! volume-discount pricing) is intentionally not surfaced through this
//! entry point yet.

use std::sync::Arc;

use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::IdentityWallet;

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Set price with a caller-supplied signer + key. Accepts the full
    /// `TokenPricingSchedule` so callers can express tiered pricing.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_set_price_with_signer<S: Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        token_pricing_schedule: Option<dpp::tokens::token_pricing_schedule::TokenPricingSchedule>,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<dash_sdk::platform::tokens::transitions::SetPriceResult, dash_sdk::Error> {
        use dash_sdk::platform::tokens::builders::set_price::TokenChangeDirectPurchasePriceTransitionBuilder;

        let mut builder = TokenChangeDirectPurchasePriceTransitionBuilder::new(
            data_contract,
            token_position,
            identity_id,
        );

        if let Some(pricing_schedule) = token_pricing_schedule {
            builder = builder.with_token_pricing_schedule(pricing_schedule);
        }

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
            .token_set_price_for_direct_purchase(builder, signing_key, signer)
            .await
    }

    /// Set price with an external signer. Contract fetched internally.
    /// `price_per_token == 0` clears the schedule (disables direct
    /// purchase); any other value lifts to `SinglePrice`.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_set_price_with_external_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        price_per_token: u64,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::SetPriceResult, PlatformWalletError> {
        use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

        let data_contract = self.token_fetch_data_contract(token_contract_id).await?;
        let signing_key = self.token_resolve_signing_key(&identity_id).await?;

        let pricing_schedule = if price_per_token == 0 {
            None
        } else {
            Some(TokenPricingSchedule::SinglePrice(price_per_token))
        };

        self.token_set_price_with_signer(
            data_contract,
            token_position,
            identity_id,
            pricing_schedule,
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
                PlatformWalletError::TokenError(format!("Token set price failed: {}", e))
            })
        })
    }
}
