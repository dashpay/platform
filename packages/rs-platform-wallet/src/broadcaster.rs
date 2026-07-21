//! Transaction broadcasting abstraction.
//!
//! Core acceptance is authoritative: a socket write to an SPV peer is not
//! enough to report success. Production SPV wallets first submit through
//! DAPI's Core `sendrawtransaction` bridge and only relay/inject the
//! transaction through SPV after Core has accepted it.

use std::sync::Arc;

use async_trait::async_trait;
use dashcore::{Transaction, Txid};

use crate::error::PlatformWalletError;
use crate::spv::SpvRuntime;

/// A failed broadcast, classified by whether Core definitively rejected the
/// transaction or its acceptance remains unknown.
///
/// The classification decides whether the transaction's reserved inputs are
/// safe to release for an immediate retry (see
/// `wallet::reservations::broadcast_releasing_on_rejection`).
#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    /// Core definitively did not accept the transaction. Reserved inputs are
    /// safe to release and the send may be retried.
    #[error("transaction broadcast rejected: {reason}")]
    Rejected { reason: String },

    /// The outcome is unknown — Core may already have accepted the
    /// transaction even though the response was lost. Reserved inputs must be
    /// kept until a later sync or the reservation-TTL backstop reconciles it.
    #[error("transaction broadcast outcome unknown — Core may already have accepted it: {reason}")]
    MaybeSent { reason: String },
}

impl From<BroadcastError> for PlatformWalletError {
    fn from(error: BroadcastError) -> Self {
        match error {
            BroadcastError::Rejected { reason } => {
                PlatformWalletError::TransactionBroadcast(reason)
            }
            BroadcastError::MaybeSent { reason } => {
                PlatformWalletError::TransactionBroadcastUnconfirmed(reason)
            }
        }
    }
}

/// Broadcasts a signed transaction to the Dash network.
///
/// `Ok(txid)` means an authoritative Core endpoint accepted the transaction
/// (or reported that it already knows it). A successful P2P socket write alone
/// must never satisfy this contract.
#[async_trait]
pub trait TransactionBroadcaster: Send + Sync {
    /// Contract: [`BroadcastError::Rejected`] is allowed only when Core
    /// definitively did not accept the transaction. Any timeout, transport
    /// ambiguity, or unverifiable response must be [`BroadcastError::MaybeSent`].
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError>;
}

#[derive(Debug)]
enum DapiSubmission {
    Accepted,
    AlreadyKnown,
    Rejected { reason: String },
    Uncertain { reason: String },
}

fn classify_dapi_response(txid: Txid, returned_txid: String) -> DapiSubmission {
    if returned_txid == txid.to_string() {
        DapiSubmission::Accepted
    } else {
        DapiSubmission::Uncertain {
            reason: format!(
                "DAPI returned transaction id '{returned_txid}' for submitted transaction {txid}"
            ),
        }
    }
}

fn classify_dapi_error(
    code: Option<dash_sdk::dapi_grpc::tonic::Code>,
    reason: String,
) -> DapiSubmission {
    use dash_sdk::dapi_grpc::tonic::Code;

    match code {
        Some(Code::AlreadyExists) => DapiSubmission::AlreadyKnown,
        Some(Code::InvalidArgument | Code::FailedPrecondition) => {
            DapiSubmission::Rejected { reason }
        }
        _ => DapiSubmission::Uncertain { reason },
    }
}

#[async_trait]
trait DapiCoreClient: Send + Sync {
    async fn submit(&self, transaction: &Transaction) -> DapiSubmission;

    async fn transaction_known(&self, txid: Txid) -> Result<bool, String>;
}

struct SdkDapiCoreClient {
    sdk: Arc<dash_sdk::Sdk>,
}

fn broadcast_request_settings() -> dash_sdk::dapi_client::RequestSettings {
    dash_sdk::dapi_client::RequestSettings {
        retries: Some(0),
        ..dash_sdk::dapi_client::RequestSettings::default()
    }
}

impl SdkDapiCoreClient {
    fn new(sdk: Arc<dash_sdk::Sdk>) -> Self {
        Self { sdk }
    }
}

fn grpc_code(
    error: &dash_sdk::dapi_client::DapiClientError,
) -> Option<dash_sdk::dapi_grpc::tonic::Code> {
    use dash_sdk::dapi_client::transport::TransportError;
    use dash_sdk::dapi_client::DapiClientError;

    match error {
        DapiClientError::Transport(TransportError::Grpc(status)) => Some(status.code()),
        DapiClientError::NoAvailableAddressesToRetry(error) => match error.as_ref() {
            TransportError::Grpc(status) => Some(status.code()),
        },
        _ => None,
    }
}

#[async_trait]
impl DapiCoreClient for SdkDapiCoreClient {
    async fn submit(&self, transaction: &Transaction) -> DapiSubmission {
        use dash_sdk::dapi_client::{DapiRequestExecutor, IntoInner};
        use dash_sdk::dapi_grpc::core::v0::BroadcastTransactionRequest;
        use dashcore::consensus;

        let txid = transaction.txid();
        let request = BroadcastTransactionRequest {
            transaction: consensus::serialize(transaction),
            allow_high_fees: false,
            bypass_limits: false,
        };

        // A broadcast is deliberately single-attempt. If the SDK retried after
        // a lost success response, a later node could report a conflict and we
        // could incorrectly release inputs for a transaction already accepted
        // by the first node.
        let settings = broadcast_request_settings();

        match self.sdk.execute(request, settings).await.into_inner() {
            Ok(response) => classify_dapi_response(txid, response.transaction_id),
            Err(error) => {
                classify_dapi_error(grpc_code(&error), format!("DAPI broadcast failed: {error}"))
            }
        }
    }

    async fn transaction_known(&self, txid: Txid) -> Result<bool, String> {
        match self.sdk.get_transaction(&txid.to_string()).await {
            Ok(Some(fetched)) if fetched.transaction.txid() == txid => Ok(true),
            Ok(Some(fetched)) => Err(format!(
                "DAPI getTransaction returned {} while reconciling {}",
                fetched.transaction.txid(),
                txid
            )),
            Ok(None) => Ok(false),
            Err(error) => Err(format!("DAPI getTransaction failed: {error}")),
        }
    }
}

/// Broadcasts transactions through Platform's DAPI Core bridge.
pub struct DapiBroadcaster {
    client: Arc<dyn DapiCoreClient>,
}

impl DapiBroadcaster {
    pub fn new(sdk: Arc<dash_sdk::Sdk>) -> Self {
        Self {
            client: Arc::new(SdkDapiCoreClient::new(sdk)),
        }
    }

    #[cfg(test)]
    fn from_client(client: Arc<dyn DapiCoreClient>) -> Self {
        Self { client }
    }

    async fn reconcile_uncertain(
        &self,
        txid: Txid,
        submit_reason: String,
    ) -> Result<Txid, BroadcastError> {
        match self.client.transaction_known(txid).await {
            Ok(true) => Ok(txid),
            Ok(false) => Err(BroadcastError::MaybeSent {
                reason: format!("{submit_reason}; follow-up getTransaction did not find {txid}"),
            }),
            Err(lookup_reason) => Err(BroadcastError::MaybeSent {
                reason: format!("{submit_reason}; {lookup_reason}"),
            }),
        }
    }
}

#[async_trait]
impl TransactionBroadcaster for DapiBroadcaster {
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
        let txid = transaction.txid();
        match self.client.submit(transaction).await {
            DapiSubmission::Accepted | DapiSubmission::AlreadyKnown => Ok(txid),
            DapiSubmission::Rejected { reason } => Err(BroadcastError::Rejected { reason }),
            DapiSubmission::Uncertain { reason } => self.reconcile_uncertain(txid, reason).await,
        }
    }
}

#[async_trait]
trait AcceptedTransactionRelay: Send + Sync {
    async fn relay(&self, transaction: &Transaction) -> Result<(), BroadcastError>;
}

#[async_trait]
impl AcceptedTransactionRelay for SpvRuntime {
    async fn relay(&self, transaction: &Transaction) -> Result<(), BroadcastError> {
        self.broadcast_transaction(transaction).await
    }
}

/// Uses DAPI/Core as the acceptance authority, then relays the accepted
/// transaction through SPV so its local mempool pipeline sees it immediately.
pub struct SpvBroadcaster {
    authority: DapiBroadcaster,
    relay: Arc<dyn AcceptedTransactionRelay>,
}

impl SpvBroadcaster {
    pub fn new(spv: Arc<SpvRuntime>, sdk: Arc<dash_sdk::Sdk>) -> Self {
        Self {
            authority: DapiBroadcaster::new(sdk),
            relay: spv,
        }
    }

    #[cfg(test)]
    fn from_components(
        authority: DapiBroadcaster,
        relay: Arc<dyn AcceptedTransactionRelay>,
    ) -> Self {
        Self { authority, relay }
    }
}

#[async_trait]
impl TransactionBroadcaster for SpvBroadcaster {
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
        let txid = self.authority.broadcast(transaction).await?;

        // Core acceptance is final for this API. SPV propagation/local apply is
        // best-effort after that point; a failure is healed by normal SPV sync.
        if let Err(error) = self.relay.relay(transaction).await {
            tracing::warn!(
                txid = %txid,
                error = %error,
                "Core accepted transaction but SPV relay/local apply failed"
            );
        }

        Ok(txid)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use dashcore::hashes::Hash;

    use super::*;

    struct FakeDapiClient {
        submission: Mutex<Option<DapiSubmission>>,
        known: Result<bool, String>,
        lookup_calls: AtomicUsize,
    }

    impl FakeDapiClient {
        fn new(submission: DapiSubmission, known: Result<bool, String>) -> Self {
            Self {
                submission: Mutex::new(Some(submission)),
                known,
                lookup_calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl DapiCoreClient for FakeDapiClient {
        async fn submit(&self, _transaction: &Transaction) -> DapiSubmission {
            self.submission
                .lock()
                .expect("submission mutex")
                .take()
                .expect("one submission")
        }

        async fn transaction_known(&self, _txid: Txid) -> Result<bool, String> {
            self.lookup_calls.fetch_add(1, Ordering::SeqCst);
            self.known.clone()
        }
    }

    struct RelaySpy {
        calls: AtomicUsize,
        result: Mutex<Option<Result<(), BroadcastError>>>,
    }

    impl RelaySpy {
        fn succeeding() -> Self {
            Self {
                calls: AtomicUsize::new(0),
                result: Mutex::new(Some(Ok(()))),
            }
        }

        fn failing() -> Self {
            Self {
                calls: AtomicUsize::new(0),
                result: Mutex::new(Some(Err(BroadcastError::MaybeSent {
                    reason: "relay failed".to_string(),
                }))),
            }
        }
    }

    #[async_trait]
    impl AcceptedTransactionRelay for RelaySpy {
        async fn relay(&self, _transaction: &Transaction) -> Result<(), BroadcastError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.result
                .lock()
                .expect("relay mutex")
                .take()
                .expect("one relay")
        }
    }

    fn transaction() -> Transaction {
        Transaction {
            version: 1,
            lock_time: 0,
            input: Vec::new(),
            output: Vec::new(),
            special_transaction_payload: None,
        }
    }

    fn dapi_broadcaster(client: Arc<FakeDapiClient>) -> DapiBroadcaster {
        DapiBroadcaster::from_client(client)
    }

    #[tokio::test]
    async fn accepted_and_already_known_are_successful_without_lookup() {
        for submission in [DapiSubmission::Accepted, DapiSubmission::AlreadyKnown] {
            let client = Arc::new(FakeDapiClient::new(submission, Ok(false)));
            let result = dapi_broadcaster(client.clone())
                .broadcast(&transaction())
                .await;

            assert!(result.is_ok());
            assert_eq!(client.lookup_calls.load(Ordering::SeqCst), 0);
        }
    }

    #[tokio::test]
    async fn explicit_rejection_is_not_reconciled() {
        let client = Arc::new(FakeDapiClient::new(
            DapiSubmission::Rejected {
                reason: "policy rejection".to_string(),
            },
            Ok(true),
        ));
        let result = dapi_broadcaster(client.clone())
            .broadcast(&transaction())
            .await;

        assert!(matches!(result, Err(BroadcastError::Rejected { .. })));
        assert_eq!(client.lookup_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn uncertain_submission_is_accepted_only_when_lookup_finds_it() {
        for (known, accepted) in [(Ok(true), true), (Ok(false), false)] {
            let client = Arc::new(FakeDapiClient::new(
                DapiSubmission::Uncertain {
                    reason: "timeout".to_string(),
                },
                known,
            ));
            let result = dapi_broadcaster(client.clone())
                .broadcast(&transaction())
                .await;

            assert_eq!(result.is_ok(), accepted);
            if !accepted {
                assert!(matches!(result, Err(BroadcastError::MaybeSent { .. })));
            }
            assert_eq!(client.lookup_calls.load(Ordering::SeqCst), 1);
        }
    }

    #[tokio::test]
    async fn lookup_failure_keeps_uncertain_submission_unknown() {
        let client = Arc::new(FakeDapiClient::new(
            DapiSubmission::Uncertain {
                reason: "timeout".to_string(),
            },
            Err("lookup unavailable".to_string()),
        ));
        let result = dapi_broadcaster(client).broadcast(&transaction()).await;

        assert!(matches!(result, Err(BroadcastError::MaybeSent { .. })));
    }

    #[tokio::test]
    async fn spv_relay_runs_only_after_core_acceptance() {
        let accepted_client = Arc::new(FakeDapiClient::new(DapiSubmission::Accepted, Ok(false)));
        let accepted_relay = Arc::new(RelaySpy::succeeding());
        let broadcaster = SpvBroadcaster::from_components(
            dapi_broadcaster(accepted_client),
            accepted_relay.clone(),
        );
        assert!(broadcaster.broadcast(&transaction()).await.is_ok());
        assert_eq!(accepted_relay.calls.load(Ordering::SeqCst), 1);

        let rejected_client = Arc::new(FakeDapiClient::new(
            DapiSubmission::Rejected {
                reason: "rejected".to_string(),
            },
            Ok(false),
        ));
        let rejected_relay = Arc::new(RelaySpy::succeeding());
        let broadcaster = SpvBroadcaster::from_components(
            dapi_broadcaster(rejected_client),
            rejected_relay.clone(),
        );
        assert!(matches!(
            broadcaster.broadcast(&transaction()).await,
            Err(BroadcastError::Rejected { .. })
        ));
        assert_eq!(rejected_relay.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn relay_failure_does_not_downgrade_core_acceptance() {
        let client = Arc::new(FakeDapiClient::new(DapiSubmission::Accepted, Ok(false)));
        let relay = Arc::new(RelaySpy::failing());
        let broadcaster = SpvBroadcaster::from_components(dapi_broadcaster(client), relay.clone());

        assert!(broadcaster.broadcast(&transaction()).await.is_ok());
        assert_eq!(relay.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn tonic_submission_codes_map_to_authoritative_states() {
        use dash_sdk::dapi_grpc::tonic::Code;

        assert!(matches!(
            classify_dapi_error(Some(Code::AlreadyExists), "known".to_string()),
            DapiSubmission::AlreadyKnown
        ));
        for code in [Code::InvalidArgument, Code::FailedPrecondition] {
            assert!(matches!(
                classify_dapi_error(Some(code), "rejected".to_string()),
                DapiSubmission::Rejected { .. }
            ));
        }
        for code in [Code::DeadlineExceeded, Code::Unavailable] {
            assert!(matches!(
                classify_dapi_error(Some(code), "transport".to_string()),
                DapiSubmission::Uncertain { .. }
            ));
        }
    }

    #[test]
    fn dapi_submission_disables_sdk_retries() {
        assert_eq!(broadcast_request_settings().retries, Some(0));
    }

    #[test]
    fn empty_or_mismatched_response_txid_requires_reconciliation() {
        let expected = transaction().txid();
        for returned in [String::new(), Txid::all_zeros().to_string()] {
            assert!(matches!(
                classify_dapi_response(expected, returned),
                DapiSubmission::Uncertain { .. }
            ));
        }
        assert!(matches!(
            classify_dapi_response(expected, expected.to_string()),
            DapiSubmission::Accepted
        ));
    }
}
