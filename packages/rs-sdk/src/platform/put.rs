use crate::platform::broadcast_request::BroadcastRequestForStateTransition;
use crate::{Error, Sdk};
use dpp::state_transition::StateTransition;
use rs_dapi_client::{DapiRequest, RequestSettings};

#[async_trait::async_trait]
pub trait BroadcastStateTransition {
    async fn broadcast(&self, sdk: &mut Sdk) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl BroadcastStateTransition for StateTransition {
    async fn broadcast(&self, sdk: &mut Sdk) -> Result<(), Error> {
        let request = self.broadcast_request_for_state_transition()?;

        request
            .clone()
            .execute(sdk, RequestSettings::default())
            .await?;

        // response is empty for a broadcast, result comes from the stream wait for state transition result

        Ok(())
    }
}
