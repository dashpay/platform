use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::{Error, Sdk};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::document::Document;
use dpp::data_contract::group::GroupSumPower;
use crate::platform::transition::fungible_tokens::freeze::TokenFreezeTransitionBuilder;

pub enum FreezeResult {
    IdentityInfo(Identifier, IdentityTokenInfo),
    HistoricalDocument(Document),
    GroupActionWithDocument(GroupSumPower, Option<Document>),
    GroupActionWithIdentityInfo(GroupSumPower, IdentityTokenInfo),
}

impl Sdk {
    pub async fn freeze_tokens<'a, S: Signer>(
        &self,
        freeze_tokens_transition_builder: TokenFreezeTransitionBuilder<'a>,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<FreezeResult, Error> {
        let platform_version = self.version();

        let state_transition = freeze_tokens_transition_builder
            .sign(self, signing_key, signer, &platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenIdentityInfo(owner_id_result, info) => {
                Ok(FreezeResult::IdentityInfo(owner_id_result, info))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                // This means the token keeps freezing history
                Ok(FreezeResult::HistoricalDocument(doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                // This means it's a group action with history
                Ok(FreezeResult::GroupActionWithDocument(power, doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenIdentityInfo(power, _, Some(info)) => {
                // Group action without history
                Ok(FreezeResult::GroupActionWithIdentityInfo(power, info))
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
                    "Expected VerifiedTokenIdentityInfo, VerifiedTokenActionWithDocument, or VerifiedTokenGroupActionWithTokenIdentityInfo for freeze transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}