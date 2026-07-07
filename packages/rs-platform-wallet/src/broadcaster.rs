//! Transaction broadcasting abstraction.
//!
//! Two implementations are provided:
//!
//! - [`DapiBroadcaster`] — broadcasts via Platform's DAPI gRPC (default for
//!   standalone wallets without SPV).
//! - [`SpvBroadcaster`] — broadcasts via SPV P2P peers (used when the wallet
//!   is managed by [`PlatformWalletManager`] with SPV enabled).

use std::sync::Arc;

use async_trait::async_trait;
use dashcore::{Transaction, Txid};

use crate::error::PlatformWalletError;
use crate::spv::SpvRuntime;

/// A failed broadcast, classified by whether the transaction could have
/// reached the network.
///
/// The classification decides whether the transaction's reserved inputs are
/// safe to release for an immediate retry (see
/// `wallet::reservations::broadcast_releasing_on_rejection`). Only the
/// broadcaster implementation has the transport knowledge to make this call,
/// so the distinction is part of the error type rather than a convention on
/// message strings.
#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    /// The transaction was definitively **not** handed to the network: the
    /// failure happened before any peer or endpoint received the bytes.
    /// Reserved inputs are safe to release and the send may be retried
    /// immediately.
    #[error("transaction broadcast rejected before reaching the network: {reason}")]
    Rejected { reason: String },

    /// The outcome is unknown — the transaction **may** already be on the
    /// network (transport timeout after delivery, partial peer send, or an
    /// internal multi-node retry whose earlier attempt may have succeeded).
    /// Reserved inputs must be kept; the reservation-TTL backstop or a later
    /// sync reconciles the outcome.
    #[error("transaction broadcast outcome unknown — it may already be on the network: {reason}")]
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
/// Implementations may use DAPI (gRPC), SPV (P2P peers), or Core RPC.
#[async_trait]
pub trait TransactionBroadcaster: Send + Sync {
    /// Contract: implementations must classify every failure as
    /// [`BroadcastError::Rejected`] **only** when the transaction provably
    /// never reached the network (no bytes handed to any peer or endpoint).
    /// Anything uncertain — timeouts after delivery, partial sends, retried
    /// submissions — must be [`BroadcastError::MaybeSent`], so callers keep
    /// the transaction's input reservation instead of releasing it into a
    /// potential double-spend.
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

/// Broadcasts transactions via SPV P2P peers.
///
/// Used when the wallet is managed by [`PlatformWalletManager`] with SPV.
pub struct SpvBroadcaster {
    spv: Arc<SpvRuntime>,
}

impl SpvBroadcaster {
    pub fn new(spv: Arc<SpvRuntime>) -> Self {
        Self { spv }
    }
}

#[async_trait]
impl TransactionBroadcaster for SpvBroadcaster {
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
        self.spv.broadcast_transaction(transaction).await?;
        Ok(transaction.txid())
    }
}
