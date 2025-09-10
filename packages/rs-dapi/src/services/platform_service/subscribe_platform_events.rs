use std::str::FromStr;
use std::{collections::BTreeMap, sync::Arc};

use dapi_grpc::platform::v0::platform_events_command::platform_events_command_v0::Command as Cmd;
use dapi_grpc::platform::v0::platform_events_command::Version as CmdVersion;
use dapi_grpc::platform::v0::platform_events_response::platform_events_response_v0::Response as Resp;
use dapi_grpc::platform::v0::platform_events_response::PlatformEventsResponseV0;
use dapi_grpc::platform::v0::{PlatformEventsCommand, PlatformEventsResponse, PlatformFilterV0};
use dapi_grpc::tonic::{Request, Response, Status};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::clients::drive_client::DriveClient;
use crate::metrics;

use super::PlatformServiceImpl;

// Use shared multiplexer from rs-dash-notify
use rs_dash_notify::platform_mux::spawn_client_command_processor;

impl PlatformServiceImpl {
    /// Proxy implementation of Platform::subscribePlatformEvents with upstream muxing.
    pub async fn subscribe_platform_events_impl(
        &self,
        request: Request<dapi_grpc::tonic::Streaming<PlatformEventsCommand>>,
    ) -> Result<Response<UnboundedReceiverStream<Result<PlatformEventsResponse, Status>>>, Status>
    {
        // Use shared upstream mux from PlatformServiceImpl
        let mux = self.platform_events_mux.clone();

        let (out_tx, out_rx) = mpsc::unbounded_channel::<Result<PlatformEventsResponse, Status>>();
        let session = mux.register_session_with_tx(out_tx.clone()).await;
        metrics::platform_events_active_sessions_inc();

        let inbound = request.into_inner();
        spawn_client_command_processor(
            session,
            inbound,
            out_tx.clone(),
        );

        Ok(Response::new(UnboundedReceiverStream::new(out_rx)))
    }
}
