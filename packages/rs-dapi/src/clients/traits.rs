use async_trait::async_trait;
use std::fmt::Debug;
use tokio::sync::broadcast;

use super::tenderdash_client::{
    BroadcastTxResponse, CheckTxResponse, NetInfoResponse, TenderdashStatusResponse, TxResponse,
    UnconfirmedTxsResponse,
};
use super::tenderdash_websocket::TransactionEvent;
use crate::clients::tenderdash_websocket::BlockEvent;
use crate::error::DAPIResult;

#[async_trait]
pub trait TenderdashClientTrait: Send + Sync + Debug {
    async fn status(&self) -> DAPIResult<TenderdashStatusResponse>;
    async fn net_info(&self) -> DAPIResult<NetInfoResponse>;

    // State transition broadcasting methods
    async fn broadcast_tx(&self, tx: String) -> DAPIResult<BroadcastTxResponse>;
    async fn check_tx(&self, tx: String) -> DAPIResult<CheckTxResponse>;
    async fn unconfirmed_txs(&self, limit: Option<u32>) -> DAPIResult<UnconfirmedTxsResponse>;
    async fn tx(&self, hash: String) -> DAPIResult<TxResponse>;

    // WebSocket functionality for waitForStateTransitionResult
    fn subscribe_to_transactions(&self) -> broadcast::Receiver<TransactionEvent>;
    fn subscribe_to_blocks(&self) -> broadcast::Receiver<BlockEvent>;
    fn is_websocket_connected(&self) -> bool;
}
