//! Subscribe to events coming from the Core.
//!
//! This module contains a [SubscriptionController] that can be used to subscribe to events coming from the Core.
//! The controller will connect to the Core and start a new thread that will watch for events.
//!
//! [Subscription] represents a single subscription to the Core.
//! It usually starts a new thread, connects to the Core through DAPI and receives events as they
//! come from the Core. All events from all subscriptions are sent to a single broadcast [channel], which
//! can be obtained with [SubscriptionController::receiver()].

use crate::Error;
use bip37_bloom_filter::{BloomFilter, BloomFilterData};
use dapi_grpc::{
    core::v0::{
        transactions_with_proofs_request, transactions_with_proofs_response, GetTransactionRequest,
        InstantSendLockMessages, TransactionsWithProofsRequest, TransactionsWithProofsResponse,
    },
    mock::Mockable,
    platform::v0::{
        get_epochs_info_request, get_epochs_info_response, GetEpochsInfoRequest,
        GetEpochsInfoResponse,
    },
    tonic::Streaming,
};
use dashcore_rpc::dashcore::{consensus::Decodable, Address, InstantLock, MerkleBlock, Txid};
use futures::{Future, FutureExt};
use rs_dapi_client::{
    transport::{TransportClient, TransportRequest},
    Dapi, DapiClient, DapiClientError, RequestSettings,
};
use std::{
    collections::BTreeMap,
    fmt::Debug,
    sync::{
        atomic::{AtomicU32, AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::{
    runtime::Handle,
    sync::{
        broadcast::{channel, Receiver, Sender},
        Mutex,
    },
    task::JoinHandle,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

const CHANNEL_CAPACITY: usize = 100;
const SLEEP: std::time::Duration = Duration::from_secs(1);
const SUBSCRIPTION_STOP_TIMEOUT: std::time::Duration = Duration::from_secs(5);

// Errors that can happen during subscription.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SubscriptionError {
    /// Error occured during processing.
    #[error("Dash Core error: {0}")]
    CoreError(String),

    /// DAPI client error
    #[error("Dapi client error: {0}")]
    DapiClientError(String),

    /// Invalid version of the platform
    #[error("Invalid version: {0}")]
    VersionError(String),

    /// Subscription cancelled.
    #[error("Subscription cancelled: {0}")]
    Cancelled(String),
}

// Message received from Subscription
pub type SubscriptionMessage = Result<Message, SubscriptionError>;

// Channel receiver for subscription messages
pub type SubscriptionReceiver = Receiver<SubscriptionMessage>;

// Handle of core chain watcher thread.
//
// When dropped, the thread will be stopped and messages will no longer be received.
pub struct SubscriptionHandle {
    // Cancellation token used to stop the thread
    cancel: CancellationToken,
    jh: JoinHandle<()>,
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        self.cancel.cancel();
        // If a thread is not stopped within SUBSCRIPTION_STOP_TIMEOUT, we cancel it.
        let handle = std::mem::replace(&mut self.jh, tokio::task::spawn(async {}));
        tokio::runtime::Handle::current().spawn(async move {
            if let Err(e) = tokio::time::timeout(SUBSCRIPTION_STOP_TIMEOUT, handle).await {
                tracing::warn!(error = ?e, "subscription thread did not stop in time");
            }
        });
    }
}

/// Subscribe and receive notifications about events coming from the Dash Core.
///
/// Controller of the whole subscription mechanism.
///
/// Connects to DAPI and subscribe to notifications about various events coming from the Dash Core.
/// Multiple subscriptions can be started at the same time, and events from all subscriptions are sent to the same
/// channel.
///
/// # Usage
///
/// 1. Create new controller with [new()](SubscriptionController::new()).
/// 2. Open new channel with [receiver()](SubscriptionController::receiver()).
/// 3. Subscribe to events with [subscribe_to_address::subscribe()] or one of conveniance methods:
/// [subscribe_to_bloom_filter()](SubscriptionController::subscribe_to_bloom_filter()) or
/// [subscribe_to_address()](SubscriptionController::subscribe_to_address()).
/// Keep returned [SubscriptionHandle] in context.
/// 4. Process [Message]s from the channel. Note the channel can contain messages from multiple subscriptions.
/// 5. When done, drop the [SubscriptionHandle] to stop the subscription.
///
/// # Error handling
///
/// In case of an error during processing, [SubscriptionError] will be sent to the channel.
///
/// In case of lack of subscribers, the error will be ignored.
#[derive(Debug)]
pub struct SubscriptionController {
    dapi: Arc<DapiClient>,
    events_tx: Sender<Result<Message, SubscriptionError>>,
    cancel: CancellationToken,
}

impl SubscriptionController {
    /// Create new controller for Dash Core event subscriptions.
    ///
    /// Creates new controller that will manage Dash Core event subscriptions.
    /// The controller will use the given client to connect to the Core.
    ///
    /// Note that no connection to the Core is made until the first subscription is started.
    ///
    /// # Arguments
    ///
    /// * `client` - Client to use to connect to core.
    /// * `cancel` - Cancellation token that will be used to stop all threads when dropped.
    pub fn new(client: Arc<DapiClient>, cancel: CancellationToken) -> Self {
        // start a broadcast channel
        let (events_tx, _rx) = channel(CHANNEL_CAPACITY);

        Self {
            dapi: client,
            events_tx,
            cancel,
        }
    }

    /// Channel to receive events from the Core.
    ///
    /// All events from all subscriptions are broadcasted to the channel returned by this method.
    /// The channel has a [limited capacity](CHANNEL_CAPACITY), and will start dropping messages
    /// when the capacity is reached.
    ///
    /// ## Returns
    ///
    /// * [`SubscriptionReceiver`] - [channel Receiver](Receiver) that will return [SubscriptionMessage]s.
    ///
    /// ## Errors
    ///
    /// When processing of a message fails, [SubscriptionError] is sent to the channel.
    pub fn receiver(&self) -> SubscriptionReceiver {
        self.events_tx.subscribe()
    }

    /// Subscribe to Core messages using provided [Subscription].
    ///
    /// New worker thread will be started and registered in the list of threads.
    /// The thread will be stopped when the returned [SubscriptionHandle] is dropped.
    ///
    /// In most cases, you should use one of the convenience methods instead of calling this directly.
    ///
    /// ## Arguments
    ///
    /// * `worker` - subscription to start
    ///
    /// # Panics
    ///
    /// This will panic if called outside the context of a Tokio runtime.
    /// See [Handle::current()](tokio::runtime::Handle) for more detailed explanation.
    /// ## See also
    ///
    /// * [subscribe_to_bloom_filter()](SubscriptionController::subscribe_to_bloom_filter())
    /// * [subscribe_to_address()](SubscriptionController::subscribe_to_address())
    async fn subscribe<
        R: TransportRequest<Response = Streaming<M>>,
        M: Mockable,
        P: Subscription<R, M>,
    >(
        &self,
        worker: P,
    ) -> SubscriptionHandle
    where
        <<R as TransportRequest>::Client as TransportClient>::Error: std::fmt::Display,
        P: Send + Sync + 'static,
        M: Send,
    {
        let token = self.cancel.child_token();

        // Start the thread
        let hdl = Handle::current();
        let events_tx = self.events_tx.clone();
        let dapi = Arc::clone(&self.dapi);

        // Spawn new thread; we don't await it as we don't want to block the caller
        let worker_cancel = token.clone();
        let handle = hdl.spawn(async move {
            if let Err(e) = worker.worker(dapi, events_tx, worker_cancel).await {
                tracing::error!(error = ?e, "core subscription watcher failed");
            }
        });

        SubscriptionHandle {
            cancel: token,
            jh: handle,
        }
    }

    /// Subscribe to Dash Core events matching the given bloom filter.
    ///
    /// Start a new thread that will watch for events coming from the Core matching the given bloom filter.
    /// The thread will be stopped when the returned [SubscriptionHandle] is dropped.
    ///
    /// This is a conveniance method that calls [subscribe()] with a [Subscription] that matches the given bloom filter.
    ///
    /// ## Arguments
    ///
    /// * `bloom_filter` - Bloom filter to use to filter events.
    /// * `start_from_block_hash` - Block hash to start from. If `None`, the stream will start from the latest block.
    ///
    /// ## Returns
    ///
    /// * [`SubscriptionHandle`] - handle of the subscription; when dropped, the subscription will be stopped
    ///
    pub async fn subscribe_to_bloom_filter(
        &self,
        bloom_filter: BloomFilter,
        start_from_block_hash: Option<&[u8]>,
    ) -> SubscriptionHandle {
        // Create a new subscription
        let asset_lock_subscription = CoreSubscription {
            bloom_filter,
            dapi: Arc::clone(&self.dapi),
            starting_block: start_from_block_hash.map(|v| v.to_vec()),
            core_chain_locked_height: AtomicU32::new(0),
        };

        // Start the subscription worker
        self.subscribe(asset_lock_subscription).await
    }

    /// Subscribe to events related to provided address.
    ///
    /// Start a new thread that will watch for events coming from the Core.
    /// The thread will be stopped when the returned [SubscriptionHandle] is dropped.
    ///
    /// This is a convenience method that calls [subscribe_to_bloom_filter()] with a bloom filter that matches the given address.
    ///
    /// # Arguments
    ///
    /// * `address` - Address to watch.
    /// * `start_from_block_hash` - Block hash to start from. If `None`, the stream will start from the latest block.
    ///
    /// # Returns
    ///
    /// * [`SubscriptionHandle`] - handle for this subscription; when dropped, the subscription will be stopped.
    ///
    /// # Panics
    ///
    /// This will panic if called outside the context of a Tokio runtime.
    /// See [Handle::current()](tokio::runtime::Handle) for more detailed explanation.
    pub async fn subscribe_to_address(
        &self,
        address: Address,
        start_from_block_hash: Option<&[u8]>,
    ) -> SubscriptionHandle {
        let address_bytes = address.as_unchecked().payload_to_vec();

        // create the bloom filter
        let bloom_filter = BloomFilter::builder(1, 0.001)
            .expect("this FP rate allows up to 10000 items")
            .add_element(&address_bytes)
            .build();

        self.subscribe_to_bloom_filter(bloom_filter, start_from_block_hash)
            .await
    }
}

/// Message retrieved from the Core.
#[derive(Debug, Clone)]
pub enum Message {
    /// Asset lock proof received.
    InstantAssetLock {
        /// The transaction's Instant Lock
        instant_lock: InstantLock,
        /// Asset Lock Special Transaction
        tx_id: Txid,
    },
    // Merkle block received.
    MerkleBlock {
        /// The merkle block
        merkle_block: MerkleBlock,
    },
}

/// Worker that processes messages from the stream
///
/// ## Generic parameters
///
/// * `R` - Request type
/// * `M` - Type of message returned from the stream
trait Subscription<R: TransportRequest<Response = Streaming<M>>, M: Mockable>
where
    Self: Send + Sync,
    M: Send,
{
    /// Generate transport request, based on data provided during struct initialization
    fn request(&self) -> R;
    /// Worker thread that will start the stream and process messages.
    ///
    /// Default worker implementation starts new streaming connection to the DAPI using request generated with
    /// [request()] and processes incoming messages using [process()].
    ///
    /// ## Arguments
    ///
    /// * `dapi` - DAPI client to use to connect to the Core
    /// * `output` - broadcast [channel] to write procesed messages to
    ///
    /// ## Cancelation
    ///
    /// The worker will stop when the given cancelation token `cancel` is cancelled.
    fn worker<D: Dapi + Send + Sync>(
        &self,
        dapi: D,
        output: Sender<SubscriptionMessage>,
        cancel: CancellationToken,
    ) -> impl futures::Future<Output = std::result::Result<(), Error>> + Send
    where
        DapiClientError<<R::Client as TransportClient>::Error>: ToString,
        Self: Send,
    {
        async move {
            let mut stream = dapi
                .execute(self.request(), RequestSettings::default())
                .await
                .map_err(|e| Error::DapiClientError(e.to_string()))?;
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        return Err(Error::Cancelled("subscription worker cancelled".to_string()))
                    }
                    recv = stream.message() => match recv {
                        Ok(Some(item)) => {
                            let msgs = self.process(item).await;
                            for msg in msgs {
                                if let Err(e) = output.send(msg) {
                                    tracing::debug!("cannot send subscription message, possibly all listeners closed: {}", e);
                                    // we ignore this as this is a non-error condition
                                }
                            }
                        }

                        Ok(None) => {
                            tracing::error!("gRPC subscription stream closed, stopping");
                            return Err(Error::Cancelled("subscription worker failed, possibly sender closed the stream, stopping".to_string()));
                        }

                        Err(e) => {
                            tracing::warn!("gRPC error: {}", e);
                            // No return here, as we still want to process new messages
                        }
                    }
                }
            }
        }
    }

    /// Process individual message from the channel.
    ///
    /// Given a gRPC message received from gRPC stream, process it and
    /// return zero or more [Message]s to be sent to the channel.
    fn process(
        &self,
        input: M,
    ) -> impl std::future::Future<Output = Vec<SubscriptionMessage>> + std::marker::Send;
}

/// subscription that receives asset lock transactions from the stream
struct CoreSubscription {
    bloom_filter: BloomFilter,
    starting_block: Option<Vec<u8>>,
    dapi: Arc<DapiClient>,
    core_chain_locked_height: AtomicU32,
}

impl CoreSubscription {
    /// Return chainlock height for the given transaction.
    ///
    /// ## Returns
    ///
    /// * `u32` - Height of chainlock for the transaction
    // TODO : This is a temporary implementation until we have headers stream running in background
    async fn wait_for_tx_chainlock(
        &self,
        tx: &Txid,
        cancel: CancellationToken,
    ) -> Result<u32, SubscriptionError> {
        loop {
            let response = self
                .dapi
                .execute(
                    GetTransactionRequest { id: tx.to_string() },
                    RequestSettings::default(),
                )
                .await
                .map_err(|e| SubscriptionError::DapiClientError(e.to_string()))?;

            if response.is_chain_locked {
                return Ok(response.height);
            }

            tracing::trace!(
                "the transaction is on height {} but not chainlocked. try again in 1 sec",
                response.height,
            );

            tokio::select! {
                _ = cancel.cancelled() => {
                    return Err(SubscriptionError::Cancelled("watcher cancelled".to_string()))
                }

                _ = tokio::time::sleep(SLEEP) => {
                    continue;
                }
            }
        }
    }

    /// Waits until the platform chain is on the given chain locked height.
    ///
    /// Returns most recent core chain locked height. Returned value is always greater or equal to the requested height.
    async fn wait_for_core_chain_locked_height(
        &self,
        core_chain_locked_height: u32,
        cancel: CancellationToken,
    ) -> Result<u32, SubscriptionError> {
        // if we are already ahead, return the cached value
        let cached_locked_core_height = self.core_chain_locked_height.load(Ordering::Relaxed);
        if cached_locked_core_height >= core_chain_locked_height {
            return Ok(self.core_chain_locked_height.load(Ordering::Relaxed));
        }

        loop {
            let request = GetEpochsInfoRequest {
                version: Some(get_epochs_info_request::Version::V0(
                    get_epochs_info_request::GetEpochsInfoRequestV0 {
                        start_epoch: Some(0),
                        count: 1,
                        ..Default::default()
                    },
                )),
            };

            let GetEpochsInfoResponse {
                version:
                    Some(get_epochs_info_response::Version::V0(
                        get_epochs_info_response::GetEpochsInfoResponseV0 {
                            metadata: Some(metadata),
                            ..
                        },
                    )),
            } = self
                .dapi
                .execute(request, RequestSettings::default())
                .await
                .map_err(|e| SubscriptionError::DapiClientError(e.to_string()))?
            else {
                return Err(SubscriptionError::VersionError(
                    "incorrect response version".to_string(),
                ));
            };

            if metadata.core_chain_locked_height >= core_chain_locked_height {
                // we ignore the fact that some other thread might have updated the value in the meantime,
                // as this is just an optimization and we don't care if we do some extra work in some edge cases
                self.core_chain_locked_height
                    .store(metadata.core_chain_locked_height, Ordering::Relaxed);
                return Ok(metadata.core_chain_locked_height);
            }

            tracing::trace!(
                "platform chain locked core height {} but we need {}. try again in 1 sec",
                metadata.core_chain_locked_height,
                core_chain_locked_height,
            );

            tokio::select! {
                _ = cancel.cancelled() => {
                    return Err(SubscriptionError::Cancelled("watcher cancelled".to_string()))
                }

                _ = tokio::time::sleep(SLEEP) => {
                    continue;
                }
            }
        }
    }

    /*
    async fn process_merkle_block(&self,raw_merkle_block: Vec<u8>)->Result<AssetLockProof,SubscriptionError>{
        tracing::debug!("received merkle block");

        let merkle_block = MerkleBlock::consensus_decode(&mut raw_merkle_block.as_slice())
            .map_err(|e| {
                tracing::error!("can't decode merkle block: {}", e);


                SubscriptionError::CoreError(e.to_string())
            })?;

        let mut matches: Vec<Txid> = vec![];
        let mut index: Vec<u32> = vec![];

        merkle_block.extract_matches(&mut matches, &mut index).map_err(|e| {
            tracing::error!("can't extract matches from merkle block: {}", e);
            SubscriptionError::CoreError(e.to_string())
        })?;


        let mut asset_locks : Vec<AssetLockProof> = Vec::with_capacity(matches.len());

        for txid in matches {
            tracing::trace!(?txid, "merkle block contains transaction");

        let  core_chain_locked_height=self.wait_for_tx_chainlock(&txid).await?;

        tracing::trace!(
            ?txid,
        "the transaction is chainlocked on height {}, waiting platform for reaching the same core height",
        core_chain_locked_height
    );

        // Wait until platform chain is on the block's chain locked height
    self.wait_for_core_chain_locked_height(core_chain_locked_height).await;

        let asset_lock_proof = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height,
            out_point: OutPoint {
                txid,
                vout: 0, // FIXME: What should we put here?
            },
        });
        asset_locks.push(asset_lock_proof);
    };

    tracing::debug!(
        ?asset_locks,
        "merkle block contains asset locks for all the broadcasted transactions, returning chain asset lock proof"
    );

    return Ok(asset_locks);

    }
    */

    fn process_instant_send_locks(
        &self,
        instant_send_lock_messages: InstantSendLockMessages,
    ) -> Vec<SubscriptionMessage> {
        instant_send_lock_messages.messages.into_iter().map(|instant_lock_bytes|{


        let instant_lock = InstantLock::consensus_decode(
            &mut instant_lock_bytes.as_slice(),
        )
        .map_err(|e| {
            tracing::error!("invalid asset lock: {}", e);

            SubscriptionError::CoreError(e.to_string())
        })?;

        let msg = Message::InstantAssetLock {
            tx_id: instant_lock.txid,
            instant_lock,
        };
        tracing::debug!(
            message=?msg,
            "instant lock is matching to the broadcasted transaction, returning instant asset lock proof"
        );

        Ok(msg)
    }).collect()
    }

    fn process_raw_merkle_block(&self, raw_merkle_block: Vec<u8>) -> Vec<SubscriptionMessage> {
        let merkle_block = MerkleBlock::consensus_decode(&mut raw_merkle_block.as_slice())
            .map_err(|e| {
                tracing::error!("can't decode merkle block: {}", e);
                SubscriptionError::CoreError(e.to_string())
            })
            .map(|merkle_block| Message::MerkleBlock { merkle_block });

        vec![merkle_block]
    }
}

impl Subscription<TransactionsWithProofsRequest, TransactionsWithProofsResponse>
    for CoreSubscription
{
    fn request(&self) -> TransactionsWithProofsRequest {
        let bloom_filter_proto = {
            let BloomFilterData {
                v_data,
                n_hash_funcs,
                n_tweak,
                n_flags,
            } = self.bloom_filter.clone().into();
            dapi_grpc::core::v0::BloomFilter {
                v_data,
                n_hash_funcs,
                n_tweak,
                n_flags,
            }
        };
        let from_block = self
            .starting_block
            .as_ref()
            .map(|v| (transactions_with_proofs_request::FromBlock::FromBlockHash(v.to_vec())));

        TransactionsWithProofsRequest {
            bloom_filter: Some(bloom_filter_proto),
            count: 0, // Subscribing to new transactions as well
            send_transaction_hashes: true,
            from_block,
        }
    }

    async fn process(
        &self,
        input: TransactionsWithProofsResponse,
    ) -> Vec<Result<Message, SubscriptionError>> {
        let input = input.responses.ok_or(SubscriptionError::DapiClientError(
            "missing `responses` field".to_string(),
        ));

        match input {
            Err(e) => {
                vec![Err(e)]
            }

            Ok(transactions_with_proofs_response::Responses::InstantSendLockMessages(
                instant_send_lock_messages,
            )) => {
                tracing::trace!(
                    "received {} instant lock message(s)",
                    instant_send_lock_messages.messages.len()
                );

                self.process_instant_send_locks(instant_send_lock_messages)
            }

            Ok(transactions_with_proofs_response::Responses::RawMerkleBlock(raw_merkle_block)) => {
                tracing::trace!("received raw merkle block message",);

                self.process_raw_merkle_block(raw_merkle_block)
            }

            Ok(transactions_with_proofs_response::Responses::RawTransactions(_)) => {
                tracing::trace!("received transaction(s), ignoring");
                vec![]
            }
        }
    }
}
