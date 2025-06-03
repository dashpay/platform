use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::fungible_tokens::set_price::TokenChangeDirectPurchasePriceTransitionBuilder;
use crate::{Error, Sdk};
use dpp::data_contract::group::GroupSumPower;
use dpp::document::Document;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

pub enum SetPriceResult {
    PricingSchedule(Identifier, Option<TokenPricingSchedule>),
    HistoricalDocument(Document),
    GroupActionWithDocument(GroupSumPower, Option<Document>),
    GroupActionWithPricingSchedule(
        GroupSumPower,
        GroupActionStatus,
        Option<TokenPricingSchedule>,
    ),
}

impl Sdk {
    pub async fn token_set_price_for_direct_purchase<S: Signer>(
        &self,
        set_price_transition_builder: TokenChangeDirectPurchasePriceTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<SetPriceResult, Error> {
        let platform_version = self.version();

        let state_transition = set_price_transition_builder
            .sign(self, signing_key, signer, &platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedTokenPricingSchedule(owner_id, schedule) => {
                Ok(SetPriceResult::PricingSchedule(owner_id, schedule))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                Ok(SetPriceResult::HistoricalDocument(doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                Ok(SetPriceResult::GroupActionWithDocument(power, doc))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenPricingSchedule(power, status, schedule) => {
                Ok(SetPriceResult::GroupActionWithPricingSchedule(power, status, schedule))
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedTokenPricingSchedule, VerifiedTokenActionWithDocument, VerifiedTokenGroupActionWithDocument, or VerifiedTokenGroupActionWithTokenPricingSchedule for set price transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
