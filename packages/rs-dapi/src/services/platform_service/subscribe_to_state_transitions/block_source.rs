//! Committed blocks, read from the Tenderdash block store.
//!
//! The block store is the single source for both history and live blocks, so a subscription
//! that reads every height in order cannot miss a transition: a dropped websocket event only
//! delays when a height is read. Two Tenderdash behaviours shape the reads:
//!
//! - A block is stored before its execution results are saved, so the latest height reported
//!   by `status` can briefly have no results; such a height is retried, never skipped.
//! - `blockchain` returns at most the 20 *highest* heights of the range asked for, so pages are
//!   requested 20 heights at a time and checked to be complete.

use crate::DapiError;
use crate::clients::tenderdash_client::{
    BlockMeta, ResultBlock, ResultBlockResults, ResultBlockchainInfo, TenderdashClient,
};
use crate::error::DAPIResult;
use async_trait::async_trait;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use quick_cache::Weighter;
use quick_cache::sync::Cache;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;

/// Heights per `blockchain` page; Tenderdash's own cap.
pub const META_PAGE: u64 = 20;
/// Bytes of decoded blocks kept for subscriptions reading the same heights.
const BLOCK_CACHE_BYTES: u64 = 64 * 1024 * 1024;
/// Block metas pages kept.
const META_PAGE_CACHE_PAGES: usize = 256;

/// The Tenderdash RPCs the block source reads.
#[async_trait]
pub trait TenderdashBlocks: Send + Sync + 'static {
    async fn latest_height(&self) -> DAPIResult<u64>;
    async fn block(&self, height: u64) -> DAPIResult<ResultBlock>;
    async fn block_results(&self, height: u64) -> DAPIResult<ResultBlockResults>;
    async fn blockchain(
        &self,
        min_height: u64,
        max_height: u64,
    ) -> DAPIResult<ResultBlockchainInfo>;
}

#[async_trait]
impl TenderdashBlocks for TenderdashClient {
    async fn latest_height(&self) -> DAPIResult<u64> {
        Ok(self.status().await?.sync_info.latest_block_height.max(0) as u64)
    }

    async fn block(&self, height: u64) -> DAPIResult<ResultBlock> {
        TenderdashClient::block(self, height).await
    }

    async fn block_results(&self, height: u64) -> DAPIResult<ResultBlockResults> {
        TenderdashClient::block_results(self, height).await
    }

    async fn blockchain(
        &self,
        min_height: u64,
        max_height: u64,
    ) -> DAPIResult<ResultBlockchainInfo> {
        TenderdashClient::blockchain(self, min_height, max_height).await
    }
}

/// One successfully executed transaction of a committed block.
#[derive(Debug, Clone)]
pub struct CommittedTx {
    /// Position among the block's transactions.
    pub index: u32,
    /// SHA-256 of `bytes`, the Tenderdash transaction hash.
    pub hash: [u8; 32],
    /// The serialized state transition.
    pub bytes: Arc<Vec<u8>>,
}

/// A committed block's header facts and its successfully executed transactions.
#[derive(Debug, Clone)]
pub struct CommittedBlock {
    pub height: u64,
    pub time_ms: u64,
    pub protocol_version: u32,
    pub txs: Vec<CommittedTx>,
}

/// What reading one height produced.
#[derive(Debug, Clone)]
pub enum BlockRead {
    /// The block and its results are stored.
    Block(Arc<CommittedBlock>),
    /// Not readable yet: the block or its execution results have not been stored. Retry.
    NotYet,
    /// Never readable here: the store keeps only the block's header (state sync) or nothing.
    Unavailable,
}

/// One height's block meta, as far as the subscription needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeightMeta {
    /// The block has no transactions.
    Empty,
    /// The block has transactions.
    HasTxs,
    /// The store keeps no transactions for this height.
    Unavailable,
}

#[derive(Clone)]
struct BlockWeighter;

impl Weighter<u64, Arc<CommittedBlock>> for BlockWeighter {
    fn weight(&self, _height: &u64, block: &Arc<CommittedBlock>) -> u64 {
        let tx_bytes: usize = block.txs.iter().map(|tx| tx.bytes.len() + 64).sum();
        (tx_bytes + 64) as u64
    }
}

/// Reads committed blocks for every subscription, sharing what was read.
pub struct BlockSource {
    tenderdash: Arc<dyn TenderdashBlocks>,
    blocks: Cache<u64, Arc<CommittedBlock>, BlockWeighter>,
    meta_pages: Cache<u64, Arc<Vec<HeightMeta>>>,
}

impl BlockSource {
    pub fn new(tenderdash: Arc<dyn TenderdashBlocks>) -> Self {
        Self {
            tenderdash,
            blocks: Cache::with_weighter(4096, BLOCK_CACHE_BYTES, BlockWeighter),
            meta_pages: Cache::new(META_PAGE_CACHE_PAGES),
        }
    }

    /// The highest height Tenderdash reports stored. Its results may not be saved yet.
    pub async fn latest_height(&self) -> DAPIResult<u64> {
        self.tenderdash.latest_height().await
    }

    /// The metas of the aligned page holding `height` (heights `page_start..page_start + 20`),
    /// up to `tip`. Only complete pages are cached.
    pub async fn meta_page(
        &self,
        height: u64,
        tip: u64,
    ) -> DAPIResult<(u64, Arc<Vec<HeightMeta>>)> {
        let page_start = (height.saturating_sub(1) / META_PAGE) * META_PAGE + 1;
        let page_end = (page_start + META_PAGE - 1).min(tip);
        if let Some(page) = self.meta_pages.get(&page_start)
            && page.len() as u64 > page_end - page_start
        {
            return Ok((page_start, page));
        }
        let info = self.tenderdash.blockchain(page_start, page_end).await?;
        let page = Arc::new(page_from_metas(page_start, page_end, &info.block_metas)?);
        if page_end == page_start + META_PAGE - 1 {
            self.meta_pages.insert(page_start, page.clone());
        }
        Ok((page_start, page))
    }

    /// Read the committed block at `height`.
    pub async fn read(&self, height: u64) -> DAPIResult<BlockRead> {
        if let Some(block) = self.blocks.get(&height) {
            return Ok(BlockRead::Block(block));
        }
        let (block, results) = tokio::join!(
            self.tenderdash.block(height),
            self.tenderdash.block_results(height)
        );
        let Some(block) = block?.block else {
            return Ok(BlockRead::NotYet);
        };
        let results = match results {
            Ok(results) => results,
            // Results are saved after the block is stored: not available yet.
            Err(_) => return Ok(BlockRead::NotYet),
        };
        if block.data.txs.len() != results.txs_results.len() {
            if results.txs_results.is_empty() {
                return Ok(BlockRead::NotYet);
            }
            return Err(DapiError::Internal(format!(
                "block {height} has {} transactions but {} execution results",
                block.data.txs.len(),
                results.txs_results.len()
            )));
        }

        let mut txs = Vec::new();
        for (index, (tx, result)) in block.data.txs.iter().zip(&results.txs_results).enumerate() {
            if result.code != 0 {
                continue;
            }
            let bytes = BASE64_STANDARD.decode(tx).map_err(|e| {
                DapiError::Internal(format!(
                    "block {height} transaction {index} is not valid base64: {e}"
                ))
            })?;
            txs.push(CommittedTx {
                index: index as u32,
                hash: Sha256::digest(&bytes).into(),
                bytes: Arc::new(bytes),
            });
        }
        let protocol_version = u32::try_from(block.header.version.app).map_err(|_| {
            DapiError::Internal(format!(
                "block {height} has protocol version {} out of range",
                block.header.version.app
            ))
        })?;
        let committed = Arc::new(CommittedBlock {
            height,
            time_ms: parse_block_time_ms(&block.header.time).ok_or_else(|| {
                DapiError::Internal(format!(
                    "block {height} has an unreadable time '{}'",
                    block.header.time
                ))
            })?,
            protocol_version,
            txs,
        });
        self.blocks.insert(height, committed.clone());
        Ok(BlockRead::Block(committed))
    }
}

/// The page `[page_start, page_end]` from the metas Tenderdash returned, which must cover
/// exactly those heights. Heights below the store's base come back missing: they are
/// unavailable, as are header-only metas (`num_txs` < 0, from state sync).
fn page_from_metas(
    page_start: u64,
    page_end: u64,
    metas: &[BlockMeta],
) -> DAPIResult<Vec<HeightMeta>> {
    let mut page = vec![HeightMeta::Unavailable; (page_end - page_start + 1) as usize];
    let mut lowest_returned = None;
    for meta in metas {
        let height = meta.header.height;
        if !(page_start..=page_end).contains(&height) {
            return Err(DapiError::Internal(format!(
                "Tenderdash returned block meta {height} outside of {page_start}..={page_end}"
            )));
        }
        lowest_returned = Some(lowest_returned.map_or(height, |lowest: u64| lowest.min(height)));
        page[(height - page_start) as usize] = match meta.num_txs {
            n if n < 0 => HeightMeta::Unavailable,
            0 => HeightMeta::Empty,
            _ => HeightMeta::HasTxs,
        };
    }
    // Every height from the lowest returned up to the page end must be there: Tenderdash only
    // omits heights below its base.
    if let Some(lowest) = lowest_returned {
        let returned = metas.len() as u64;
        if returned != page_end - lowest + 1 {
            return Err(DapiError::Internal(format!(
                "Tenderdash returned {returned} block metas for heights {lowest}..={page_end}"
            )));
        }
    }
    Ok(page)
}

/// A Tenderdash RFC 3339 block time in milliseconds since the Unix epoch.
fn parse_block_time_ms(time: &str) -> Option<u64> {
    let time = chrono::DateTime::parse_from_rfc3339(time).ok()?;
    u64::try_from(time.timestamp_millis()).ok()
}

/// How long to wait before re-reading a height that is not readable yet.
pub const NOT_YET_RETRY: Duration = Duration::from_millis(200);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::tenderdash_client::{
        Block, BlockData, BlockHeader, ConsensusVersion, ExecTxResult,
    };

    fn meta(height: u64, num_txs: i64) -> BlockMeta {
        BlockMeta {
            header: BlockHeader {
                height,
                ..Default::default()
            },
            num_txs,
        }
    }

    #[test]
    fn should_accept_a_complete_page_and_mark_empty_heights() {
        let metas: Vec<_> = (1..=20).rev().map(|h| meta(h, (h % 2) as i64)).collect();
        let page = page_from_metas(1, 20, &metas).unwrap();
        assert_eq!(page[0], HeightMeta::HasTxs);
        assert_eq!(page[1], HeightMeta::Empty);
    }

    #[test]
    fn should_treat_heights_below_the_store_base_and_header_only_metas_as_unavailable() {
        let metas = vec![meta(40, 1), meta(39, -1), meta(38, 0)];
        let page = page_from_metas(21, 40, &metas).unwrap();
        // Heights 21..=37 are below the store's base.
        assert_eq!(page[0], HeightMeta::Unavailable);
        assert_eq!(page[16], HeightMeta::Unavailable);
        assert_eq!(page[17], HeightMeta::Empty);
        // Height 39 keeps only its header.
        assert_eq!(page[18], HeightMeta::Unavailable);
        assert_eq!(page[19], HeightMeta::HasTxs);
    }

    #[test]
    fn should_reject_a_page_with_a_gap_above_the_base() {
        let metas = vec![meta(40, 1), meta(38, 0)];
        assert!(page_from_metas(21, 40, &metas).is_err());
    }

    #[test]
    fn should_reject_metas_outside_the_page() {
        assert!(page_from_metas(1, 20, &[meta(21, 0)]).is_err());
    }

    #[test]
    fn should_parse_tenderdash_block_times() {
        assert_eq!(
            parse_block_time_ms("2026-10-05T12:00:00.123456789Z"),
            Some(1_791_201_600_123)
        );
        assert_eq!(parse_block_time_ms("not a time"), None);
    }

    struct OneBlock {
        txs: Vec<String>,
        results: Option<Vec<ExecTxResult>>,
    }

    #[async_trait]
    impl TenderdashBlocks for OneBlock {
        async fn latest_height(&self) -> DAPIResult<u64> {
            Ok(1)
        }
        async fn block(&self, height: u64) -> DAPIResult<ResultBlock> {
            Ok(ResultBlock {
                block: Some(Block {
                    header: BlockHeader {
                        version: ConsensusVersion { block: 14, app: 12 },
                        height,
                        time: "2026-10-05T12:00:00Z".to_string(),
                    },
                    data: BlockData {
                        txs: self.txs.clone(),
                    },
                }),
            })
        }
        async fn block_results(&self, height: u64) -> DAPIResult<ResultBlockResults> {
            match &self.results {
                Some(results) => Ok(ResultBlockResults {
                    height,
                    txs_results: results.clone(),
                }),
                None => Err(DapiError::Client("no results for height".to_string())),
            }
        }
        async fn blockchain(&self, _: u64, _: u64) -> DAPIResult<ResultBlockchainInfo> {
            Ok(ResultBlockchainInfo::default())
        }
    }

    fn result(code: u32) -> ExecTxResult {
        ExecTxResult {
            code,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn should_keep_only_successful_transactions_with_their_block_positions() {
        let txs = vec![
            BASE64_STANDARD.encode(b"first"),
            BASE64_STANDARD.encode(b"second"),
        ];
        let source = BlockSource::new(Arc::new(OneBlock {
            txs,
            results: Some(vec![result(1), result(0)]),
        }));
        let BlockRead::Block(block) = source.read(1).await.unwrap() else {
            panic!("expected a block");
        };
        assert_eq!(block.protocol_version, 12);
        assert_eq!(block.txs.len(), 1);
        assert_eq!(block.txs[0].index, 1);
        assert_eq!(block.txs[0].bytes.as_slice(), b"second");
        assert_eq!(
            block.txs[0].hash,
            <[u8; 32]>::from(Sha256::digest(b"second"))
        );
    }

    #[tokio::test]
    async fn should_report_a_block_without_saved_results_as_not_yet_readable() {
        let source = BlockSource::new(Arc::new(OneBlock {
            txs: vec![BASE64_STANDARD.encode(b"tx")],
            results: None,
        }));
        assert!(matches!(source.read(1).await.unwrap(), BlockRead::NotYet));
    }
}
