//! Transaction broadcasting abstraction.
//!
//! Network acceptance, not a socket write, is what counts as success for the
//! production path:
//!
//! - [`SpvBroadcaster`] (production, SPV wallets): pure P2P via dash-spv's
//!   acceptance detection (rust-dashcore#913) — the transaction is sent to a
//!   subset of peers while withheld from the rest, and a withheld peer
//!   announcing the txid back, an InstantSend lock, or a confirmation proves
//!   the network accepted it. Trustless; no DAPI involvement.
//! - [`DapiBroadcaster`] (fallback for wallets without an SPV runtime):
//!   submission via DAPI's gRPC endpoint, with every failure conservatively
//!   classified as [`BroadcastError::MaybeSent`].

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

/// Broadcasts transactions via Platform's DAPI gRPC endpoint.
///
/// Used by default when no SPV runtime is available.
pub struct DapiBroadcaster {
    sdk: Arc<dash_sdk::Sdk>,
}

impl DapiBroadcaster {
    pub fn new(sdk: Arc<dash_sdk::Sdk>) -> Self {
        Self { sdk }
    }
}

#[async_trait]
impl TransactionBroadcaster for DapiBroadcaster {
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
        use dash_sdk::dapi_client::{DapiRequestExecutor, IntoInner, RequestSettings};
        use dash_sdk::dapi_grpc::core::v0::BroadcastTransactionRequest;
        use dashcore::consensus;

        let tx_bytes = consensus::serialize(transaction);

        let request = BroadcastTransactionRequest {
            transaction: tx_bytes,
            allow_high_fees: false,
            bypass_limits: false,
        };

        // Every DAPI failure is classified `MaybeSent`: `sdk.execute` retries
        // across nodes internally (RequestSettings::default()), so the error
        // surfaced here is only the *last* attempt's — an earlier attempt may
        // have delivered the transaction even though the response was lost
        // (the classic shape being a node that accepts the tx while its gRPC
        // response times out, followed by a retry that fails differently).
        // Distinguishing a genuinely pre-send rejection would require
        // disabling the internal retries and inspecting transport errors;
        // until then the conservative classification keeps reserved inputs
        // safe from double-spends at the cost of holding them for the
        // reservation TTL.
        let _response = self
            .sdk
            .execute(request, RequestSettings::default())
            .await
            .into_inner()
            .map_err(|e| BroadcastError::MaybeSent {
                reason: format!("DAPI broadcast failed: {}", e),
            })?;

        Ok(transaction.txid())
    }
}

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
        match self.spv.broadcast_and_wait(transaction, None).await {
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
                reason:
                    "SPV broadcast saw no acceptance signal before dash-spv's acceptance timeout"
                        .to_string(),
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

    use super::*;

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
