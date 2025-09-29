use dapi_grpc::core::v0::{MasternodeListRequest, MasternodeListResponse};
use dapi_grpc::tonic::{Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, warn};

use crate::DapiError;
use crate::services::streaming_service::{FilterType, StreamingEvent, StreamingServiceImpl};

const MASTERNODE_STREAM_BUFFER: usize = 512;

impl StreamingServiceImpl {
    pub async fn subscribe_to_masternode_list_impl(
        &self,
        _request: Request<MasternodeListRequest>,
    ) -> Result<Response<ReceiverStream<Result<MasternodeListResponse, Status>>>, Status> {
        // Create filter (no filtering needed for masternode list - all updates)
        let filter = FilterType::CoreAllMasternodes;

        // Create channel for streaming responses
        let (tx, rx) = mpsc::channel(MASTERNODE_STREAM_BUFFER);

        // Add subscription to manager
        let subscription_handle = self.subscriber_manager.add_subscription(filter).await;

        let subscriber_id = subscription_handle.id();
        debug!(subscriber_id, "masternode_list_stream=subscribed");

        // Spawn task to convert internal messages to gRPC responses
        let sub_handle = subscription_handle.clone();
        let tx_stream = tx.clone();
        self.workers.spawn(async move {
            while let Some(message) = sub_handle.recv().await {
                let response = match message {
                    StreamingEvent::CoreMasternodeListDiff { data } => {
                        debug!(
                            subscriber_id = sub_handle.id(),
                            payload_size = data.len(),
                            "masternode_list_stream=forward_diff"
                        );
                        let response = MasternodeListResponse {
                            masternode_list_diff: data,
                        };

                        Ok(response)
                    }
                    other => {
                        tracing::trace!(event=?other, event_type=std::any::type_name_of_val(&other), "Ignoring non-matching event message type");
                        // Ignore other message types for this subscription
                        continue;
                    }
                };

                if tx_stream.send(response).await.is_err() {
                    debug!(
                        "Client disconnected from masternode list subscription: {}",
                        sub_handle.id()
                    );
                    break;
                }
            }
            Result::<(),DapiError>::Ok(())
        });

        if let Err(err) = self.masternode_list_sync.ensure_ready().await {
            warn!(
                subscriber_id,
                error = %err,
                "masternode_list_stream=ensure_ready_failed"
            );
            return Err(tonic::Status::from(err));
        }

        if let Some(diff) = self.masternode_list_sync.current_full_diff().await {
            debug!(
                subscriber_id,
                payload_size = diff.len(),
                "masternode_list_stream=send_initial_diff"
            );
            if tx
                .send(Ok(MasternodeListResponse {
                    masternode_list_diff: diff,
                }))
                .await
                .is_err()
            {
                debug!(
                    "Client disconnected from masternode list subscription before initial response: {}",
                    subscription_handle.id()
                );
            }
        } else {
            debug!(subscriber_id, "masternode_list_stream=no_initial_diff");
        }

        let stream = ReceiverStream::new(rx);
        debug!(subscriber_id, "masternode_list_stream=stream_ready");
        Ok(Response::new(stream))
    }
}
