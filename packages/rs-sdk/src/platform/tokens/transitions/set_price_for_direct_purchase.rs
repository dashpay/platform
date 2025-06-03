//! Token price setting operations for direct purchase functionality.
//!
//! This module provides functionality to set or update pricing schedules
//! for tokens that can be purchased directly.

use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::fungible_tokens::set_price::TokenChangeDirectPurchasePriceTransitionBuilder;
use crate::{Error, Sdk};
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

/// Result types returned from setting token price operations.
///
/// This enum represents the different possible outcomes when setting token prices,
/// depending on the token configuration and whether it's a group action.
pub enum SetPriceResult {
    /// Standard price setting result containing owner ID and pricing schedule.
    PricingSchedule(Identifier, Option<TokenPricingSchedule>),
    /// Price setting result with historical tracking via document storage.
    HistoricalDocument(Document),
    /// Group-based price setting action with optional document for history.
    GroupActionWithDocument(GroupSumPower, Option<Document>),
    /// Group-based price setting action with pricing schedule and status.
    GroupActionWithPricingSchedule(
        GroupSumPower,
        GroupActionStatus,
        Option<TokenPricingSchedule>,
    ),
}

impl Sdk {
    /// Sets or updates the pricing schedule for direct token purchases.
    ///
    /// This method broadcasts a set price transition to define how tokens can be
    /// purchased directly. The pricing schedule can include fixed prices or
    /// dynamic pricing rules. The result varies based on token configuration:
    /// - Standard tokens return the pricing schedule
    /// - Tokens with history tracking return documents
    /// - Group-managed tokens include group power and action status
    ///
    /// # Arguments
    ///
    /// * `set_price_transition_builder` - Builder containing pricing parameters
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `SetPriceResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    pub async fn token_set_price_for_direct_purchase<S: Signer>(
        &self,
        set_price_transition_builder: TokenChangeDirectPurchasePriceTransitionBuilder<'_>,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<SetPriceResult, Error> {
        let platform_version = self.version();

        let state_transition = set_price_transition_builder
            .sign(self, signing_key, signer, platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenPricingSchedule(owner_id, schedule) => {
                Ok(SetPriceResult::PricingSchedule(owner_id, schedule))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                Ok(SetPriceResult::HistoricalDocument(doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                Ok(SetPriceResult::GroupActionWithDocument(power, doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenPricingSchedule(power, status, schedule) => {
                Ok(SetPriceResult::GroupActionWithPricingSchedule(power, status, schedule))
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenPricingSchedule, VerifiedTokenActionWithDocument, VerifiedTokenGroupActionWithDocument, or VerifiedTokenGroupActionWithTokenPricingSchedule for set price transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
