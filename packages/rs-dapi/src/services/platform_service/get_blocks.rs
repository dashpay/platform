//! `getBlocks`: up to ten consecutive blocks of the Platform chain.
//!
//! The blocks come from Tenderdash and are not proved: the client trusts the node it asks.
//! This is for looking blocks up, not for syncing a chain from DAPI, so the cap here works
//! together with a much lower per-IP request limit in the gateway's rate limiter.

use std::future::Future;
use std::ops::RangeInclusive;

use chrono::DateTime;
use dapi_grpc::platform::v0::get_blocks_response::get_blocks_response_v0::{Block, Transaction};
use dapi_grpc::platform::v0::get_blocks_response::{self, GetBlocksResponseV0};
use dapi_grpc::platform::v0::{GetBlocksRequest, GetBlocksResponse, get_blocks_request};
use dapi_grpc::tonic::{Request, Response};
use sha2::{Digest, Sha256};

use super::error_mapping::base64_decode;
use crate::cache::make_cache_key;
use crate::clients::tenderdash_client::{
    Block as TenderdashBlock, BlockTxResult, MAX_TENDERDASH_HEIGHT, ResultBlock, ResultBlockResults,
};
use crate::error::{DAPIResult, DapiError};
use crate::services::platform_service::PlatformServiceImpl;

const MAX_BLOCKS_PER_REQUEST: u32 = 10;

/// Keeps a response within the 4 MiB a gRPC client decodes by default. A block can hold
/// 2 MiB, so ten full blocks would not fit, while any single block does.
const MAX_RESPONSE_BYTES: usize = 4_000_000;

impl PlatformServiceImpl {
    /// Read the requested blocks from Tenderdash, each transaction with its execution result.
    pub(super) async fn get_blocks_impl(
        &self,
        request: Request<GetBlocksRequest>,
    ) -> DAPIResult<Response<GetBlocksResponse>> {
        let request = request.into_inner();
        let heights = parse_request(&request)?;

        let key = make_cache_key("get_blocks", &request);
        if let Some(cached) = self.platform_cache.get(&key) {
            return Ok(Response::new(cached));
        }
        let generation = self.platform_cache.generation();

        let blocks = collect_blocks(heights, |height| self.fetch_block(height)).await?;

        let response = GetBlocksResponse {
            version: Some(get_blocks_response::Version::V0(GetBlocksResponseV0 {
                blocks,
            })),
        };
        self.platform_cache.put_from_generation(
            key,
            &response,
            dapi_grpc::Message::encoded_len(&response),
            generation,
        );

        Ok(Response::new(response))
    }

    async fn fetch_block(&self, height: u64) -> DAPIResult<Option<Block>> {
        read_block(
            height,
            self.tenderdash_client.block(height),
            self.tenderdash_client.block_results(height),
        )
        .await
    }
}

/// Await the two Tenderdash answers for one height; `results` is only awaited, and so only
/// sent, when the block has transactions.
///
/// `None` means the node's chain does not reach this height. A block it holds without its
/// results is an error instead, because that block must not vanish from a range.
async fn read_block(
    height: u64,
    block: impl Future<Output = DAPIResult<ResultBlock>>,
    results: impl Future<Output = DAPIResult<ResultBlockResults>>,
) -> DAPIResult<Option<Block>> {
    let result = match block.await {
        Ok(result) => result,
        Err(error) if error.is_missing_tenderdash_height() => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(block) = result.block else {
        return Ok(None);
    };
    let results = if block.data.txs.is_empty() {
        Vec::new()
    } else {
        results.await?.txs_results
    };

    build_block(height, &result.block_id.hash, block, &results).map(Some)
}

/// Validate the request, returning the heights it asks for.
fn parse_request(request: &GetBlocksRequest) -> DAPIResult<RangeInclusive<u64>> {
    let Some(get_blocks_request::Version::V0(v0)) = &request.version else {
        return Err(DapiError::InvalidArgument(
            "missing request version".to_string(),
        ));
    };
    if v0.count == 0 || v0.count > MAX_BLOCKS_PER_REQUEST {
        return Err(DapiError::InvalidArgument(format!(
            "count must be between 1 and {MAX_BLOCKS_PER_REQUEST}"
        )));
    }
    if v0.start_height == 0 || v0.start_height > MAX_TENDERDASH_HEIGHT {
        return Err(DapiError::InvalidArgument(format!(
            "start_height must be between 1 and {MAX_TENDERDASH_HEIGHT}"
        )));
    }
    let end_height = v0.start_height + u64::from(v0.count) - 1;
    if end_height > MAX_TENDERDASH_HEIGHT {
        return Err(DapiError::InvalidArgument(format!(
            "the last height must not be above {MAX_TENDERDASH_HEIGHT}"
        )));
    }

    Ok(v0.start_height..=end_height)
}

/// Fetch the blocks one at a time, stopping early where the chain ends or the response
/// is full. The first block is always either returned or the error of the query.
async fn collect_blocks<F, Fut>(heights: RangeInclusive<u64>, fetch: F) -> DAPIResult<Vec<Block>>
where
    F: Fn(u64) -> Fut,
    Fut: Future<Output = DAPIResult<Option<Block>>>,
{
    let mut blocks = Vec::new();
    let mut response_bytes = 0;
    for height in heights {
        let Some(block) = fetch(height).await? else {
            if blocks.is_empty() {
                return Err(DapiError::NotFound(format!("no block at height {height}")));
            }
            break;
        };
        response_bytes += dapi_grpc::Message::encoded_len(&block);
        if response_bytes > MAX_RESPONSE_BYTES && !blocks.is_empty() {
            break;
        }
        blocks.push(block);
    }
    Ok(blocks)
}

/// Convert a Tenderdash block, pairing each transaction with its execution result.
fn build_block(
    height: u64,
    hash: &str,
    block: TenderdashBlock,
    results: &[BlockTxResult],
) -> DAPIResult<Block> {
    // Results line up with transactions by position, so a list of another length
    // cannot be trusted to give any transaction its own result.
    if results.len() != block.data.txs.len() {
        return Err(DapiError::Internal(format!(
            "block {height} has {} transactions but {} results",
            block.data.txs.len(),
            results.len()
        )));
    }
    let transactions = block
        .data
        .txs
        .iter()
        .zip(results)
        .enumerate()
        .map(|(index, (tx, result))| {
            let state_transition = base64_decode(tx).ok_or_else(|| {
                DapiError::Internal(format!(
                    "transaction {index} of block {height} is not base64"
                ))
            })?;
            Ok(Transaction {
                hash: Sha256::digest(&state_transition).to_vec(),
                state_transition,
                code: result.code,
            })
        })
        .collect::<DAPIResult<Vec<_>>>()?;

    Ok(Block {
        height,
        hash: decode_hex("hash", hash)?,
        time_ms: parse_time_ms(&block.header.time)?,
        core_chain_locked_height: block.header.core_chain_locked_height,
        app_hash: decode_hex("app_hash", &block.header.app_hash)?,
        proposer_pro_tx_hash: decode_hex(
            "proposer_pro_tx_hash",
            &block.header.proposer_pro_tx_hash,
        )?,
        transactions,
    })
}

fn decode_hex(field: &str, value: &str) -> DAPIResult<Vec<u8>> {
    hex::decode(value)
        .map_err(|error| DapiError::Internal(format!("block {field} is not hex: {error}")))
}

fn parse_time_ms(time: &str) -> DAPIResult<u64> {
    let timestamp = DateTime::parse_from_rfc3339(time)
        .map_err(|error| DapiError::Internal(format!("block time {time:?}: {error}")))?;
    u64::try_from(timestamp.timestamp_millis())
        .map_err(|_| DapiError::Internal(format!("block time {time:?} is before the epoch")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_blocks_request::GetBlocksRequestV0;
    use serde_json::json;

    fn request(start_height: u64, count: u32) -> GetBlocksRequest {
        GetBlocksRequest {
            version: Some(get_blocks_request::Version::V0(GetBlocksRequestV0 {
                start_height,
                count,
            })),
        }
    }

    fn block_of_size(height: u64, payload_bytes: usize) -> Block {
        Block {
            height,
            transactions: vec![Transaction {
                state_transition: vec![0; payload_bytes],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// An error in the shape Tenderdash's RPC answers with.
    fn tenderdash_error(data: &str) -> DapiError {
        DapiError::from_tenderdash_error(json!({
            "code": -32603,
            "message": "Internal error",
            "data": data,
        }))
    }

    fn heights(blocks: &[Block]) -> Vec<u64> {
        blocks.iter().map(|block| block.height).collect()
    }

    #[test]
    fn should_accept_up_to_ten_blocks() {
        assert_eq!(parse_request(&request(5, 1)).unwrap(), 5..=5);
        assert_eq!(parse_request(&request(5, 10)).unwrap(), 5..=14);
    }

    #[test]
    fn should_reject_a_count_outside_one_to_ten() {
        for count in [0, 11, u32::MAX] {
            assert!(matches!(
                parse_request(&request(5, count)),
                Err(DapiError::InvalidArgument(_))
            ));
        }
    }

    #[test]
    fn should_reject_heights_tenderdash_cannot_hold() {
        for (start_height, count) in [
            (0, 1),
            (MAX_TENDERDASH_HEIGHT + 1, 1),
            (MAX_TENDERDASH_HEIGHT, 2),
            (u64::MAX, 10),
        ] {
            assert!(matches!(
                parse_request(&request(start_height, count)),
                Err(DapiError::InvalidArgument(_))
            ));
        }
        assert_eq!(
            parse_request(&request(MAX_TENDERDASH_HEIGHT, 1)).unwrap(),
            MAX_TENDERDASH_HEIGHT..=MAX_TENDERDASH_HEIGHT
        );
    }

    #[test]
    fn should_reject_a_request_without_a_version() {
        assert!(matches!(
            parse_request(&GetBlocksRequest { version: None }),
            Err(DapiError::InvalidArgument(_))
        ));
    }

    #[tokio::test]
    async fn should_return_every_block_of_the_range_in_order() {
        let blocks = collect_blocks(
            3..=6,
            |height| async move { Ok(Some(block_of_size(height, 1))) },
        )
        .await
        .unwrap();

        assert_eq!(heights(&blocks), [3, 4, 5, 6]);
    }

    #[tokio::test]
    async fn should_return_the_blocks_that_exist_when_the_range_passes_the_tip() {
        let blocks = collect_blocks(3..=12, |height| async move {
            Ok((height <= 5).then(|| block_of_size(height, 1)))
        })
        .await
        .unwrap();

        assert_eq!(heights(&blocks), [3, 4, 5]);
    }

    #[tokio::test]
    async fn should_report_not_found_when_the_first_block_is_missing() {
        let result = collect_blocks(3..=12, |_| async { Ok(None) }).await;

        assert!(matches!(result, Err(DapiError::NotFound(_))));
    }

    #[tokio::test]
    async fn should_fail_rather_than_cut_the_range_short_on_another_error() {
        let result = collect_blocks(3..=12, |height| async move {
            if height > 5 {
                Err(DapiError::Internal("broken".to_string()))
            } else {
                Ok(Some(block_of_size(height, 1)))
            }
        })
        .await;

        assert!(matches!(result, Err(DapiError::Internal(_))));
    }

    #[tokio::test]
    async fn should_fail_rather_than_omit_a_block_whose_results_are_missing() {
        let result = collect_blocks(3..=12, |height| {
            let txs = if height > 5 {
                json!(["AQI="])
            } else {
                json!(null)
            };
            read_block(height, async move { Ok(tenderdash_block(txs)) }, async {
                Err(tenderdash_error("could not find results for height #6"))
            })
        })
        .await;

        let error = result.unwrap_err();
        assert!(error.is_missing_tenderdash_height());
    }

    #[tokio::test]
    async fn should_end_the_range_when_the_block_itself_is_missing() {
        let blocks = collect_blocks(3..=12, |height| {
            read_block(
                height,
                async move {
                    if height > 5 {
                        Err(tenderdash_error(
                            "height must be less than or equal to the head of the node's blockchain",
                        ))
                    } else {
                        Ok(tenderdash_block(json!(null)))
                    }
                },
                async { Err(DapiError::Internal("no results expected".to_string())) },
            )
        })
        .await
        .unwrap();

        assert_eq!(heights(&blocks), [3, 4, 5]);
    }

    #[tokio::test]
    async fn should_stop_before_the_response_passes_its_size_limit() {
        let blocks = collect_blocks(1..=10, |height| async move {
            Ok(Some(block_of_size(height, 1_500_000)))
        })
        .await
        .unwrap();

        assert_eq!(heights(&blocks), [1, 2]);
    }

    #[tokio::test]
    async fn should_return_a_first_block_larger_than_the_size_limit() {
        let blocks = collect_blocks(1..=10, |height| async move {
            Ok(Some(block_of_size(height, MAX_RESPONSE_BYTES + 1)))
        })
        .await
        .unwrap();

        assert_eq!(heights(&blocks), [1]);
    }

    /// A `block` result in the shape Tenderdash's RPC serves.
    fn tenderdash_block(txs: serde_json::Value) -> ResultBlock {
        serde_json::from_value(json!({
            "block_id": {
                "hash": "ABCD",
                "parts": { "total": 1, "hash": "00" },
                "state_id": "00",
            },
            "block": {
                "header": {
                    "version": { "block": "14", "app": "11" },
                    "chain_id": "dash-1",
                    "height": "7",
                    "core_chain_locked_height": 2300000,
                    "time": "2024-07-29T15:00:00.123456789Z",
                    "app_hash": "0102",
                    "proposer_pro_tx_hash": "0A0B",
                },
                "data": { "txs": txs },
                "core_chain_lock": null,
                "evidence": [],
                "last_commit": null,
            },
        }))
        .unwrap()
    }

    #[test]
    fn should_convert_a_tenderdash_block() {
        let result = tenderdash_block(json!(["AQI=", "AwQ="]));
        let results: ResultBlockResults = serde_json::from_value(json!({
            "height": "7",
            "txs_results": [{ "code": 0 }, { "code": 40100 }],
        }))
        .unwrap();

        let block = build_block(
            7,
            &result.block_id.hash,
            result.block.unwrap(),
            &results.txs_results,
        )
        .unwrap();

        assert_eq!(block.height, 7);
        assert_eq!(block.hash, [0xab, 0xcd]);
        assert_eq!(block.time_ms, 1_722_265_200_123);
        assert_eq!(block.core_chain_locked_height, 2_300_000);
        assert_eq!(block.app_hash, [1, 2]);
        assert_eq!(block.proposer_pro_tx_hash, [10, 11]);
        assert_eq!(block.transactions.len(), 2);
        assert_eq!(block.transactions[0].state_transition, [1, 2]);
        assert_eq!(block.transactions[0].hash, Sha256::digest([1, 2]).to_vec());
        assert_eq!(block.transactions[0].code, 0);
        assert_eq!(block.transactions[1].state_transition, [3, 4]);
        assert_eq!(block.transactions[1].code, 40100);
    }

    #[test]
    fn should_convert_a_block_without_transactions() {
        let result = tenderdash_block(json!(null));

        let block = build_block(7, &result.block_id.hash, result.block.unwrap(), &[]).unwrap();

        assert!(block.transactions.is_empty());
    }

    #[test]
    fn should_fail_when_results_do_not_line_up_with_the_block() {
        let result = tenderdash_block(json!(["AQI=", "AwQ="]));

        assert!(matches!(
            build_block(
                7,
                &result.block_id.hash,
                result.block.unwrap(),
                &[BlockTxResult { code: 0 }],
            ),
            Err(DapiError::Internal(_))
        ));
    }

    #[test]
    fn should_fail_on_a_transaction_that_is_not_base64() {
        let result = tenderdash_block(json!(["not base64!"]));

        assert!(matches!(
            build_block(
                7,
                &result.block_id.hash,
                result.block.unwrap(),
                &[BlockTxResult { code: 0 }],
            ),
            Err(DapiError::Internal(_))
        ));
    }

    #[test]
    fn should_fail_on_a_block_time_it_cannot_read() {
        assert!(matches!(
            parse_time_ms("yesterday"),
            Err(DapiError::Internal(_))
        ));
        assert!(matches!(
            parse_time_ms("1969-12-31T23:59:59Z"),
            Err(DapiError::Internal(_))
        ));
    }
}
