use crate::{DapiError, metrics};
use dapi_grpc::platform::v0::{PlatformSubscriptionRequest, PlatformSubscriptionResponse};
use dapi_grpc::tonic::{Request, Response, Status};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tokio_stream::wrappers::ReceiverStream;

use super::PlatformServiceImpl;

const PLATFORM_EVENTS_STREAM_BUFFER: usize = 512;

/// Tracks an active platform events session until all clones drop.
struct ActiveSessionGuard;

impl ActiveSessionGuard {
    fn new() -> Arc<Self> {
        metrics::platform_events_active_sessions_inc();
        Arc::new(Self)
    }
}

impl Drop for ActiveSessionGuard {
    fn drop(&mut self) {
        metrics::platform_events_active_sessions_dec();
    }
}

impl PlatformServiceImpl {
    /// Proxy implementation of Platform::subscribePlatformEvents.
    ///
    /// Forwards a subscription request upstream to Drive and streams responses back to the caller.
    pub async fn subscribe_platform_events_impl(
        &self,
        mut request: Request<PlatformSubscriptionRequest>,
    ) -> Result<Response<ReceiverStream<Result<PlatformSubscriptionResponse, Status>>>, Status>
    {
        let active_session = ActiveSessionGuard::new();

        let timeout_duration = Duration::from_millis(self.config.dapi.platform_events_timeout);
        request.set_timeout(timeout_duration);

        let mut client = self.drive_client.get_client();
        let uplink_resp = client.subscribe_platform_events(request).await?;
        metrics::platform_events_upstream_stream_started();
        let mut uplink_resp_rx = uplink_resp.into_inner();

        // Channel to forward responses back to caller (downlink)
        let (downlink_resp_tx, downlink_resp_rx) = mpsc::channel::<
            Result<PlatformSubscriptionResponse, Status>,
        >(PLATFORM_EVENTS_STREAM_BUFFER);

        {
            let timeout_tx = downlink_resp_tx.clone();
            self.workers.spawn(async move {
                sleep(timeout_duration).await;
                let status =
                    Status::deadline_exceeded("platform events subscription deadline exceeded");
                metrics::platform_events_forwarded_error();
                if timeout_tx.send(Err(status.clone())).await.is_ok() {
                    tracing::debug!("Platform events stream timeout elapsed");
                }
                Ok::<(), DapiError>(())
            });
        }

        // Spawn a task to forward uplink responses -> downlink
        {
            let session_handle = active_session;
            self.workers.spawn(async move {
                let _session_guard = session_handle;
                while let Some(msg) = uplink_resp_rx.next().await {
                    match msg {
                        Ok(response) => {
                            metrics::platform_events_forwarded_event();
                            if downlink_resp_tx.send(Ok(response)).await.is_err() {
                                tracing::debug!(
                                    "Platform events downlink response channel closed; stopping forward"
                                );
                                break;
                            }
                        }
                        Err(status) => {
                            metrics::platform_events_forwarded_error();
                            if downlink_resp_tx.send(Err(status)).await.is_err() {
                                tracing::debug!(
                                    "Platform events downlink response channel closed while forwarding error"
                                );
                                break;
                            }
                        }
                    }
                }
                tracing::debug!("Platform events uplink response stream closed");
                Err::<(), DapiError>(DapiError::ConnectionClosed)
            });
        }

        Ok(Response::new(ReceiverStream::new(downlink_resp_rx)))
    }
}
