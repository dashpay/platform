//! Token configuration update operations for the Dash Platform SDK.
//!
//! This module provides functionality to update token contract configuration
//! settings after initial deployment.

use crate::platform::tokens::builders::config_update::TokenConfigUpdateTransitionBuilder;
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::{Error, Sdk};
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::proof_result::StateTransitionProofResult;

/// Result types returned from token configuration update operations.
///
/// This enum represents the different possible outcomes when updating token
/// configuration, always returning a document with the updated settings.
pub enum ConfigUpdateResult {
    /// Standard configuration update result containing the updated configuration document.
    Document(Document),
    /// Group-based configuration update action with document and group power information.
    GroupActionWithDocument(GroupSumPower, Document),
}

impl Sdk {
    /// Updates the configuration of a token contract.
    ///
    /// This method broadcasts a configuration update transition to modify token
    /// settings such as minting rules, transfer restrictions, or other contract
    /// parameters. The result always includes a document with the updated
    /// configuration details.
    ///
    /// # Arguments
    ///
    /// * `config_update_transition_builder` - Builder containing configuration update parameters
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `ConfigUpdateResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    /// - A group action result is missing the expected document
    pub async fn token_update_contract_token_configuration<S: Signer>(
        &self,
        config_update_transition_builder: TokenConfigUpdateTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<ConfigUpdateResult, Error> {
        let platform_version = self.version();

        let state_transition = config_update_transition_builder
            .sign(self, signing_key, signer, platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenActionWithDocument(document) => {
                Ok(ConfigUpdateResult::Document(document))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, Some(document)) => {
                Ok(ConfigUpdateResult::GroupActionWithDocument(power, document))
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
                    "Expected VerifiedTokenActionWithDocument or VerifiedTokenGroupActionWithDocument for config update transition"
                        .to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
