//! Token unfreezing operations for the Dash Platform SDK.
//!
//! This module provides functionality to unfreeze previously frozen tokens,
//! restoring their transfer capabilities.

use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::fungible_tokens::unfreeze::TokenUnfreezeTransitionBuilder;
use crate::{Error, Sdk};
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::tokens::info::IdentityTokenInfo;

/// Result types returned from unfreezing token operations.
///
/// This enum represents the different possible outcomes when unfreezing tokens,
/// depending on the token configuration and whether it's a group action.
pub enum UnfreezeResult {
    /// Standard unfreeze result containing identity and token information.
    IdentityInfo(Identifier, IdentityTokenInfo),
    /// Unfreeze result with historical tracking via document storage.
    HistoricalDocument(Document),
    /// Group-based unfreeze action with optional document for history.
    GroupActionWithDocument(GroupSumPower, Option<Document>),
    /// Group-based unfreeze action with identity token information.
    GroupActionWithIdentityInfo(GroupSumPower, IdentityTokenInfo),
}

impl Sdk {
    /// Unfreezes tokens for a specific identity.
    ///
    /// This method broadcasts an unfreeze transition to restore transfer capabilities
    /// for previously frozen tokens. The result varies based on token configuration:
    /// - Standard tokens return identity info
    /// - Tokens with history tracking return documents
    /// - Group-managed tokens include group power information
    ///
    /// # Arguments
    ///
    /// * `unfreeze_tokens_transition_builder` - Builder containing unfreeze parameters
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing an `UnfreezeResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    pub async fn token_unfreeze_identity<S: Signer>(
        &self,
        unfreeze_tokens_transition_builder: TokenUnfreezeTransitionBuilder<'_>,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<UnfreezeResult, Error> {
        let platform_version = self.version();

        let state_transition = unfreeze_tokens_transition_builder
            .sign(self, signing_key, signer, platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenIdentityInfo(owner_id_result, info) => {
                Ok(UnfreezeResult::IdentityInfo(owner_id_result, info))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                // This means the token keeps freezing history
                Ok(UnfreezeResult::HistoricalDocument(doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                // This means it's a group action with history
                Ok(UnfreezeResult::GroupActionWithDocument(power, doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenIdentityInfo(power, _, Some(info)) => {
                // Group action without history
                Ok(UnfreezeResult::GroupActionWithIdentityInfo(power, info))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenIdentityInfo(_, _, None) => {
                Err(Error::DriveProofError(
                    drive::error::proof::ProofError::IncorrectProof(
                        "Expected token identity info in group action result".to_string(),
                    ),
                    vec![],
                    Default::default(),
                ))
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenIdentityInfo, VerifiedTokenActionWithDocument, or VerifiedTokenGroupActionWithTokenIdentityInfo for unfreeze transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
