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
/// configuration, depending on whether it's a group action.
pub enum ConfigUpdateResult {
    /// Standard configuration update result containing the updated configuration document.
    Document(Document),
    /// Group-based configuration update action with group power information.
    ///
    /// The document is `None` while the group action is still active (a proposal,
    /// or a co-signature that does not close the action); it is present once the
    /// action closes and the update is applied.
    GroupActionWithDocument(GroupSumPower, Option<Document>),
}

impl Sdk {
    /// Updates the configuration of a token contract.
    ///
    /// This method broadcasts a configuration update transition to modify token
    /// settings such as minting rules, transfer restrictions, or other contract
    /// parameters. The result includes a document with the updated
    /// configuration details, except for a group action that is still active.
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
    pub async fn token_update_contract_token_configuration<S: Signer<IdentityPublicKey>>(
        &self,
        config_update_transition_builder: TokenConfigUpdateTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<ConfigUpdateResult, Error> {
        let platform_version = self.version();

        let put_settings = config_update_transition_builder.settings;

        let state_transition = config_update_transition_builder
            .sign(self, signing_key, signer, platform_version)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, put_settings)
            .await?;

        ConfigUpdateResult::try_from(proof_result)
    }
}

impl TryFrom<StateTransitionProofResult> for ConfigUpdateResult {
    type Error = Error;

    fn try_from(proof_result: StateTransitionProofResult) -> Result<Self, Self::Error> {
        match proof_result {
            StateTransitionProofResult::VerifiedTokenActionWithDocument(document) => {
                Ok(ConfigUpdateResult::Document(document))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, document) => {
                // The document is absent while the group action is still active
                Ok(ConfigUpdateResult::GroupActionWithDocument(power, document))
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::document::DocumentV0;

    #[test]
    fn should_forward_active_group_config_update_without_document() {
        // Drive proves a still-active group action (a proposal, or a co-signature
        // that does not reach the threshold) with no historical document.
        let result = ConfigUpdateResult::try_from(
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(5, None),
        )
        .expect("an active group action is a verified result, not a proof error");
        assert!(matches!(
            result,
            ConfigUpdateResult::GroupActionWithDocument(5, None)
        ));
    }

    #[test]
    fn should_forward_closed_group_config_update_with_document() {
        let result = ConfigUpdateResult::try_from(
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(
                7,
                Some(Document::V0(DocumentV0::default())),
            ),
        )
        .expect("convert verified closed group action");
        assert!(matches!(
            result,
            ConfigUpdateResult::GroupActionWithDocument(7, Some(_))
        ));
    }

    #[test]
    fn should_reject_a_proof_result_of_another_transition() {
        let result = ConfigUpdateResult::try_from(
            StateTransitionProofResult::VerifiedTokenBalance(Default::default(), 1),
        );
        assert!(matches!(
            result,
            Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(_),
                _,
                _
            ))
        ));
    }
}
