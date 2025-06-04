//! Emergency action operations for token management.
//!
//! This module provides functionality for executing emergency actions on tokens,
//! typically requiring group authorization for critical interventions.

use crate::platform::tokens::builders::emergency_action::TokenEmergencyActionTransitionBuilder;
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::{Error, Sdk};
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::proof_result::StateTransitionProofResult;

/// Result type returned from emergency action operations.
///
/// Emergency actions always require group authorization and may include
/// a document recording the action details.
pub enum EmergencyActionResult {
    /// Emergency action result with group power and optional document for history.
    GroupActionWithDocument(GroupSumPower, Option<Document>),
}

impl Sdk {
    /// Executes an emergency action on a token contract.
    ///
    /// This method broadcasts an emergency action transition for critical token
    /// management operations that require group authorization. Emergency actions
    /// are typically used for scenarios like halting trading, recovering from
    /// critical bugs, or other urgent interventions.
    ///
    /// # Arguments
    ///
    /// * `emergency_action_transition_builder` - Builder containing emergency action parameters
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing an `EmergencyActionResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    /// - The group authorization is insufficient
    pub async fn token_emergency_action<S: Signer>(
        &self,
        emergency_action_transition_builder: TokenEmergencyActionTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<EmergencyActionResult, Error> {
        let platform_version = self.version();

        let state_transition = emergency_action_transition_builder
            .sign(self, signing_key, signer, platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(
                group_power,
                document,
            ) => Ok(EmergencyActionResult::GroupActionWithDocument(
                group_power,
                document,
            )),
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenGroupActionWithDocument for emergency action transition"
                        .to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
