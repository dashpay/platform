use crate::{Error, Sdk};
use dapi_grpc::core::v0::{
    transactions_with_proofs_response, TransactionsWithProofsRequest,
    TransactionsWithProofsResponse,
};
use dpp::dashcore::consensus::Decodable;
use dpp::dashcore::{InstantLock, Transaction};
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::prelude::AssetLockProof;
use rs_dapi_client::{Dapi, RequestSettings};
use std::time::Duration;
use tokio::time::timeout;

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

    /// Waits for a response for the asset lock proof
    pub async fn wait_for_asset_lock_proof_for_transaction(
        mut stream: dapi_grpc::tonic::Streaming<TransactionsWithProofsResponse>,
        transaction: &Transaction,
        time_out_ms: Option<u64>,
    ) -> Result<AssetLockProof, Error> {
        let transaction_id = transaction.txid();

        // Define an inner async block to handle the stream processing.
        let stream_processing = async {
            loop {
                if let Some(TransactionsWithProofsResponse { responses }) =
                    stream
                        .message()
                        .await
                        .map_err(|e| Error::DapiClientError(e.to_string()))?
                {
                    match responses {
                        Some(
                            transactions_with_proofs_response::Responses::InstantSendLockMessages(
                                instant_send_lock_messages,
                            ),
                        ) => {
                            match instant_send_lock_messages.messages.into_iter().find_map(
                                |instant_send_lock_message| match InstantLock::consensus_decode(
                                    &mut instant_send_lock_message.as_slice(),
                                )
                                .map_err(|e| Error::CoreError(e.into()))
                                {
                                    Ok(instant_lock) => {
                                        if instant_lock.txid == transaction_id {
                                            Some(Ok(AssetLockProof::Instant(
                                                InstantAssetLockProof {
                                                    instant_lock,
                                                    transaction: transaction.clone(),
                                                    output_index: 0,
                                                },
                                            )))
                                        } else {
                                            None
                                        }
                                    }
                                    Err(e) => Some(Err(e)),
                                },
                            ) {
                                Some(Ok(found_asset_lock_proof)) => {
                                    return Ok(found_asset_lock_proof)
                                }
                                Some(Err(e)) => return Err(e),
                                None => (),
                            }
                            break;
                        }
                        // Some(transactions_with_proofs_response::Responses::RawMerkleBlock(
                        //          raw_merkle_block,
                        //      )) => {
                        //     let merkle_block = MerkleBlock::consensus_decode(&mut raw_merkle_block.as_slice()).map_err(|e| Error::CoreError(e.into()))?;
                        //     let mut matches: Vec<Txid> = vec![];
                        //     let mut index: Vec<u32> = vec![];
                        //     merkle_block.extract_matches(&mut matches, &mut index)?;
                        //
                        //     if matches.contains(&transaction_id) {
                        //         return Ok(AssetLockProof::Chain(ChainAssetLockProof {
                        //             core_chain_locked_height: 0, //todo
                        //             out_point: Default::default() //todo
                        //         }))
                        //     }
                        // }
                        _ => continue,
                    }
                } else {
                    return Err(Error::DapiClientError(
                        "stream closed unexpectedly".to_string(),
                    ));
                }
            }
            Err(Error::DapiClientError(
                "Asset lock proof not found".to_string(),
            ))
        };

        // Apply the timeout if `time_out_ms` is Some, otherwise just await the processing.
        match time_out_ms {
            Some(ms) => timeout(Duration::from_millis(ms), stream_processing)
                .await
                .map_err(|_| Error::DapiClientError("Timeout reached".to_string()))?,
            None => stream_processing.await,
        }
    }
}
