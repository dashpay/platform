use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::fungible_tokens::mint::TokenMintTransitionBuilder;
use crate::{Error, Sdk};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;

pub enum MintResult {
    TokenBalance(Identifier, TokenAmount),
    HistoricalDocument(Document),
    GroupActionWithDocument(GroupSumPower, Option<Document>),
    GroupActionWithBalance(GroupSumPower, GroupActionStatus, Option<TokenAmount>),
}

impl Sdk {
    pub async fn token_mint<S: Signer>(
        &self,
        mint_tokens_transition_builder: TokenMintTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<MintResult, Error> {
        let platform_version = self.version();

        let state_transition = mint_tokens_transition_builder
            .sign(self, signing_key, signer, &platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenBalance(recipient_id_result, new_balance) => {
                Ok(MintResult::TokenBalance(recipient_id_result, new_balance))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                // This means the token keeps minting history
                Ok(MintResult::HistoricalDocument(doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                // This means it's a group action with history
                Ok(MintResult::GroupActionWithDocument(power, doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenBalance(power, status, balance) => {
                // Group action without history
                Ok(MintResult::GroupActionWithBalance(power, status, balance))
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenBalance, VerifiedTokenActionWithDocument, VerifiedTokenGroupActionWithDocument, or VerifiedTokenGroupActionWithTokenBalance for mint transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
