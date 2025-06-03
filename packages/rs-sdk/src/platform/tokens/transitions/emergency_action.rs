use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::fungible_tokens::emergency_action::TokenEmergencyActionTransitionBuilder;
use crate::{Error, Sdk};
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::proof_result::StateTransitionProofResult;

impl Sdk {
    pub async fn token_emergency_action<'a, S: Signer>(
        &self,
        emergency_action_transition_builder: TokenEmergencyActionTransitionBuilder<'a>,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<EmergencyActionResult, Error> {
        let platform_version = self.version();

        let state_transition = emergency_action_transition_builder
            .sign(self, signing_key, signer, &platform_version, None)
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

pub enum EmergencyActionResult {
    GroupActionWithDocument(GroupSumPower, Option<Document>),
}
