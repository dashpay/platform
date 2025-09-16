use dapi_grpc::platform::v0::{PlatformEventsCommand, PlatformEventsResponse};
use dapi_grpc::tonic::{Request, Response, Status};
use rs_dash_notify::UnboundedSenderSink;
use rs_dash_notify::event_mux::EventsResponseResult;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::metrics;

use super::PlatformServiceImpl;

impl PlatformServiceImpl {
    /// Proxy implementation of Platform::subscribePlatformEvents with upstream muxing.
    pub async fn subscribe_platform_events_impl(
        &self,
        request: Request<dapi_grpc::tonic::Streaming<PlatformEventsCommand>>,
    ) -> Result<Response<UnboundedReceiverStream<Result<PlatformEventsResponse, Status>>>, Status>
    {
        // Use shared upstream mux from PlatformServiceImpl
        let mux = self.platform_events_mux.clone();

        let (resp_tx, resp_rx) = mpsc::unbounded_channel::<EventsResponseResult>();
        let subscriber = mux.add_subscriber().await;
        metrics::platform_events_active_sessions_inc();

        // Link inbound stream to mux command channel
        let inbound = request.into_inner();
        let resp_sink = UnboundedSenderSink::from(resp_tx.clone());

        let mut workers = self.workers.lock().await;
        workers.spawn(subscriber.forward(inbound, resp_sink));

        Ok(Response::new(UnboundedReceiverStream::new(resp_rx)))
    }
}
