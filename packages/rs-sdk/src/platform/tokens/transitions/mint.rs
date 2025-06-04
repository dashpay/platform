//! Token minting operations for the Dash Platform SDK.
//!
//! This module provides functionality to mint new tokens, increasing the
//! total supply of a token according to its configured minting rules.

use crate::platform::tokens::builders::mint::TokenMintTransitionBuilder;
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

/// Result types returned from minting token operations.
///
/// This enum represents the different possible outcomes when minting tokens,
/// depending on the token configuration and whether it's a group action.
pub enum MintResult {
    /// Standard mint result containing recipient identity and new balance.
    TokenBalance(Identifier, TokenAmount),
    /// Mint result with historical tracking via document storage.
    HistoricalDocument(Document),
    /// Group-based mint action with optional document for history.
    GroupActionWithDocument(GroupSumPower, Option<Document>),
    /// Group-based mint action with balance and status information.
    GroupActionWithBalance(GroupSumPower, GroupActionStatus, Option<TokenAmount>),
}

impl Sdk {
    /// Mints new tokens according to the token's configuration.
    ///
    /// This method broadcasts a mint transition to create new tokens and allocate
    /// them to the specified recipient. The result varies based on token configuration:
    /// - Standard tokens return the recipient's new balance
    /// - Tokens with history tracking return documents
    /// - Group-managed tokens include group power and action status
    ///
    /// # Arguments
    ///
    /// * `mint_tokens_transition_builder` - Builder containing mint parameters including amount and recipient
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `MintResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    pub async fn token_mint<S: Signer>(
        &self,
        mint_tokens_transition_builder: TokenMintTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<MintResult, Error> {
        let platform_version = self.version();

        let state_transition = mint_tokens_transition_builder
            .sign(self, signing_key, signer, platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenBalance(recipient_id_result, new_balance) => {
                Ok(MintResult::TokenBalance(recipient_id_result, new_balance))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                // This means the token keeps minting history
                Ok(MintResult::HistoricalDocument(doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                // This means it's a group action with history
                Ok(MintResult::GroupActionWithDocument(power, doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenBalance(power, status, balance) => {
                // Group action without history
                Ok(MintResult::GroupActionWithBalance(power, status, balance))
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenBalance, VerifiedTokenActionWithDocument, VerifiedTokenGroupActionWithDocument, or VerifiedTokenGroupActionWithTokenBalance for mint transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
