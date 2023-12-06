//! Subscribe to events coming from the Core.

use std::{collections::BTreeMap, sync::Arc};

use bip37_bloom_filter::{BloomFilter, BloomFilterData};
use dapi_grpc::core::v0::{
    transactions_with_proofs_request, TransactionsWithProofsRequest, TransactionsWithProofsResponse,
};
use dashcore_rpc::dashcore::Address;
use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use rs_dapi_client::{Dapi, DapiClient, RequestSettings};
use tokio::{
    runtime::Handle,
    sync::{
        broadcast::{Receiver, Sender},
        Mutex, Notify,
    },
};
use tokio_util::sync::CancellationToken;

use crate::Error;

const CHANNEL_CAPACITY: usize = 100;

/// Subscriber to events coming from the Core.
///
/// Controller of the whole subscription mechanism.
/// Calling [watch_address()] starts a new thread to watch all events releated to some address.
/// The app can subscribe to these events either with or [notify()] (to get notifications when some message arrives)
/// or [subscribe()] (to get channel returning retrieved messages).
///
/// # Error handling
///
/// In case of an error during processing, [Message::Error] will be sent to the channel.
#[derive(Debug)]
pub struct Subscriber {
    dapi: Arc<DapiClient>,
    notifier: Arc<tokio::sync::Notify>,
    events_tx: Sender<Message>,

    /// Map of threads watching for events, indexed by their ID.
    threads: Arc<Mutex<BTreeMap<u64, CancellationToken>>>,
    /// Id of last created thread
    last_thread_id: u64,

    cancel: CancellationToken,
}

/// Controller of the whole subscription mechanism.
impl Subscriber {
    /// Create new controller for Dash Core event subscriptions.
    ///
    /// Creates new controller that will manage Dash Core event subscriptions.
    /// No subscriptions will be started until [watch_address()] is called.
    ///
    /// # Arguments
    ///
    /// * `client` - Client to use to connect to core.

    pub fn new(client: Arc<DapiClient>, cancel: CancellationToken) -> Self {
        let (events_tx, _events_rx) = tokio::sync::broadcast::channel(CHANNEL_CAPACITY);

        Self {
            dapi: client,
            notifier: Arc::new(Notify::new()),
            threads: Default::default(),
            last_thread_id: 0,
            events_tx,
            cancel,
        }
    }

    /// Get notification channel.
    ///
    /// This channel will be notified whenever a new message is received.
    pub fn notify(&self) -> Arc<tokio::sync::Notify> {
        self.notifier.clone()
    }

    /// Subscribe to events coming from the Core.
    ///
    /// Subscribe to events coming from the Core. This will start a new thread that will
    ///
    pub fn subscribe(&self) -> Receiver<Message> {
        self.events_tx.subscribe()
    }

    /// Create a new stream subscription that will retrieve events matching the given bloom filter.
    ///
    /// This will start a new thread that will watch for events coming from the Core.
    ///
    /// # Arguments
    ///
    /// * `bloom_filter` - Bloom filter to use to filter events.
    /// * `start_from_block_hash` - Block hash to start from. If `None`, the stream will start from the latest block.
    ///
    /// # Returns
    ///
    /// * `u64` - ID of the watch thread that can be used to [stop] it.
    pub async fn watch_bloom_filter(
        &mut self,
        bloom_filter: BloomFilter,
        start_from_block_hash: Option<&[u8]>,
    ) -> u64 {
        //
        let token = self.cancel.child_token();

        // Register new thread
        self.last_thread_id += 1;
        let thread_id = self.last_thread_id;

        let mut guard = self.threads.lock().await;
        guard.insert(self.last_thread_id, token.clone());
        drop(guard);

        // Start the thread
        let hdl = Handle::current();
        let threads = Arc::clone(&self.threads);
        let events_tx = self.events_tx.clone();
        let notifier = self.notifier.clone();
        let dapi = Arc::clone(&self.dapi);
        let starting_block = start_from_block_hash.map(|v| v.to_vec());

        hdl.spawn(async move {
            let watcher = Watcher::new(
                bloom_filter.clone(),
                starting_block.clone(),
                dapi,
                events_tx,
                notifier,
                token,
            );

            if let Err(e) = watcher.worker().await {
                tracing::error!(thread_id, error = ?e, ?starting_block,?bloom_filter, "core subscription watcher failed");
            }

            threads.lock().await.remove(&thread_id);
        });

        thread_id
    }

    /// Watch an address.
    ///
    /// This will start a new thread that will watch for events coming from the Core.
    ///
    /// # Panics
    ///
    /// This will panic if called outside the context of a Tokio runtime.
    /// See [Handle::current()](tokio::runtime::Handle) for more detailed explanation.
    pub async fn watch_address(&mut self, address: Address, start_from_block_hash: Option<&[u8]>) {
        let address_bytes = address.as_unchecked().payload_to_vec();

        // create the bloom filter
        let bloom_filter = BloomFilter::builder(1, 0.001)
            .expect("this FP rate allows up to 10000 items")
            .add_element(&address_bytes)
            .build();

        self.watch_bloom_filter(bloom_filter, start_from_block_hash)
            .await;
    }

    /// Stop thread watching for events.
    pub async fn stop(&mut self, thread_id: u64) {
        let guard = self.threads.lock().await;
        if let Some(cancel) = guard.get(&thread_id) {
            cancel.cancel();
        } else {
            tracing::warn!(thread_id, "cannot stop core watcher thread: not found",);
        }
    }
}

/// Message retrieved from the Core.
#[derive(Debug, Clone)]
pub enum Message {
    /// Asset lock proof received.
    ///
    /// We use Box here because AssetLockProof is huge, and it's not recommended to pass it by value into enum.
    AssetLock(Box<AssetLockProof>),
    /// Error occured.
    Error { payload: Vec<u8>, error: String },
}

struct Watcher {
    dapi: Arc<DapiClient>,
    sender: Sender<Message>,
    notifier: Arc<Notify>,
    cancel: CancellationToken,

    bloom_filter: BloomFilter,
    starting_block: Option<Vec<u8>>,
}

impl Watcher {
    fn new(
        bloom_filter: BloomFilter,
        starting_block: Option<Vec<u8>>,

        dapi: Arc<DapiClient>,
        sender: Sender<Message>,
        notifier: Arc<Notify>,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            dapi,
            bloom_filter,
            sender,
            notifier,
            cancel,
            starting_block,
        }
    }

    async fn worker(self) -> Result<(), Error> {
        let mut stream = self.start_stream().await?;
        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => {
                    return Err(Error::Cancelled("watcher cancelled".to_string()))
                }

                Ok(Some(item)) = stream.message() => {
                    let msg = self.process_item(item).await;
                    self.broadcast(msg);
                }
            }
        }
    }

    pub async fn start_stream(
        &self,
    ) -> Result<dapi_grpc::tonic::Streaming<TransactionsWithProofsResponse>, Error> {
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

        let core_transactions_stream = TransactionsWithProofsRequest {
            bloom_filter: Some(bloom_filter_proto),
            count: 0, // Subscribing to new transactions as well
            send_transaction_hashes: true,
            from_block,
        };
        self.dapi
            .execute(core_transactions_stream, RequestSettings::default())
            .await
            .map_err(|e| Error::DapiClientError(e.to_string()))
    }

    pub async fn process_item(&self, item: TransactionsWithProofsResponse) -> Message {
        self.error_item(&item, "not implemented")
    }

    pub fn error_item(&self, item: &TransactionsWithProofsResponse, msg: &str) -> Message {
        let payload =
            serde_json::to_vec(item).unwrap_or("JSON serialization error".as_bytes().to_vec());
        Message::Error {
            payload,
            error: msg.to_string(),
        }
    }

    fn broadcast(&self, message: Message) {
        // we ignore send error here, because it means that there are no subscribers
        self.sender.send(message).ok();
        self.notifier.notify_waiters();
    }
}
