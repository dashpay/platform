use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::fungible_tokens::config_update::TokenConfigUpdateTransitionBuilder;
use crate::{Error, Sdk};
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::proof_result::StateTransitionProofResult;

pub enum ConfigUpdateResult {
    Document(Document),
    GroupActionWithDocument(GroupSumPower, Document),
}

impl Sdk {
    pub async fn token_update_contract_token_configuration<S: Signer>(
        &self,
        config_update_transition_builder: TokenConfigUpdateTransitionBuilder<'_>,
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
