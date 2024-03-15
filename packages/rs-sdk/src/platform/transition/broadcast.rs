use super::broadcast_request::BroadcastRequestForStateTransition;
use crate::{Error, Sdk};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use rs_dapi_client::{DapiRequest, RequestSettings};

#[async_trait::async_trait]
pub trait BroadcastStateTransition {
    async fn broadcast(&self, sdk: &Sdk) -> Result<(), Error>;
    async fn broadcast_and_wait(
        &self,
        sdk: &Sdk,
        time_out_ms: Option<u64>,
    ) -> Result<StateTransitionProofResult, Error>;
}

#[async_trait::async_trait]
impl BroadcastStateTransition for StateTransition {
    async fn broadcast(&self, sdk: &Sdk) -> Result<(), Error> {
        let request = self.broadcast_request_for_state_transition()?;

        request.execute(sdk, RequestSettings::default()).await?;

        // response is empty for a broadcast, result comes from the stream wait for state transition result

        Ok(())
    }

    async fn broadcast_and_wait(
        &self,
        sdk: &Sdk,
        _time_out_ms: Option<u64>,
    ) -> Result<StateTransitionProofResult, Error> {
        let request = self.broadcast_request_for_state_transition()?;

        request
            .clone()
            .execute(sdk, RequestSettings::default())
            .await?;

        let request = self.wait_for_state_transition_result_request()?;

        let response = request.execute(sdk, RequestSettings::default()).await?;

        let block_time = response.metadata()?.time_ms;
        let proof = response.proof_owned()?;

        let (_, result) = Drive::verify_state_transition_was_executed_with_proof(
            self,
            block_time,
            proof.grovedb_proof.as_slice(),
            &|_| Ok(None),
            sdk.version(),
        )?;

        Ok(result)
    }
}
