//! Transaction broadcasting abstraction.
//!
//! Network acceptance, not a socket write, is what counts as success. Two
//! implementations establish it, chosen by how the wallet runs:
//!
//! - [`SpvBroadcaster`] (production, SPV wallets): pure P2P via dash-spv's
//!   acceptance detection (rust-dashcore#913) — the transaction is sent to a
//!   subset of peers while withheld from the rest, and a withheld peer
//!   announcing the txid back, an InstantSend lock, or a confirmation proves
//!   the network accepted it. Trustless; no DAPI involvement.
//! - [`DapiBroadcaster`] (standalone wallets without an SPV runtime):
//!   submission through DAPI's Core `sendrawtransaction` bridge, with
//!   ambiguous responses reconciled via `getTransaction`.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dash_spv::BroadcastResult;
use dashcore::{Transaction, Txid};

use crate::error::PlatformWalletError;
use crate::spv::SpvRuntime;

/// A failed broadcast, classified by whether the transaction definitively
/// never entered the network or its acceptance remains unknown.
///
/// The classification decides whether the transaction's reserved inputs are
/// safe to release for an immediate retry (see
/// `wallet::reservations::broadcast_releasing_on_rejection`).
#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    /// The network definitively did not take the transaction — a rejection
    /// from the submitting endpoint, or a failure that provably preceded any
    /// send. Reserved inputs are safe to release and the send may be retried.
    #[error("transaction broadcast rejected: {reason}")]
    Rejected { reason: String },

    /// The outcome is unknown — the network may already have accepted the
    /// transaction even though no signal confirmed it. Reserved inputs must
    /// be kept until a later sync or the reservation-TTL backstop reconciles
    /// it.
    #[error("transaction broadcast outcome unknown — the network may already have accepted it: {reason}")]
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
/// `Ok(txid)` means the network accepted the transaction — proven by SPV
/// peer echo / InstantSend lock / confirmation, or by an accepting Core
/// endpoint. A successful P2P socket write alone must never satisfy this
/// contract.
#[async_trait]
pub trait TransactionBroadcaster: Send + Sync {
    /// Contract: [`BroadcastError::Rejected`] is allowed only when the
    /// transaction definitively did not enter the network. Any timeout,
    /// transport ambiguity, or unverifiable response must be
    /// [`BroadcastError::MaybeSent`].
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError>;
}

#[derive(Debug)]
enum DapiSubmission {
    Accepted,
    AlreadyKnown,
    Rejected {
        reason: String,
    },
    /// The request provably never left the process (e.g. the DAPI address
    /// list is empty). Unlike `Uncertain`, nothing was submitted anywhere.
    NeverSent {
        reason: String,
    },
    Uncertain {
        reason: String,
    },
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
    error: &dash_sdk::dapi_client::DapiClientError,
    reason: String,
) -> DapiSubmission {
    use dash_sdk::dapi_client::DapiClientError;
    use dash_sdk::dapi_grpc::tonic::Code;

    // These fire before any request is dispatched: nothing reached the
    // network, so the submission is provably never-sent rather than
    // ambiguous. (`NoAvailableAddressesToRetry` is different — addresses
    // were tried and banned, so an earlier attempt may have delivered.)
    if matches!(
        error,
        DapiClientError::NoAvailableAddresses | DapiClientError::AddressList(_)
    ) {
        return DapiSubmission::NeverSent { reason };
    }

    match grpc_code(error) {
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
                let reason = format!("DAPI broadcast failed: {error}");
                classify_dapi_error(&error, reason)
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

    /// Submit through DAPI and reconcile ambiguous responses via
    /// `getTransaction`, keeping the never-sent case distinguishable so a
    /// caller with another delivery channel (SPV) can still try it.
    async fn submit_outcome(&self, transaction: &Transaction) -> DapiOutcome {
        let txid = transaction.txid();
        match self.client.submit(transaction).await {
            DapiSubmission::Accepted | DapiSubmission::AlreadyKnown => DapiOutcome::Accepted,
            DapiSubmission::Rejected { reason } => DapiOutcome::Rejected { reason },
            // Nothing was submitted, so there is nothing to reconcile.
            DapiSubmission::NeverSent { reason } => DapiOutcome::NeverSent { reason },
            DapiSubmission::Uncertain { reason } => {
                match self.client.transaction_known(txid).await {
                    Ok(true) => DapiOutcome::Accepted,
                    Ok(false) => DapiOutcome::Uncertain {
                        reason: format!("{reason}; follow-up getTransaction did not find {txid}"),
                    },
                    Err(lookup_reason) => DapiOutcome::Uncertain {
                        reason: format!("{reason}; {lookup_reason}"),
                    },
                }
            }
        }
    }
}

/// Post-reconciliation DAPI outcome.
#[derive(Debug)]
enum DapiOutcome {
    Accepted,
    Rejected {
        reason: String,
    },
    /// The transaction provably never left the process through DAPI.
    NeverSent {
        reason: String,
    },
    Uncertain {
        reason: String,
    },
}

#[async_trait]
impl TransactionBroadcaster for DapiBroadcaster {
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
        let txid = transaction.txid();
        match self.submit_outcome(transaction).await {
            DapiOutcome::Accepted => Ok(txid),
            DapiOutcome::Rejected { reason } => Err(BroadcastError::Rejected { reason }),
            // With DAPI as the only channel, never-sent is definitive: no
            // bytes reached the network, so the reservation is safe to
            // release for an immediate retry.
            DapiOutcome::NeverSent { reason } => Err(BroadcastError::Rejected { reason }),
            DapiOutcome::Uncertain { reason } => Err(BroadcastError::MaybeSent { reason }),
        }
    }
}

/// How long the SPV broadcast waits for a network-acceptance verdict before
/// reporting the outcome as unknown. Shorter than dash-spv's own default so a
/// user-facing send does not hang for a full minute. On live Dash networks
/// acceptance usually resolves in seconds via the InstantSend lock or the
/// withheld-peer echo, well inside this bound.
const SPV_ACCEPTANCE_TIMEOUT: Duration = Duration::from_secs(30);

/// The SPV broadcast channel: send through P2P peers and await dash-spv's
/// network-acceptance verdict (rust-dashcore#913).
#[async_trait]
trait SpvChannel: Send + Sync {
    /// Broadcast through SPV peers and await dash-spv's network-acceptance
    /// verdict (a withheld peer announcing the txid back, an InstantSend
    /// lock, or a confirmation proves propagation).
    ///
    /// Error contract: `Err(BroadcastError::Rejected)` means the transaction
    /// provably never entered the SPV send pipeline (client not started, or
    /// zero connected peers checked before any dispatch); anything that may
    /// follow a partial send is `Err(BroadcastError::MaybeSent)`.
    async fn broadcast_and_wait(
        &self,
        transaction: &Transaction,
        timeout: Option<Duration>,
    ) -> Result<BroadcastResult, BroadcastError>;
}

#[async_trait]
impl SpvChannel for SpvRuntime {
    async fn broadcast_and_wait(
        &self,
        transaction: &Transaction,
        timeout: Option<Duration>,
    ) -> Result<BroadcastResult, BroadcastError> {
        self.broadcast_transaction_and_wait(transaction, timeout)
            .await
    }
}

/// Broadcasts purely over the SPV P2P network — no DAPI involvement.
///
/// dash-spv sends the transaction to a subset of connected peers while
/// withholding it from the rest; a withheld peer announcing the txid back
/// (or an InstantSend lock / confirmation) proves the network accepted it.
/// dash-spv also injects the transaction into its local mempool pipeline,
/// so the wallet sees it immediately without a separate relay step.
pub struct SpvBroadcaster {
    spv: Arc<dyn SpvChannel>,
}

impl SpvBroadcaster {
    pub fn new(spv: Arc<SpvRuntime>) -> Self {
        Self { spv }
    }

    #[cfg(test)]
    fn from_channel(spv: Arc<dyn SpvChannel>) -> Self {
        Self { spv }
    }
}

#[async_trait]
impl TransactionBroadcaster for SpvBroadcaster {
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
        let txid = transaction.txid();
        match self
            .spv
            .broadcast_and_wait(transaction, Some(SPV_ACCEPTANCE_TIMEOUT))
            .await
        {
            Ok(BroadcastResult::Accepted { relayed_by }) => {
                tracing::info!(
                    txid = %txid,
                    relayed_by,
                    "SPV broadcast accepted by the network"
                );
                Ok(txid)
            }
            // The p2p network has no negative signal (modern Dash Core
            // removed BIP61 reject), so no-echo is the only failure shape —
            // and the transaction DID go out to SPV peers, so the outcome
            // must stay ambiguous. dash-spv keeps rebroadcasting it, and a
            // later echo/IS-lock/confirmation or the reservation-TTL
            // backstop reconciles the reservation.
            Ok(BroadcastResult::Uncertain) => Err(BroadcastError::MaybeSent {
                reason: format!(
                    "SPV broadcast saw no acceptance signal within {SPV_ACCEPTANCE_TIMEOUT:?}"
                ),
            }),
            // Provably never sent (per the SpvChannel error contract): no
            // bytes reached the network, so the reservation is safe to
            // release for an immediate retry.
            Err(never_sent @ BroadcastError::Rejected { .. }) => Err(never_sent),
            Err(other) => Err(other),
        }
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

    struct AcceptanceSpy {
        calls: AtomicUsize,
        verdict: Mutex<Option<Result<BroadcastResult, BroadcastError>>>,
    }

    impl AcceptanceSpy {
        fn with(verdict: Result<BroadcastResult, BroadcastError>) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                verdict: Mutex::new(Some(verdict)),
            }
        }
    }

    #[async_trait]
    impl SpvChannel for AcceptanceSpy {
        async fn broadcast_and_wait(
            &self,
            _transaction: &Transaction,
            _timeout: Option<Duration>,
        ) -> Result<BroadcastResult, BroadcastError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.verdict
                .lock()
                .expect("verdict mutex")
                .take()
                .expect("one acceptance check")
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

    /// DAPI-only broadcaster: a never-sent submission is definitive (safe to
    /// release) and needs no getTransaction reconciliation.
    #[tokio::test]
    async fn never_sent_submission_is_definitive_without_lookup() {
        let client = Arc::new(FakeDapiClient::new(
            DapiSubmission::NeverSent {
                reason: "no addresses".to_string(),
            },
            Ok(true),
        ));
        let result = dapi_broadcaster(client.clone())
            .broadcast(&transaction())
            .await;

        assert!(matches!(result, Err(BroadcastError::Rejected { .. })));
        assert_eq!(client.lookup_calls.load(Ordering::SeqCst), 0);
    }

    fn grpc_error(
        code: dash_sdk::dapi_grpc::tonic::Code,
    ) -> dash_sdk::dapi_client::DapiClientError {
        use dash_sdk::dapi_client::transport::TransportError;
        use dash_sdk::dapi_client::DapiClientError;
        use dash_sdk::dapi_grpc::tonic::Status;

        DapiClientError::Transport(TransportError::Grpc(Status::new(code, "test")))
    }

    #[test]
    fn tonic_submission_codes_map_to_authoritative_states() {
        use dash_sdk::dapi_grpc::tonic::Code;

        assert!(matches!(
            classify_dapi_error(&grpc_error(Code::AlreadyExists), "known".to_string()),
            DapiSubmission::AlreadyKnown
        ));
        for code in [Code::InvalidArgument, Code::FailedPrecondition] {
            assert!(matches!(
                classify_dapi_error(&grpc_error(code), "rejected".to_string()),
                DapiSubmission::Rejected { .. }
            ));
        }
        for code in [Code::DeadlineExceeded, Code::Unavailable] {
            assert!(matches!(
                classify_dapi_error(&grpc_error(code), "transport".to_string()),
                DapiSubmission::Uncertain { .. }
            ));
        }
    }

    /// `NoAvailableAddresses` fires before any request is dispatched, so it
    /// must classify never-sent — not ambiguous. (Regression for the stuck
    /// UTXO reservation reported on the predecessor PR: unknown kept the
    /// reservation and the retry failed with artificial insufficient funds.)
    #[test]
    fn empty_address_list_classifies_never_sent() {
        use dash_sdk::dapi_client::DapiClientError;

        assert!(matches!(
            classify_dapi_error(
                &DapiClientError::NoAvailableAddresses,
                "no addresses".to_string()
            ),
            DapiSubmission::NeverSent { .. }
        ));
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

    /// The SPV broadcaster maps dash-spv's verdict directly: peer-echo
    /// acceptance succeeds, silence stays ambiguous (reservation kept), and
    /// a provably-never-sent failure is definitive (reservation released).
    #[tokio::test]
    async fn spv_broadcast_maps_dash_spv_verdicts() {
        let cases: Vec<(Result<BroadcastResult, BroadcastError>, &str)> = vec![
            (Ok(BroadcastResult::Accepted { relayed_by: 1 }), "accepted"),
            (Ok(BroadcastResult::Uncertain), "uncertain"),
            (
                Err(BroadcastError::Rejected {
                    reason: "SPV broadcast not sent: client not started".to_string(),
                }),
                "never-sent",
            ),
            (
                Err(BroadcastError::MaybeSent {
                    reason: "event bus closed".to_string(),
                }),
                "transport",
            ),
        ];

        for (verdict, label) in cases {
            let spv = Arc::new(AcceptanceSpy::with(verdict));
            let broadcaster = SpvBroadcaster::from_channel(spv.clone());

            let result = broadcaster.broadcast(&transaction()).await;

            assert_eq!(spv.calls.load(Ordering::SeqCst), 1, "{label}");
            match label {
                "accepted" => assert!(result.is_ok(), "{label}: got {result:?}"),
                "never-sent" => assert!(
                    matches!(result, Err(BroadcastError::Rejected { .. })),
                    "{label}: never-sent must release the reservation, got {result:?}"
                ),
                _ => assert!(
                    matches!(result, Err(BroadcastError::MaybeSent { .. })),
                    "{label}: got {result:?}"
                ),
            }
        }
    }
}
