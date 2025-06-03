use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::fungible_tokens::transfer::TokenTransferTransitionBuilder;
use crate::{Error, Sdk};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use std::collections::BTreeMap;

pub enum TransferResult {
    IdentitiesBalances(BTreeMap<Identifier, TokenAmount>),
    HistoricalDocument(Document),
    GroupActionWithDocument(GroupSumPower, Option<Document>),
}

impl Sdk {
    pub async fn token_transfer<S: Signer>(
        &self,
        transfer_tokens_transition_builder: TokenTransferTransitionBuilder<'_>,
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
