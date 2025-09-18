use dapi_grpc::platform::v0::{PlatformEventsCommand, PlatformEventsResponse};
use dapi_grpc::tonic::{Request, Response, Status};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::PlatformServiceImpl;

impl PlatformServiceImpl {
    /// Proxy implementation of Platform::subscribePlatformEvents.
    ///
    /// Forwards commands from the caller (downlink) upstream to Drive
    /// and forwards responses back to the caller.
    pub async fn subscribe_platform_events_impl(
        &self,
        request: Request<dapi_grpc::tonic::Streaming<PlatformEventsCommand>>,
    ) -> Result<Response<UnboundedReceiverStream<Result<PlatformEventsResponse, Status>>>, Status>
    {
        // Inbound commands from the caller (downlink)
        let downlink_req_rx = request.into_inner();

        // Channel to feed commands upstream to Drive
        let (uplink_req_tx, uplink_req_rx) = mpsc::unbounded_channel::<PlatformEventsCommand>();

        // Spawn a task to forward downlink commands -> uplink channel
        {
            let mut downlink = downlink_req_rx;
            let workers = self.workers.clone();
            let mut workers = workers.lock().await;
            workers.spawn(async move {
                while let Some(cmd) = downlink.next().await {
                    match cmd {
                        Ok(msg) => {
                            if uplink_req_tx.send(msg).is_err() {
                                tracing::warn!(
                                    "Platform events uplink command channel closed; stopping forward"
                                );
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                "Error receiving platform event command from downlink"
                            );
                            break;
                        }
                    }
                }
                tracing::debug!("Platform events downlink stream closed");
            });
        }

        // Call upstream with our command stream
        let mut client = self.drive_client.get_client();
        let uplink_resp = client
            .subscribe_platform_events(tokio_stream::wrappers::UnboundedReceiverStream::new(
                uplink_req_rx,
            ))
            .await?;
        let mut uplink_resp_rx = uplink_resp.into_inner();

        // Channel to forward responses back to caller (downlink)
        let (downlink_resp_tx, downlink_resp_rx) =
            mpsc::unbounded_channel::<Result<PlatformEventsResponse, Status>>();

        // Spawn a task to forward uplink responses -> downlink
        {
            let workers = self.workers.clone();
            let mut workers = workers.lock().await;
            workers.spawn(async move {
                while let Some(msg) = uplink_resp_rx.next().await {
                    if downlink_resp_tx.send(msg).is_err() {
                        tracing::warn!(
                            "Platform events downlink response channel closed; stopping forward"
                        );
                        break;
                    }
                }
                tracing::debug!("Platform events uplink response stream closed");
            });
        }

        Ok(Response::new(UnboundedReceiverStream::new(
            downlink_resp_rx,
        )))
    }
}
