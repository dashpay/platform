use crate::{Error, Sdk};
use dapi_grpc::core::v0::{
    transactions_with_proofs_response, InstantSendLockMessages, TransactionsWithProofsRequest,
    TransactionsWithProofsResponse,
};
use rs_dapi_client::{Dapi, RequestSettings};

impl Sdk {
    /// Starts the stream to listen for instant send lock messages
    pub async fn start_instant_send_lock_stream(
        &mut self,
    ) -> Result<dapi_grpc::tonic::Streaming<TransactionsWithProofsResponse>, Error> {
        let core_transactions_stream = TransactionsWithProofsRequest {
            bloom_filter: None,
            count: 100,
            send_transaction_hashes: false,
            from_block: None,
        };
        self.execute(core_transactions_stream, RequestSettings::default())
            .await
            .map_err(|e| Error::DapiClientError(e.to_string()))
    }
}

async fn wait_for_instant_send_lock_messages(
    mut stream: dapi_grpc::tonic::Streaming<TransactionsWithProofsResponse>,
) -> Result<InstantSendLockMessages, Error> {
    let instant_send_lock_messages;
    loop {
        if let Some(TransactionsWithProofsResponse { responses }) = stream
            .message()
            .await
            .map_err(|e| Error::DapiClientError(e.to_string()))?
        {
            match responses {
                Some(transactions_with_proofs_response::Responses::InstantSendLockMessages(
                    messages,
                )) => {
                    instant_send_lock_messages = messages;
                    break;
                }
                _ => continue,
            }
        } else {
            return Err(Error::DapiClientError(
                "steam closed unexpectedly".to_owned(),
            ));
        }
    }

    Ok(instant_send_lock_messages)
}
