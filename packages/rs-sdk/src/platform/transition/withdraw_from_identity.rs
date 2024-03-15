use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Address;
use dpp::identity::accessors::IdentityGettersV0;

use dpp::identity::core_script::CoreScript;
use dpp::identity::signer::Signer;
use dpp::identity::Identity;
use dpp::prelude::UserFeeIncrease;

use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;

use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::state_transition::identity_credit_withdrawal_transition::methods::IdentityCreditWithdrawalTransitionMethodsV0;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::withdrawal::Pooling;
use drive::drive::Drive;
use rs_dapi_client::{DapiRequest, RequestSettings};

#[async_trait::async_trait]
pub trait WithdrawFromIdentity {
    /// Function to withdraw credits from an identity. Returns the final identity balance.
    async fn withdraw<S: Signer + Send>(
        &self,
        sdk: &Sdk,
        address: Address,
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
        address: Address,
        amount: u64,
        core_fee_per_byte: Option<u32>,
        user_fee_increase: Option<UserFeeIncrease>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error> {
        let new_identity_nonce = sdk.get_identity_nonce(self.id(), true, settings).await?;
        let state_transition = IdentityCreditWithdrawalTransition::try_from_identity(
            self,
            CoreScript::new(address.script_pubkey()),
            amount,
            Pooling::Never,
            core_fee_per_byte.unwrap_or(1),
            user_fee_increase.unwrap_or_default(),
            signer,
            new_identity_nonce,
            sdk.version(),
            None,
        )?;

        let request = state_transition.broadcast_request_for_state_transition()?;

        request
            .clone()
            .execute(sdk, settings.unwrap_or_default().request_settings)
            .await?;

        let request = state_transition.wait_for_state_transition_result_request()?;

        let response = request.execute(sdk, RequestSettings::default()).await?;

        let block_time = response.metadata()?.time_ms;

        let proof = response.proof_owned()?;

        let (_, result) = Drive::verify_state_transition_was_executed_with_proof(
            &state_transition,
            block_time,
            proof.grovedb_proof.as_slice(),
            &|_| Ok(None),
            sdk.version(),
        )?;

        match result {
            StateTransitionProofResult::VerifiedPartialIdentity(identity) => {
                identity.balance.ok_or(Error::DapiClientError(
                    "expected an identity balance".to_string(),
                ))
            }
            _ => Err(Error::DapiClientError("proved a non identity".to_string())),
        }
    }
}
