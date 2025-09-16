use dapi_grpc::core::v0::{MasternodeListRequest, MasternodeListResponse};
use dapi_grpc::tonic::{Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::debug;

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
        let tx_stream = tx.clone();
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

                if tx_stream.send(response).is_err() {
                    debug!(
                        "Client disconnected from masternode list subscription: {}",
                        sub_handle.id()
                    );
                    break;
                }
            }
        });

        if let Err(err) = self.masternode_list_sync.ensure_ready().await {
            return Err(tonic::Status::from(err));
        }

        if let Some(diff) = self.masternode_list_sync.current_full_diff().await {
            if tx
                .send(Ok(MasternodeListResponse {
                    masternode_list_diff: diff,
                }))
                .is_err()
            {
                debug!(
                    "Client disconnected from masternode list subscription before initial response: {}",
                    subscription_handle.id()
                );
            }
        } else {
            debug!("Masternode list diff not available yet for initial response");
        }

        let stream = UnboundedReceiverStream::new(rx);
        Ok(Response::new(stream))
    }
}
