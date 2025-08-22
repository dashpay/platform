//! Token frozen funds destruction operations for the Dash Platform SDK.
//!
//! This module provides functionality to permanently destroy frozen tokens,
//! removing them from circulation entirely.

use crate::platform::tokens::builders::destroy::TokenDestroyFrozenFundsTransitionBuilder;
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::{Error, Sdk};
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::proof_result::StateTransitionProofResult;

/// Result types returned from destroying frozen funds operations.
///
/// This enum represents the different possible outcomes when destroying frozen funds,
/// always including historical tracking.
pub enum DestroyFrozenFundsResult {
    /// Destruction result with historical tracking via document storage.
    HistoricalDocument(Document),
    /// Group-based destruction action with optional document for history.
    GroupActionWithDocument(GroupSumPower, Option<Document>),
}

impl Sdk {
    /// Destroys frozen tokens permanently.
    ///
    /// This method broadcasts a destroy frozen funds transition to permanently
    /// remove frozen tokens from circulation. This operation always maintains
    /// a historical record of the destruction.
    ///
    /// # Arguments
    ///
    /// * `destroy_frozen_funds_transition_builder` - Builder containing destruction parameters
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `DestroyFrozenFundsResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    pub async fn token_destroy_frozen_funds<S: Signer>(
        &self,
        destroy_frozen_funds_transition_builder: TokenDestroyFrozenFundsTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DestroyFrozenFundsResult, Error> {
        let platform_version = self.version();

        let put_settings = destroy_frozen_funds_transition_builder.settings;

        let state_transition = destroy_frozen_funds_transition_builder
            .sign(self, signing_key, signer, platform_version)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, put_settings)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                // DestroyFrozenFunds always keeps history
                Ok(DestroyFrozenFundsResult::HistoricalDocument(doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                // Group action with history
                Ok(DestroyFrozenFundsResult::GroupActionWithDocument(power, doc))
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenActionWithDocument or VerifiedTokenGroupActionWithDocument for destroy frozen funds transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
