//! Token claiming operations for the Dash Platform SDK.
//!
//! This module provides functionality to claim ownership of tokens that have
//! been allocated or made available for claiming.

use crate::platform::tokens::builders::claim::TokenClaimTransitionBuilder;
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::{Error, Sdk};
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::proof_result::StateTransitionProofResult;

/// Result types returned from claiming token operations.
///
/// This enum represents the different possible outcomes when claiming tokens,
/// depending on whether it's a standard claim or a group action.
pub enum ClaimResult {
    /// Standard claim result containing the claim document.
    Document(Document),
    /// Group-based claim action with document and group power information.
    GroupActionWithDocument(GroupSumPower, Document),
}

impl Sdk {
    /// Claims tokens for a specific identity.
    ///
    /// This method broadcasts a claim transition to acquire ownership of tokens
    /// that have been allocated or made available for claiming. The result
    /// includes a document that records the claim details.
    ///
    /// # Arguments
    ///
    /// * `claim_tokens_transition_builder` - Builder containing claim parameters
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `ClaimResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    /// - A group action result is missing the expected document
    pub async fn token_claim<S: Signer>(
        &self,
        claim_tokens_transition_builder: TokenClaimTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<ClaimResult, Error> {
        let platform_version = self.version();

        let put_settings = claim_tokens_transition_builder.settings;

        let state_transition = claim_tokens_transition_builder
            .sign(self, signing_key, signer, platform_version)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, put_settings)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenActionWithDocument(document) => {
                Ok(ClaimResult::Document(document))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, Some(document)) => {
                Ok(ClaimResult::GroupActionWithDocument(power, document))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(_, None) => {
                Err(Error::DriveProofError(
                    drive::error::proof::ProofError::IncorrectProof(
                        "Expected document in group action result".to_string(),
                    ),
                    vec![],
                    Default::default(),
                ))
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenActionWithDocument or VerifiedTokenGroupActionWithDocument for claim transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
