use crate::platform::fetch_current_no_parameters::FetchCurrent;
use crate::platform::types::epoch::Epoch;
use crate::{Error, Sdk};
use bip37_bloom_filter::{BloomFilter, BloomFilterData};
use dapi_grpc::core::v0::{
    transactions_with_proofs_request, transactions_with_proofs_response, GetTransactionRequest,
    GetTransactionResponse, TransactionsWithProofsRequest, TransactionsWithProofsResponse,
};
use dpp::dashcore::consensus::Decodable;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{Address, BlockHash, InstantLock, MerkleBlock, OutPoint, Transaction, Txid};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::prelude::AssetLockProof;

use dapi_grpc::tonic::Code;
use rs_dapi_client::transport::TransportError;
use rs_dapi_client::{DapiClientError, DapiRequestExecutor, IntoInner, RequestSettings};
use std::time::Duration;
use tokio::time::{sleep, timeout};

/// A Core transaction fetched by id, plus the finality metadata needed to
/// reconstruct an asset-lock proof from it (an InstantSend proof when the
/// InstantLock is known, otherwise a ChainLock proof once chain-locked).
#[derive(Clone, Debug)]
pub struct FetchedCoreTransaction {
    /// The decoded transaction.
    pub transaction: Transaction,
    /// Height of the block the transaction was mined in (0 if unconfirmed).
    pub height: u32,
    /// Whether the transaction's block is ChainLocked.
    pub is_chain_locked: bool,
    /// Whether the transaction is InstantSend-locked. Deliberately surfaced but
    /// not required by the invitation claim: the proof carries the islock from
    /// the link, and consensus re-verifies it — this flag is informational.
    pub is_instant_locked: bool,
}

/// Where a Core transaction was mined, as the queried DAPI node reports it.
///
/// Everything here is self-reported by one node. In particular `block_hash`
/// is not verified: a caller that builds anything on the placement must check
/// the hash against a header chain it verified itself.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CoreTransactionPlacement {
    /// The decoded transaction.
    pub transaction: Transaction,
    /// Height of the block the transaction was mined in (0 if unconfirmed).
    pub height: u32,
    /// Hash of the block the transaction was mined in; `None` when unconfirmed
    /// or when the reported bytes are not a 32-byte hash.
    pub block_hash: Option<BlockHash>,
    /// Whether the transaction's block is ChainLocked.
    pub is_chain_locked: bool,
    /// Whether the transaction is InstantSend-locked.
    pub is_instant_locked: bool,
}

/// Whether an SDK error is a gRPC `NOT_FOUND` (the requested tx is unknown to
/// the node), as opposed to a transient/transport failure. Used to distinguish
/// "retry with a reversed txid" from "surface the error".
fn error_is_not_found(err: &Error) -> bool {
    match err {
        Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(status))) => {
            status.code() == Code::NotFound
        }
        Error::NoAvailableAddressesToRetry(inner) => error_is_not_found(inner),
        _ => false,
    }
}

/// DAPI fills `GetTransactionResponse.block_hash` by hex-decoding Core's
/// display string, so the bytes arrive reversed relative to the hash's
/// internal order.
fn block_hash_from_display_bytes(bytes: &[u8]) -> Option<BlockHash> {
    let mut hash: [u8; 32] = bytes.try_into().ok()?;
    hash.reverse();
    Some(BlockHash::from_byte_array(hash))
}

/// Decode the consensus-encoded transaction bytes of a `getTransaction` reply.
fn decode_transaction(bytes: &[u8]) -> Result<Transaction, Error> {
    Transaction::consensus_decode(&mut &bytes[..]).map_err(|e| Error::CoreError(e.into()))
}

impl Sdk {
    /// Fetch a Core transaction by its id via DAPI `getTransaction`.
    ///
    /// `txid` is the transaction id as a hex string (big-endian display form).
    /// Returns `Ok(Some(..))` with the decoded transaction plus its
    /// confirmation/lock metadata; `Ok(None)` when the node does not know the tx
    /// (empty response or gRPC `NOT_FOUND`) so the caller can retry with the id
    /// byte-reversed; and `Err` for a transient/transport failure that must not
    /// be masked by a doomed reversed-id retry.
    pub async fn get_transaction(
        &self,
        txid: &str,
    ) -> Result<Option<FetchedCoreTransaction>, Error> {
        let Some(response) = self
            .fetch_core_transaction(txid, RequestSettings::default())
            .await?
        else {
            return Ok(None);
        };

        Ok(Some(FetchedCoreTransaction {
            transaction: decode_transaction(&response.transaction)?,
            height: response.height,
            is_chain_locked: response.is_chain_locked,
            is_instant_locked: response.is_instant_locked,
        }))
    }

    /// Fetch where a Core transaction was mined via DAPI `getTransaction`,
    /// including the block hash the node reports.
    ///
    /// Same `txid` form and `Ok(None)` / `Err` contract as
    /// [`Sdk::get_transaction`]; `settings` override the SDK's request
    /// settings for this call, so a caller on a deadline can bound it. The
    /// placement is unverified — see [`CoreTransactionPlacement`].
    pub async fn get_transaction_placement(
        &self,
        txid: &str,
        settings: RequestSettings,
    ) -> Result<Option<CoreTransactionPlacement>, Error> {
        let Some(response) = self.fetch_core_transaction(txid, settings).await? else {
            return Ok(None);
        };

        Ok(Some(CoreTransactionPlacement {
            transaction: decode_transaction(&response.transaction)?,
            height: response.height,
            block_hash: block_hash_from_display_bytes(&response.block_hash),
            is_chain_locked: response.is_chain_locked,
            is_instant_locked: response.is_instant_locked,
        }))
    }

    /// Run `getTransaction`, mapping an unknown transaction (gRPC `NOT_FOUND`
    /// or an empty reply) to `Ok(None)` and every other failure to `Err`.
    async fn fetch_core_transaction(
        &self,
        txid: &str,
        settings: RequestSettings,
    ) -> Result<Option<GetTransactionResponse>, Error> {
        let response = match self
            .execute(
                GetTransactionRequest {
                    id: txid.to_string(),
                },
                settings,
            )
            .await
            .into_inner()
        {
            Ok(response) => response,
            Err(e) => {
                let err: Error = e.into();
                return if error_is_not_found(&err) {
                    Ok(None)
                } else {
                    Err(err)
                };
            }
        };

        if response.transaction.is_empty() {
            return Ok(None);
        }
        Ok(Some(response))
    }

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
            .into_inner()
            .map_err(|e| e.into())
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
                    .map_err(|e| Error::Generic(format!("can't receive message: {e}")))?;

                let Some(TransactionsWithProofsResponse { responses }) = message else {
                    return Err(Error::Generic("stream closed unexpectedly".to_string()));
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
                                .await // TODO: We need better way to handle execution errors
                                .into_inner()?;

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

#[cfg(test)]
mod tests {
    use super::*;

    /// DAPI's display-order bytes come back as the hash's internal order.
    #[test]
    fn block_hash_from_display_bytes_reverses_the_bytes() {
        let mut display = [0u8; 32];
        display[0] = 0xaa;
        display[31] = 0x01;
        let hash = block_hash_from_display_bytes(&display).expect("32 bytes");
        let internal = hash.to_byte_array();
        assert_eq!(internal[0], 0x01);
        assert_eq!(internal[31], 0xaa);
    }

    /// Anything but a 32-byte hash, including an unconfirmed tx's empty field,
    /// yields no hash.
    #[test]
    fn block_hash_from_display_bytes_rejects_wrong_lengths() {
        assert!(block_hash_from_display_bytes(&[]).is_none());
        assert!(block_hash_from_display_bytes(&[0u8; 31]).is_none());
        assert!(block_hash_from_display_bytes(&[0u8; 33]).is_none());
    }
}
