use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use crate::{Error, Sdk};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::PrivateKey;
use dpp::identity::Identity;
use dpp::prelude::{AssetLockProof, UserFeeIncrease};
use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use drive::drive::Drive;
use rs_dapi_client::{DapiRequest, RequestSettings};

#[async_trait::async_trait]
pub trait TopUpIdentity {
    async fn top_up_identity(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        user_fee_increase: Option<UserFeeIncrease>,
    ) -> Result<u64, Error>;
}

#[async_trait::async_trait]
impl TopUpIdentity for Identity {
    async fn top_up_identity(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        user_fee_increase: Option<UserFeeIncrease>,
    ) -> Result<u64, Error> {
        let state_transition = IdentityTopUpTransition::try_from_identity(
            self,
            asset_lock_proof,
            asset_lock_proof_private_key.inner.as_ref(),
            user_fee_increase.unwrap_or_default(),
            sdk.version(),
            None,
        )?;

        let request = state_transition.broadcast_request_for_state_transition()?;

        request
            .clone()
            .execute(sdk, RequestSettings::default())
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
