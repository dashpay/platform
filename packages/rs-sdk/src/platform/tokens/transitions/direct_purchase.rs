use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::fungible_tokens::purchase::TokenDirectPurchaseTransitionBuilder;
use crate::{Error, Sdk};
use dpp::balances::credits::TokenAmount;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;

impl Sdk {
    pub async fn token_purchase<'a, S: Signer>(
        &self,
        purchase_tokens_transition_builder: TokenDirectPurchaseTransitionBuilder<'a>,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DirectPurchaseResult, Error> {
        let platform_version = self.version();

        let state_transition = purchase_tokens_transition_builder
            .sign(self, signing_key, signer, &platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenBalance(owner_id, balance) => {
                Ok(DirectPurchaseResult::TokenBalance(owner_id, balance))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                Ok(DirectPurchaseResult::HistoricalDocument(doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                Ok(DirectPurchaseResult::GroupActionWithDocument(power, doc))
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenBalance, VerifiedTokenActionWithDocument, or VerifiedTokenGroupActionWithDocument for direct purchase transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}

pub enum DirectPurchaseResult {
    TokenBalance(Identifier, TokenAmount),
    HistoricalDocument(dpp::document::Document),
    GroupActionWithDocument(
        dpp::data_contract::group::GroupSumPower,
        Option<dpp::document::Document>,
    ),
}
