use crate::metrics;
use dapi_grpc::platform::v0::{
    PlatformEventsCommand, PlatformEventsResponse, platform_events_command,
    platform_events_response,
};
use dapi_grpc::tonic::{Request, Response, Status};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc;
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

fn platform_events_command_label(command: &PlatformEventsCommand) -> &'static str {
    use platform_events_command::Version;
    use platform_events_command::platform_events_command_v0::Command;

    match command.version.as_ref() {
        Some(Version::V0(v0)) => match v0.command.as_ref() {
            Some(Command::Add(_)) => "add",
            Some(Command::Remove(_)) => "remove",
            None => "unknown",
        },
        None => "unknown",
    }
}

enum ForwardedVariant {
    Event,
    Ack,
    Error,
    Unknown,
}

fn classify_forwarded_response(
    response: &Result<PlatformEventsResponse, Status>,
) -> ForwardedVariant {
    match response {
        Ok(res) => {
            use platform_events_response::Version;
            use platform_events_response::platform_events_response_v0::Response;
            match res.version.as_ref() {
                Some(Version::V0(v0)) => match v0.response.as_ref() {
                    Some(Response::Event(_)) => ForwardedVariant::Event,
                    Some(Response::Ack(_)) => ForwardedVariant::Ack,
                    Some(Response::Error(_)) => ForwardedVariant::Error,
                    None => ForwardedVariant::Unknown,
                },
                None => ForwardedVariant::Unknown,
            }
        }
        Err(_) => ForwardedVariant::Error,
    }
}

impl PlatformServiceImpl {
    /// Proxy implementation of Platform::subscribePlatformEvents.
    ///
    /// Forwards commands from the caller (downlink) upstream to Drive
    /// and forwards responses back to the caller.
    pub async fn subscribe_platform_events_impl(
        &self,
        request: Request<dapi_grpc::tonic::Streaming<PlatformEventsCommand>>,
    ) -> Result<Response<ReceiverStream<Result<PlatformEventsResponse, Status>>>, Status> {
        // Inbound commands from the caller (downlink)
        let downlink_req_rx = request.into_inner();

        // Channel to feed commands upstream to Drive
        let (uplink_req_tx, uplink_req_rx) =
            mpsc::channel::<PlatformEventsCommand>(PLATFORM_EVENTS_STREAM_BUFFER);

        let active_session = ActiveSessionGuard::new();

        // Spawn a task to forward downlink commands -> uplink channel
        {
            let mut downlink = downlink_req_rx;
            let session_handle = active_session.clone();
            let uplink_req_tx = uplink_req_tx.clone();

            self.workers.lock().await.spawn(async move {
                let _session_guard = session_handle;
                while let Some(cmd) = downlink.next().await {
                    match cmd {
                        Ok(msg) => {
                            let op_label = platform_events_command_label(&msg);
                            if let Err(e) = uplink_req_tx.send(msg).await {
                                tracing::debug!(
                                    error = %e,
                                    "Platform events uplink command channel closed; stopping forward"
                                );
                                break;
                            } else {
                                metrics::platform_events_command(op_label);
                            }
                        }
                        Err(e) => {
                            tracing::debug!(
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
            .subscribe_platform_events(ReceiverStream::new(uplink_req_rx))
            .await?;
        metrics::platform_events_upstream_stream_started();
        let mut uplink_resp_rx = uplink_resp.into_inner();

        // Channel to forward responses back to caller (downlink)
        let (downlink_resp_tx, downlink_resp_rx) =
            mpsc::channel::<Result<PlatformEventsResponse, Status>>(PLATFORM_EVENTS_STREAM_BUFFER);

        // Spawn a task to forward uplink responses -> downlink
        {
            let session_handle = active_session;
            self.workers.lock().await.spawn(async move {
                let _session_guard = session_handle;
                while let Some(msg) = uplink_resp_rx.next().await {
                    let variant = classify_forwarded_response(&msg);
                    if downlink_resp_tx.send(msg).await.is_err() {
                        tracing::debug!(
                            "Platform events downlink response channel closed; stopping forward"
                        );
                        break;
                    } else {
                        match variant {
                            ForwardedVariant::Event => metrics::platform_events_forwarded_event(),
                            ForwardedVariant::Ack => metrics::platform_events_forwarded_ack(),
                            ForwardedVariant::Error => metrics::platform_events_forwarded_error(),
                            ForwardedVariant::Unknown => {}
                        }
                    }
                }
                tracing::debug!("Platform events uplink response stream closed");
            });
        }

        Ok(Response::new(ReceiverStream::new(downlink_resp_rx)))
    }
}
