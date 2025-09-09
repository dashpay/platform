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
use rs_dapi_client::AddressList;
use rs_dash_notify::platform_mux::{PlatformEventsMux, PlatformMuxSettings};

impl PlatformServiceImpl {
    /// Proxy implementation of Platform::subscribePlatformEvents with upstream muxing.
    pub async fn subscribe_platform_events_impl(
        &self,
        request: Request<dapi_grpc::tonic::Streaming<PlatformEventsCommand>>,
    ) -> Result<Response<UnboundedReceiverStream<Result<PlatformEventsResponse, Status>>>, Status>
    {
        // Ensure single upstream mux exists (lazy init stored in self via once_cell)
        let mux = {
            use once_cell::sync::OnceCell;
            static MUX: OnceCell<PlatformEventsMux> = OnceCell::new();
            if let Some(m) = MUX.get() {
                m.clone()
            } else {
                let addresses = AddressList::from_str(&self.config.dapi.drive.uri)
                    .map_err(|e| Status::internal(format!("invalid drive uri: {}", e)))?;
                let settings = PlatformMuxSettings {
                    upstream_conn_count: 2,
                };
                let m = PlatformEventsMux::new(addresses, settings)
                    .map_err(|e| Status::internal(format!("failed to init upstream mux: {}", e)))?;
                MUX.set(m.clone()).ok();
                m
            }
        };

        let (out_tx, out_rx) = mpsc::unbounded_channel::<Result<PlatformEventsResponse, Status>>();
        let session = mux.register_session_with_tx(out_tx.clone()).await;
        metrics::platform_events_active_sessions_inc();

        let inbound = request.into_inner();
        rs_dash_notify::platform_mux::spawn_client_command_processor(
            session,
            inbound,
            out_tx.clone(),
        );

        Ok(Response::new(UnboundedReceiverStream::new(out_rx)))
    }
}
