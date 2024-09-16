use crate::platform::fetch_current_no_parameters::FetchCurrent;
use crate::platform::types::epoch::Epoch;
use crate::{Error, Sdk};
use bip37_bloom_filter::{BloomFilter, BloomFilterData};
use dapi_grpc::core::v0::{
    transactions_with_proofs_request, transactions_with_proofs_response, GetTransactionRequest,
    GetTransactionResponse, TransactionsWithProofsRequest, TransactionsWithProofsResponse,
};
use dpp::dashcore::consensus::Decodable;
use dpp::dashcore::{Address, InstantLock, MerkleBlock, OutPoint, Transaction, Txid};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::prelude::AssetLockProof;

use rs_dapi_client::{DapiRequestExecutor, RequestSettings};
use std::time::Duration;
use tokio::time::{sleep, timeout};

impl Sdk {
    /// Starts the stream to listen for instant send lock messages
    pub async fn start_instant_send_lock_stream(
        &self,
        from_block_hash: Vec<u8>,
        address: &Address,
    ) -> Result<dapi_grpc::tonic::Streaming<TransactionsWithProofsResponse>, Error> {
        let address_bytes = address.as_unchecked().payload_to_vec();

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

        let core_transactions_stream = TransactionsWithProofsRequest {
            bloom_filter: Some(bloom_filter_proto),
            count: 0, // Subscribing to new transactions as well
            send_transaction_hashes: true,
            from_block: Some(transactions_with_proofs_request::FromBlock::FromBlockHash(
                from_block_hash,
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
        time_out: Option<Duration>,
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
                    }
                    Some(transactions_with_proofs_response::Responses::RawMerkleBlock(
                        raw_merkle_block,
                    )) => {
                        tracing::debug!("received merkle block");

                        let merkle_block =
                            MerkleBlock::consensus_decode(&mut raw_merkle_block.as_slice())
                                .map_err(|e| {
                                    tracing::error!("can't decode merkle block: {}", e);

                                    Error::CoreError(e.into())
                                })?;

                        let mut matches: Vec<Txid> = vec![];
                        let mut index: Vec<u32> = vec![];

                        merkle_block.extract_matches(&mut matches, &mut index)?;

                        // Continue receiving messages until we find the transaction
                        if !matches.contains(&transaction_id) {
                            tracing::debug!(
                                "merkle block doesn't contain the transaction, waiting for the next message"
                            );

                            continue;
                        }

                        tracing::debug!(
                            "merkle block contains the transaction, obtaining core chain locked height"
                        );

                        // TODO: This a temporary implementation until we have headers stream running in background
                        //  so we can always get actual height and chain locks

                        // Wait until the block is chainlocked
                        let mut core_chain_locked_height;
                        loop {
                            let GetTransactionResponse {
                                height,
                                is_chain_locked,
                                ..
                            } = self
                                .execute(
                                    GetTransactionRequest {
                                        id: transaction_id.to_string(),
                                    },
                                    RequestSettings::default(),
                                )
                                .await?;

                            core_chain_locked_height = height;

                            if is_chain_locked {
                                break;
                            }

                            tracing::trace!("the transaction is on height {} but not chainlocked. try again in 1 sec", height);

                            sleep(Duration::from_secs(1)).await;
                        }

                        tracing::debug!(
                            "the transaction is chainlocked on height {}, waiting platform for reaching the same core height",
                            core_chain_locked_height
                        );

                        // Wait until platform chain is on the block's chain locked height
                        loop {
                            let (_epoch, metadata) =
                                Epoch::fetch_current_with_metadata(self).await?;

                            if metadata.core_chain_locked_height >= core_chain_locked_height {
                                break;
                            }

                            tracing::trace!(
                                "platform chain locked core height {} but we need {}. try again in 1 sec",
                                metadata.core_chain_locked_height,
                                core_chain_locked_height,
                            );

                            sleep(Duration::from_secs(1)).await;
                        }

                        let asset_lock_proof = AssetLockProof::Chain(ChainAssetLockProof {
                            core_chain_locked_height,
                            out_point: OutPoint {
                                txid: transaction.txid(),
                                vout: 0,
                            },
                        });

                        tracing::debug!(
                                ?asset_lock_proof,
                                "merkle block contains the broadcasted transaction, returning chain asset lock proof"
                            );

                        return Ok(asset_lock_proof);
                    }
                    Some(transactions_with_proofs_response::Responses::RawTransactions(_)) => {
                        tracing::trace!("received transaction(s), ignoring")
                    }
                    None => tracing::trace!(
                        "received empty response as a workaround for the bug in tonic, ignoring"
                    ),
                }
            }
        };

        // Apply the timeout if `time_out_ms` is Some, otherwise just await the processing.
        match time_out {
            Some(duration) => timeout(duration, stream_processing).await.map_err(|_| {
                Error::TimeoutReached(duration, String::from("receiving asset lock proof"))
            })?,
            None => stream_processing.await,
        }
    }
}
