use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::fungible_tokens::destroy::TokenDestroyFrozenFundsTransitionBuilder;
use crate::{Error, Sdk};
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::proof_result::StateTransitionProofResult;

pub enum DestroyFrozenFundsResult {
    HistoricalDocument(Document),
    GroupActionWithDocument(GroupSumPower, Option<Document>),
}

impl Sdk {
    pub async fn token_destroy_frozen_funds<'a, S: Signer>(
        &self,
        destroy_frozen_funds_transition_builder: TokenDestroyFrozenFundsTransitionBuilder<'a>,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DestroyFrozenFundsResult, Error> {
        let platform_version = self.version();

        let state_transition = destroy_frozen_funds_transition_builder
            .sign(self, signing_key, signer, &platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
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
