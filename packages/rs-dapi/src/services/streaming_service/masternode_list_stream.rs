use dapi_grpc::core::v0::{MasternodeListRequest, MasternodeListResponse};
use dapi_grpc::tonic::{Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, info};

use crate::services::streaming_service::{FilterType, StreamingEvent, StreamingServiceImpl};

impl StreamingServiceImpl {
    pub async fn subscribe_to_masternode_list_impl(
        &self,
        _request: Request<MasternodeListRequest>,
    ) -> Result<Response<UnboundedReceiverStream<Result<MasternodeListResponse, Status>>>, Status>
    {
        // Create filter (no filtering needed for masternode list - all updates)
        let filter = FilterType::CoreAllMasternodes;

        // Create channel for streaming responses
        let (tx, rx) = mpsc::unbounded_channel();

        // Add subscription to manager
        let subscription_handle = self.subscriber_manager.add_subscription(filter).await;

        // Spawn task to convert internal messages to gRPC responses
        let sub_handle = subscription_handle.clone();
        tokio::spawn(async move {
            while let Some(message) = sub_handle.recv().await {
                let response = match message {
                    StreamingEvent::CoreMasternodeListDiff { data } => {
                        let response = MasternodeListResponse {
                            masternode_list_diff: data,
                        };

                        Ok(response)
                    }
                    _ => {
                        // Ignore other message types for this subscription
                        continue;
                    }
                };

                if tx.send(response).is_err() {
                    debug!(
                        "Client disconnected from masternode list subscription: {}",
                        sub_handle.id()
                    );
                    break;
                }
            }
        });

        // Send initial full masternode list
        tokio::spawn(async move {
            // TODO: Get current masternode list and send as initial diff
            debug!("Should send initial full masternode list");
        });

        let stream = UnboundedReceiverStream::new(rx);
        Ok(Response::new(stream))
    }
}
