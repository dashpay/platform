use crate::{Error, Sdk};
use bip37_bloom_filter::{BloomFilter, BloomFilterData};
use dapi_grpc::core::v0::{
    transactions_with_proofs_request, transactions_with_proofs_response, GetStatusRequest,
    TransactionsWithProofsRequest, TransactionsWithProofsResponse,
};

use dpp::dashcore::consensus::Decodable;
use dpp::dashcore::{Address, InstantLock, Transaction};
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::prelude::AssetLockProof;
use rs_dapi_client::{Dapi, RequestSettings};

use std::time::Duration;

impl Sdk {
    /// Starts the stream to listen for instant send lock messages
    pub async fn start_instant_send_lock_stream(
        &self,
        address: &Address,
    ) -> Result<dapi_grpc::tonic::Streaming<TransactionsWithProofsResponse>, Error> {
        let address_bytes = address.payload().script_pubkey().into_bytes();

        // create the bloom filter
        let bloom_filter = BloomFilter::builder(1, 0.001)
            .expect("this FP rate allows up to 10000 items")
            .add_element(&address_bytes)
            .build();

        let bloom_filter_proto = {
            let BloomFilterData {
                v_data,
                n_hash_funcs,
                n_tweak,
                n_flags,
            } = bloom_filter.into();
            dapi_grpc::core::v0::BloomFilter {
                v_data,
                n_hash_funcs,
                n_tweak,
                n_flags,
            }
        };

        let block_hash = self
            .execute(GetStatusRequest {}, RequestSettings::default())
            .await?
            .chain
            .map(|chain| chain.best_block_hash)
            .ok_or_else(|| Error::DapiClientError("missing `chain` field".to_owned()))?;

        let core_transactions_stream = TransactionsWithProofsRequest {
            bloom_filter: Some(bloom_filter_proto),
            count: 0, // Subscribing to new transactions as well
            send_transaction_hashes: true,
            from_block: Some(transactions_with_proofs_request::FromBlock::FromBlockHash(
                block_hash,
            )),
        };
        self.execute(core_transactions_stream, RequestSettings::default())
            .await
            .map_err(|e| Error::DapiClientError(e.to_string()))
    }

    /// Waits for a response for the asset lock proof
    pub async fn wait_for_asset_lock_proof_for_transaction(
        &self,
        mut stream: dapi_grpc::tonic::Streaming<TransactionsWithProofsResponse>,
        transaction: &Transaction,
        timeout: Option<Duration>,
        // core_chain_locked_height: u32,
    ) -> Result<AssetLockProof, Error> {
        let transaction_id = transaction.txid();

        let _span = tracing::debug_span!(
            "wait_for_asset_lock_proof_for_transaction",
            transaction_id = transaction_id.to_string(),
        )
        .entered();

        tracing::debug!("waiting for messages from stream");

        // Define an inner async block to handle the stream processing.
        let stream_processing = async {
            loop {
                // TODO: We should retry if Err is returned
                let message = stream
                    .message()
                    .await
                    .map_err(|e| Error::DapiClientError(format!("can't receive message: {e}")))?;

                let Some(TransactionsWithProofsResponse { responses }) = message else {
                    return Err(Error::DapiClientError(
                        "stream closed unexpectedly".to_string(),
                    ));
                };

                match responses {
                    Some(
                        transactions_with_proofs_response::Responses::InstantSendLockMessages(
                            instant_send_lock_messages,
                        ),
                    ) => {
                        tracing::debug!(
                            "received {} instant lock message(s)",
                            instant_send_lock_messages.messages.len()
                        );

                        for instant_lock_bytes in instant_send_lock_messages.messages {
                            let instant_lock =
                                InstantLock::consensus_decode(&mut instant_lock_bytes.as_slice())
                                    .map_err(|e| {
                                    tracing::error!("invalid asset lock: {}", e);

                                    Error::CoreError(e.into())
                                })?;

                            if instant_lock.txid == transaction_id {
                                let asset_lock_proof =
                                    AssetLockProof::Instant(InstantAssetLockProof {
                                        instant_lock,
                                        transaction: transaction.clone(),
                                        output_index: 0,
                                    });

                                tracing::debug!(
                                    ?asset_lock_proof,
                                    "instant lock is matching to the broadcasted transaction, returning instant asset lock proof"
                                );

                                return Ok(asset_lock_proof);
                            } else {
                                tracing::debug!(
                                    "instant lock is not matching, waiting for the next message"
                                );
                            }
                        }
                    } // TODO: Implement chain asset lock proof
                    Some(transactions_with_proofs_response::Responses::RawMerkleBlock(
                        _raw_merkle_block,
                    )) => {
                        tracing::debug!("received merkle block");

                        // let merkle_block =
                        //     MerkleBlock::consensus_decode(&mut raw_merkle_block.as_slice())
                        //         .map_err(|e| {
                        //             tracing::error!("can't decode merkle block: {}", error);
                        //
                        //             Error::CoreError(e.into())
                        //         })?;
                        //
                        // let mut matches: Vec<Txid> = vec![];
                        // let mut index: Vec<u32> = vec![];
                        //
                        // merkle_block.extract_matches(&mut matches, &mut index)?;
                        //
                        // if matches.contains(&transaction_id) {
                        //     let asset_lock_proof = AssetLockProof::Chain(ChainAssetLockProof {
                        //         core_chain_locked_height: 0,   //todo
                        //         out_point: Default::default(), //todo
                        //     });
                        //
                        //     tracing::debug!(
                        //               ?asset_lock_proof,
                        //               "merkle block contains the broadcasted transaction, returning chain asset lock proof"
                        //           );
                        //
                        //     return Ok(asset_lock_proof);
                        // }
                    }
                    None => tracing::trace!(
                        "received None when waiting for asset lock, it can be safely ignored as it's our workaround for tonic bug"
                    ),
                    _ => {
                        tracing::debug!(response=?responses, "received unexpected response");

                        continue;
                    }
                };
            }
            // Err(Error::DapiClientError(
            //     "Asset lock proof not found".to_string(),
            // ))
        };

        // TODO: Timeout must be set when we open the stream. Tonic will deal with it
        // Apply the timeout if `time_out_ms` is Some, otherwise just await the processing.
        match timeout {
            Some(t) => tokio::time::timeout(t, stream_processing)
                .await
                .map_err(|_| Error::DapiClientError("Timeout reached".to_string()))?,
            None => stream_processing.await,
        }
    }
}
