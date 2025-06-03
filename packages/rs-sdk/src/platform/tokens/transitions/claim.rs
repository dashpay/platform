use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::fungible_tokens::claim::TokenClaimTransitionBuilder;
use crate::{Error, Sdk};
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::proof_result::StateTransitionProofResult;

pub enum ClaimResult {
    Document(Document),
    GroupActionWithDocument(GroupSumPower, Document),
}

impl Sdk {
    pub async fn token_claim<S: Signer>(
        &self,
        claim_tokens_transition_builder: TokenClaimTransitionBuilder<'_>,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<ClaimResult, Error> {
        let platform_version = self.version();

        let state_transition = claim_tokens_transition_builder
            .sign(self, signing_key, signer, platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
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
