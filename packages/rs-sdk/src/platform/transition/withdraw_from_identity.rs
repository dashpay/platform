use dpp::dashcore::Address;
use dpp::identity::accessors::IdentityGettersV0;

use dpp::identity::core_script::CoreScript;
use dpp::identity::signer::Signer;
use dpp::identity::Identity;
use dpp::prelude::UserFeeIncrease;

use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::state_transition::identity_credit_withdrawal_transition::methods::{
    IdentityCreditWithdrawalTransitionMethodsV0, PreferredKeyPurposeForSigningWithdrawal,
};
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::withdrawal::Pooling;

#[async_trait::async_trait]
pub trait WithdrawFromIdentity {
    /// Function to withdraw credits from an identity. Returns the final identity balance.
    async fn withdraw<S: Signer + Send>(
        &self,
        sdk: &Sdk,
        address: Option<Address>,
        amount: u64,
        core_fee_per_byte: Option<u32>,
        user_fee_increase: Option<UserFeeIncrease>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error>;
}

#[async_trait::async_trait]
impl WithdrawFromIdentity for Identity {
    async fn withdraw<S: Signer + Send>(
        &self,
        sdk: &Sdk,
        address: Option<Address>,
        amount: u64,
        core_fee_per_byte: Option<u32>,
        user_fee_increase: Option<UserFeeIncrease>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error> {
        let new_identity_nonce = sdk.get_identity_nonce(self.id(), true, settings).await?;
        let script = address.map(|address| CoreScript::new(address.script_pubkey()));
        let state_transition = IdentityCreditWithdrawalTransition::try_from_identity(
            self,
            script,
            amount,
            Pooling::Never,
            core_fee_per_byte.unwrap_or(1),
            user_fee_increase.unwrap_or_default(),
            signer,
            None,
            PreferredKeyPurposeForSigningWithdrawal::TransferPreferred,
            new_identity_nonce,
            sdk.version(),
            None,
        )?;

        let result = state_transition.broadcast_and_wait(sdk, None).await?;

        match result {
            StateTransitionProofResult::VerifiedPartialIdentity(identity) => {
                identity.balance.ok_or(Error::DapiClientError(
                    "expected an identity balance after withdrawal".to_string(),
                ))
            }
            _ => Err(Error::DapiClientError("proved a non identity".to_string())),
        }
    }
}
