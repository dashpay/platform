use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::{Error, Sdk};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::document::Document;
use dpp::data_contract::group::GroupSumPower;
use crate::platform::transition::fungible_tokens::unfreeze::TokenUnfreezeTransitionBuilder;

pub enum UnfreezeResult {
    IdentityInfo(Identifier, IdentityTokenInfo),
    HistoricalDocument(Document),
    GroupActionWithDocument(GroupSumPower, Option<Document>),
    GroupActionWithIdentityInfo(GroupSumPower, IdentityTokenInfo),
}

impl Sdk {
    pub async fn unfreeze_tokens<'a, S: Signer>(
        &self,
        unfreeze_tokens_transition_builder: TokenUnfreezeTransitionBuilder<'a>,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<UnfreezeResult, Error> {
        let platform_version = self.version();

        let state_transition = unfreeze_tokens_transition_builder
            .sign(self, signing_key, signer, &platform_version, None)
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