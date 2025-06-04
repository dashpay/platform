//! Token transfer operations for the Dash Platform SDK.
//!
//! This module provides functionality to transfer tokens between identities
//! on the Dash Platform.

use crate::platform::tokens::builders::transfer::TokenTransferTransitionBuilder;
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::{Error, Sdk};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use std::collections::BTreeMap;

/// Result types returned from token transfer operations.
///
/// This enum represents the different possible outcomes when transferring tokens,
/// depending on the token configuration and whether it's a group action.
pub enum TransferResult {
    /// Standard transfer result containing updated balances for affected identities.
    IdentitiesBalances(BTreeMap<Identifier, TokenAmount>),
    /// Transfer result with historical tracking via document storage.
    HistoricalDocument(Document),
    /// Group-based transfer action with optional document for history.
    GroupActionWithDocument(GroupSumPower, Option<Document>),
}

impl Sdk {
    /// Transfers tokens from one identity to one or more recipients.
    ///
    /// This method broadcasts a transfer transition to move tokens between identities.
    /// The result varies based on token configuration:
    /// - Standard tokens return updated balances for all affected identities
    /// - Tokens with history tracking return documents
    /// - Group-managed tokens include group power information
    ///
    /// # Arguments
    ///
    /// * `transfer_tokens_transition_builder` - Builder containing transfer parameters including recipients and amounts
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `TransferResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    pub async fn token_transfer<S: Signer>(
        &self,
        transfer_tokens_transition_builder: TokenTransferTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<TransferResult, Error> {
        let platform_version = self.version();

        let state_transition = transfer_tokens_transition_builder
            .sign(self, signing_key, signer, platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenIdentitiesBalances(balances) => {
                Ok(TransferResult::IdentitiesBalances(balances))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                Ok(TransferResult::HistoricalDocument(doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                Ok(TransferResult::GroupActionWithDocument(power, doc))
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenIdentitiesBalances, VerifiedTokenActionWithDocument, or VerifiedTokenGroupActionWithDocument for transfer transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
