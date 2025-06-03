use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::{Error, Sdk};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::balances::credits::TokenAmount;
use dpp::document::Document;
use dpp::data_contract::group::GroupSumPower;
use dpp::group::group_action_status::GroupActionStatus;
use crate::platform::transition::fungible_tokens::burn::TokenBurnTransitionBuilder;

pub enum BurnResult {
    TokenBalance(Identifier, TokenAmount),
    HistoricalDocument(Document),
    GroupActionWithDocument(GroupSumPower, Option<Document>),
    GroupActionWithBalance(GroupSumPower, GroupActionStatus, Option<TokenAmount>),
}

impl Sdk {
    pub async fn burn_tokens<'a, S: Signer>(
        &self,
        burn_tokens_transition_builder: TokenBurnTransitionBuilder<'a>,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<BurnResult, Error> {
        let platform_version = self.version();

        let state_transition = burn_tokens_transition_builder
            .sign(self, signing_key, signer, &platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenBalance(owner_id, remaining_balance) => {
                Ok(BurnResult::TokenBalance(owner_id, remaining_balance))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                // This means the token keeps burning history
                Ok(BurnResult::HistoricalDocument(doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                // This means it's a group action with history
                Ok(BurnResult::GroupActionWithDocument(power, doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenBalance(power, status, balance) => {
                // Group action without history
                Ok(BurnResult::GroupActionWithBalance(power, status, balance))
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenBalance, VerifiedTokenActionWithDocument, VerifiedTokenGroupActionWithDocument, or VerifiedTokenGroupActionWithTokenBalance for burn transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}