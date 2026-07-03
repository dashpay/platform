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

/// Broadcasts a signed transaction to the Dash network.
///
/// Implementations may use DAPI (gRPC), SPV (P2P peers), or Core RPC.
#[async_trait]
pub trait TransactionBroadcaster: Send + Sync {
    /// Contract: `Err` must mean the transaction was **not** accepted by the
    /// network, so callers may safely release its reserved inputs and retry.
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, PlatformWalletError>;
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
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, PlatformWalletError> {
        use dash_sdk::dapi_client::{DapiRequestExecutor, IntoInner, RequestSettings};
        use dash_sdk::dapi_grpc::core::v0::BroadcastTransactionRequest;
        use dashcore::consensus;

        let tx_bytes = consensus::serialize(transaction);

        let request = BroadcastTransactionRequest {
            transaction: tx_bytes,
            allow_high_fees: false,
            bypass_limits: false,
        };

        let _response = self
            .sdk
            .execute(request, RequestSettings::default())
            .await
            .into_inner()
            .map_err(|e| {
                PlatformWalletError::TransactionBroadcast(format!("DAPI broadcast failed: {}", e))
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
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, PlatformWalletError> {
        self.spv.broadcast_transaction(transaction).await?;
        Ok(transaction.txid())
    }
}
