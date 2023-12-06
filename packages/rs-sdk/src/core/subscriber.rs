//! Subscribe to events coming from the Core.

use std::{collections::BTreeMap, sync::Arc};

use bip37_bloom_filter::{BloomFilter, BloomFilterData};
use dapi_grpc::core::v0::{
    transactions_with_proofs_request, GetStatusRequest, TransactionsWithProofsRequest,
    TransactionsWithProofsResponse,
};
use dashcore_rpc::dashcore::Address;
use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use futures::Future;
use rs_dapi_client::{Dapi, DapiClient, RequestSettings};
use tokio::{
    runtime::Handle,
    sync::{
        broadcast::{Receiver, Sender},
        Notify,
    },
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::Error;

const CHANNEL_CAPACITY: usize = 100;
const DLQ_CAPACITY: usize = 100;

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
    events_rx: Receiver<Message>,

    threads: BTreeMap<Address, JoinHandle<()>>,
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
        let (events_tx, events_rx) = tokio::sync::broadcast::channel(CHANNEL_CAPACITY);

        Self {
            dapi: client,
            notifier: Arc::new(Notify::new()),
            threads: BTreeMap::new(),
            events_tx,
            events_rx,
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
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Message> {
        self.events_tx.subscribe()
    }

    /// Watch an address.
    ///
    /// This will start a new thread that will watch for events coming from the Core.
    ///
    /// # Panics
    ///
    /// This will panic if called outside the context of a Tokio runtime.
    /// See [Handle::current()](tokio::runtime::Handle) for more detailed explanation.
    pub fn watch_address(&mut self, address: Address) {
        let watcher = AddressWatcher::spawn(
            Arc::clone(&self.dapi),
            address,
            self.events_tx.clone(),
            self.notifier.clone(),
            self.cancel.clone(),
        );
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    AssetLock(AssetLockProof),
    Error { payload: Vec<u8>, error: String },
}

type AddressWatcherJoinHandle = JoinHandle<impl Future<Output = ()>>;
struct AddressWatcher {
    dapi: Arc<DapiClient>,
    address: Address,
    sender: Sender<Message>,
    notifier: Arc<Notify>,
    cancel: CancellationToken,
}

impl AddressWatcher {
    fn spawn(
        dapi: Arc<DapiClient>,
        address: Address,
        sender: Sender<Message>,
        notifier: Arc<Notify>,
        cancel: CancellationToken,
    ) -> JoinHandle<()> {
        let mut watcher = Self {
            dapi,
            address,
            sender,
            notifier,
            cancel,
        };

        let hdl = Handle::current();
        hdl.spawn(async move { watcher.worker(address) })
    }

    async fn worker(&self, address: Address) {
        self.start_stream(&address);
        todo!("not implemented")
    }

    pub async fn start_stream(
        &self,
        address: &Address,
    ) -> Result<dapi_grpc::tonic::Streaming<TransactionsWithProofsResponse>, Error> {
        // TODO: this is just a copy of
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
            .dapi
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
        self.dapi
            .execute(core_transactions_stream, RequestSettings::default())
            .await
            .map_err(|e| Error::DapiClientError(e.to_string()))
    }

    fn broadcast(&self, message: Message) {
        // we ignore send error here, because it means that there are no subscribers
        self.sender.send(message);
        self.notifier.notify_waiters();
    }
}
