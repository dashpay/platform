use dapi_grpc::core::v0::{MasternodeListRequest, MasternodeListResponse};
use dapi_grpc::tonic::{Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, info};

use crate::services::streaming_service::{
    FilterType, StreamingMessage, StreamingServiceImpl, SubscriptionType,
};

impl StreamingServiceImpl {
    pub async fn subscribe_to_masternode_list_impl(
        &self,
        _request: Request<MasternodeListRequest>,
    ) -> Result<Response<UnboundedReceiverStream<Result<MasternodeListResponse, Status>>>, Status>
    {
        // Create filter (no filtering needed for masternode list - all updates)
        let filter = FilterType::AllMasternodes;

        // Create channel for streaming responses
        let (tx, rx) = mpsc::unbounded_channel();

        // Create message channel for internal communication
        let (message_tx, mut message_rx) = mpsc::unbounded_channel::<StreamingMessage>();

        // Add subscription to manager
        let subscription_id = self
            .subscriber_manager
            .add_subscription(filter, SubscriptionType::MasternodeList, message_tx)
            .await;

        info!("Started masternode list subscription: {}", subscription_id);

        // Spawn task to convert internal messages to gRPC responses
        let subscriber_manager = self.subscriber_manager.clone();
        let sub_id = subscription_id.clone();
        tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                let response = match message {
                    StreamingMessage::MasternodeListDiff { data } => {
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
                        sub_id
                    );
                    break;
                }
            }

            // Clean up subscription when client disconnects
            subscriber_manager.remove_subscription(&sub_id).await;
            info!("Cleaned up masternode list subscription: {}", sub_id);
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
