//! Token burning operations for the Dash Platform SDK.
//!
//! This module provides functionality to permanently remove tokens from
//! circulation by burning them.

use crate::platform::tokens::builders::burn::TokenBurnTransitionBuilder;
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::{Error, Sdk};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;

/// Result types returned from burning token operations.
///
/// This enum represents the different possible outcomes when burning tokens,
/// depending on the token configuration and whether it's a group action.
pub enum BurnResult {
    /// Standard burn result containing owner ID and remaining balance.
    TokenBalance(Identifier, TokenAmount),
    /// Burn result with historical tracking via document storage.
    HistoricalDocument(Document),
    /// Group-based burn action with optional document for history.
    GroupActionWithDocument(GroupSumPower, Option<Document>),
    /// Group-based burn action with balance and status information.
    GroupActionWithBalance(GroupSumPower, GroupActionStatus, Option<TokenAmount>),
}

impl Sdk {
    /// Burns tokens to permanently remove them from circulation.
    ///
    /// This method broadcasts a burn transition to destroy a specified amount
    /// of tokens. The result varies based on token configuration:
    /// - Standard tokens return the owner's remaining balance
    /// - Tokens with history tracking return documents
    /// - Group-managed tokens include group power and action status
    ///
    /// # Arguments
    ///
    /// * `burn_tokens_transition_builder` - Builder containing burn parameters including amount
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `BurnResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    /// - Insufficient token balance for burning
    pub async fn token_burn<S: Signer>(
        &self,
        burn_tokens_transition_builder: TokenBurnTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<BurnResult, Error> {
        let platform_version = self.version();

        let put_settings = burn_tokens_transition_builder.settings;

        let state_transition = burn_tokens_transition_builder
            .sign(self, signing_key, signer, platform_version)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, put_settings)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenBalance(owner_id, remaining_balance) => {
                Ok(BurnResult::TokenBalance(owner_id, remaining_balance))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                // This means the token keeps burning history
                Ok(BurnResult::HistoricalDocument(doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                // This means it's a group action with history
                Ok(BurnResult::GroupActionWithDocument(power, doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenBalance(power, status, balance) => {
                // Group action without history
                Ok(BurnResult::GroupActionWithBalance(power, status, balance))
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenBalance, VerifiedTokenActionWithDocument, VerifiedTokenGroupActionWithDocument, or VerifiedTokenGroupActionWithTokenBalance for burn transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
