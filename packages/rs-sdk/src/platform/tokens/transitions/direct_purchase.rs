//! Direct token purchase operations for the Dash Platform SDK.
//!
//! This module provides functionality to purchase tokens directly at
//! previously set prices.

use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::fungible_tokens::purchase::TokenDirectPurchaseTransitionBuilder;
use crate::{Error, Sdk};
use dpp::balances::credits::TokenAmount;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;

/// Result types returned from direct token purchase operations.
///
/// This enum represents the different possible outcomes when purchasing tokens,
/// depending on the token configuration and whether it's a group action.
pub enum DirectPurchaseResult {
    /// Standard purchase result containing purchaser ID and new balance.
    TokenBalance(Identifier, TokenAmount),
    /// Purchase result with historical tracking via document storage.
    HistoricalDocument(dpp::document::Document),
    /// Group-based purchase action with optional document for history.
    GroupActionWithDocument(
        dpp::data_contract::group::GroupSumPower,
        Option<dpp::document::Document>,
    ),
}

impl Sdk {
    /// Purchases tokens directly at the configured price.
    ///
    /// This method broadcasts a direct purchase transition to buy tokens at
    /// the price set by the token owner. The purchase uses platform credits
    /// as payment. The result varies based on token configuration:
    /// - Standard tokens return the purchaser's new balance
    /// - Tokens with history tracking return documents
    /// - Group-managed tokens include group power information
    ///
    /// # Arguments
    ///
    /// * `purchase_tokens_transition_builder` - Builder containing purchase parameters including amount
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `DirectPurchaseResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    /// - Insufficient credits for the purchase
    pub async fn token_purchase<S: Signer>(
        &self,
        purchase_tokens_transition_builder: TokenDirectPurchaseTransitionBuilder<'_>,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DirectPurchaseResult, Error> {
        let platform_version = self.version();

        let state_transition = purchase_tokens_transition_builder
            .sign(self, signing_key, signer, platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenBalance(owner_id, balance) => {
                Ok(DirectPurchaseResult::TokenBalance(owner_id, balance))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                Ok(DirectPurchaseResult::HistoricalDocument(doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                Ok(DirectPurchaseResult::GroupActionWithDocument(power, doc))
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenBalance, VerifiedTokenActionWithDocument, or VerifiedTokenGroupActionWithDocument for direct purchase transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
