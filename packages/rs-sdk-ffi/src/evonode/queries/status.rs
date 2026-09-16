//! Status of one evonode, asked of that node directly (DAPI `getStatus`).

use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dapi_client::{Address, RequestSettings};
use dash_sdk::platform::types::evonode::EvoNode;
use dash_sdk::platform::FetchUnproved;
use dash_sdk::query_types::evonode_status::EvoNodeStatus;
use serde_json::{json, Value};
use std::ffi::{c_char, c_void, CStr, CString};
use std::time::Duration;

/// Ask a single evonode for its DAPI `getStatus` self-report.
///
/// Unlike the other queries this does NOT go through the SDK's address
/// list: the request is sent to `address` only, over a one-connection pool
/// that is dropped after the call, with no failover to another node. The
/// response is unproved by nature — it is the node describing itself.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `address` - DAPI URI of the node, e.g. `https://203.0.113.7:443`
///
/// # Returns
/// A JSON object mirroring `EvoNodeStatus`, every field the node returned:
///
/// ```json
/// {"version":{"software":{"dapi":"…","drive":"…","tenderdash":"…"},
///             "protocol":{"tenderdash":{"p2p":9,"block":14},
///                         "drive":{"latest":9,"current":9,"nextEpoch":9}}},
///  "node":{"id":"<hex>","proTxHash":"<hex>"},
///  "chain":{"catchingUp":false,"latestBlockHash":"<hex>","latestAppHash":"<hex>",
///           "earliestBlockHash":"<hex>","earliestAppHash":"<hex>",
///           "latestBlockHeight":…,"earliestBlockHeight":…,"maxPeerBlockHeight":…,
///           "coreChainLockedHeight":…},
///  "network":{"chainId":"…","peersCount":…,"listening":true},
///  "stateSync":{"totalSyncedTime":…,"remainingTime":…,"totalSnapshots":…,
///               "chunkProcessAvgTime":…,"snapshotHeight":…,"snapshotChunksCount":…,
///               "backfilledBlocks":…,"backfillBlocksTotal":…},
///  "time":{"local":…,"block":…,"genesis":…,"epoch":…}}
/// ```
///
/// Hashes and ids are hex. Optional protobuf fields the node omitted are
/// `null`. Timestamps are passed through exactly as the node sent them:
/// `time.block` / `time.genesis` are Unix milliseconds (Drive's `time_ms`;
/// Drive sends `0` when it has no genesis info), while `time.local` is Unix
/// seconds from rs-dapi and milliseconds from the legacy JS DAPI.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
/// - `address` must be a valid NUL-terminated C string for the duration of the call.
/// - On success the returned C string pointer must be freed by the caller with `dash_sdk_string_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_evonode_get_status(
    sdk_handle: *const SDKHandle,
    address: *const c_char,
) -> DashSDKResult {
    match get_evonode_status(sdk_handle, address) {
        Ok(json) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult {
                        data_type: DashSDKResultDataType::NoData,
                        data: std::ptr::null_mut(),
                        error: Box::into_raw(Box::new(DashSDKError::new(
                            DashSDKErrorCode::InternalError,
                            format!("Failed to create CString: {}", e),
                        ))),
                    }
                }
            };
            DashSDKResult {
                data_type: DashSDKResultDataType::String,
                data: c_str.into_raw() as *mut c_void,
                error: std::ptr::null_mut(),
            }
        }
        Err((code, message)) => DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(DashSDKError::new(code, message))),
        },
    }
}

fn get_evonode_status(
    sdk_handle: *const SDKHandle,
    address: *const c_char,
) -> Result<String, (DashSDKErrorCode, String)> {
    if sdk_handle.is_null() {
        return Err((
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }
    if address.is_null() {
        return Err((
            DashSDKErrorCode::InvalidParameter,
            "Address is null".to_string(),
        ));
    }

    let address_str = unsafe {
        CStr::from_ptr(address).to_str().map_err(|e| {
            (
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid UTF-8 in address: {}", e),
            )
        })?
    };
    let address: Address = address_str.parse().map_err(|e| {
        (
            DashSDKErrorCode::InvalidParameter,
            format!("Invalid evonode address '{}': {}", address_str, e),
        )
    })?;

    let rt = crate::runtime::BigStackRuntime::new_isolated().map_err(|e| {
        (
            DashSDKErrorCode::InternalError,
            format!("Failed to create Tokio runtime: {}", e),
        )
    })?;

    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    // One node, asked once (plus a single retry): the SDK's default retry
    // budget is sized for rotating through an address list, which this
    // request never does — every retry would hit the same unreachable node.
    // Bound the TCP connect too: the SDK default leaves it to the OS
    // (~75 s on Apple platforms), which would make an offline node look like
    // a hang. Don't ban the address either; it is not in the SDK's pool.
    let settings = RequestSettings {
        connect_timeout: Some(Duration::from_secs(10)),
        timeout: Some(Duration::from_secs(15)),
        retries: Some(1),
        ban_failed_address: Some(false),
        ..RequestSettings::default()
    };

    rt.block_on(async move {
        match EvoNodeStatus::fetch_unproved_with_settings(&sdk, EvoNode::new(address), settings)
            .await
        {
            Ok((Some(status), _metadata)) => Ok(evonode_status_json(&status).to_string()),
            Ok((None, _metadata)) => Err((
                DashSDKErrorCode::NotFound,
                "The evonode returned no status".to_string(),
            )),
            Err(e) => Err((
                DashSDKErrorCode::NetworkError,
                format!("Failed to fetch evonode status: {}", e),
            )),
        }
    })
}

/// Serialize every `EvoNodeStatus` field. Kept separate from the FFI entry
/// point so the wire shape is unit-testable without a network.
fn evonode_status_json(status: &EvoNodeStatus) -> Value {
    let version = &status.version;
    let node = &status.node;
    let chain = &status.chain;
    let network = &status.network;
    let state_sync = &status.state_sync;
    let time = &status.time;

    json!({
        "version": {
            "software": version.software.as_ref().map(|s| json!({
                "dapi": s.dapi,
                "drive": s.drive,
                "tenderdash": s.tenderdash,
            })),
            "protocol": version.protocol.as_ref().map(|p| json!({
                "tenderdash": p.tenderdash.as_ref().map(|t| json!({
                    "p2p": t.p2p,
                    "block": t.block,
                })),
                "drive": p.drive.as_ref().map(|d| json!({
                    "latest": d.latest,
                    "current": d.current,
                    "nextEpoch": d.next_epoch,
                })),
            })),
        },
        "node": {
            "id": hex::encode(&node.id),
            "proTxHash": node.pro_tx_hash.as_ref().map(hex::encode),
        },
        "chain": {
            "catchingUp": chain.catching_up,
            "latestBlockHash": hex::encode(&chain.latest_block_hash),
            "latestAppHash": hex::encode(&chain.latest_app_hash),
            "earliestBlockHash": hex::encode(&chain.earliest_block_hash),
            "earliestAppHash": hex::encode(&chain.earliest_app_hash),
            "latestBlockHeight": chain.latest_block_height,
            "earliestBlockHeight": chain.earliest_block_height,
            "maxPeerBlockHeight": chain.max_peer_block_height,
            "coreChainLockedHeight": chain.core_chain_locked_height,
        },
        "network": {
            "chainId": network.chain_id,
            "peersCount": network.peers_count,
            "listening": network.listening,
        },
        "stateSync": {
            "totalSyncedTime": state_sync.total_synced_time,
            "remainingTime": state_sync.remaining_time,
            "totalSnapshots": state_sync.total_snapshots,
            "chunkProcessAvgTime": state_sync.chunk_process_avg_time,
            "snapshotHeight": state_sync.snapshot_height,
            "snapshotChunksCount": state_sync.snapshot_chunks_count,
            "backfilledBlocks": state_sync.backfilled_blocks,
            "backfillBlocksTotal": state_sync.backfill_blocks_total,
        },
        "time": {
            "local": time.local,
            "block": time.block,
            "genesis": time.genesis,
            "epoch": time.epoch,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use dash_sdk::query_types::evonode_status::{
        Chain, DriveProtocol, Network, Node, Protocol, Software, StateSync, TenderdashProtocol,
        Time, Version,
    };

    #[test]
    fn test_get_evonode_status_null_handle() {
        unsafe {
            let address = CString::new("https://127.0.0.1:1").unwrap();
            let result = dash_sdk_evonode_get_status(std::ptr::null(), address.as_ptr());
            assert!(!result.error.is_null());
            assert_eq!(
                (*result.error).code,
                DashSDKErrorCode::InvalidParameter,
                "a null handle is a caller error, not a network failure"
            );
            crate::dash_sdk_error_free(result.error);
        }
    }

    #[test]
    fn test_get_evonode_status_null_address() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_evonode_get_status(handle, std::ptr::null());
            assert!(!result.error.is_null());
            assert_eq!((*result.error).code, DashSDKErrorCode::InvalidParameter);
            crate::dash_sdk_error_free(result.error);
            destroy_mock_sdk_handle(handle);
        }
    }

    /// An address without a host can never be contacted — reject it before
    /// touching the network instead of reporting a misleading transport error.
    #[test]
    fn test_get_evonode_status_invalid_address() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let address = CString::new("not a uri").unwrap();
            let result = dash_sdk_evonode_get_status(handle, address.as_ptr());
            assert!(!result.error.is_null());
            assert_eq!((*result.error).code, DashSDKErrorCode::InvalidParameter);
            let message = CStr::from_ptr((*result.error).message)
                .to_string_lossy()
                .into_owned();
            assert!(
                message.contains("Invalid evonode address"),
                "unexpected message: {message}"
            );
            crate::dash_sdk_error_free(result.error);
            destroy_mock_sdk_handle(handle);
        }
    }

    /// Every `EvoNodeStatus` field must reach the JSON — the wallet shows
    /// the whole self-report, so a dropped field is a silently missing row.
    #[test]
    fn test_evonode_status_json_carries_every_field() {
        let status = EvoNodeStatus {
            version: Version {
                software: Some(Software {
                    dapi: "1.2.3".to_string(),
                    drive: Some("4.5.6".to_string()),
                    tenderdash: Some("0.14.0-dev.1".to_string()),
                }),
                protocol: Some(Protocol {
                    tenderdash: Some(TenderdashProtocol { p2p: 9, block: 12 }),
                    drive: Some(DriveProtocol {
                        latest: 7,
                        current: 6,
                        next_epoch: 7,
                    }),
                }),
            },
            node: Node {
                id: vec![0xAA; 20],
                pro_tx_hash: Some(vec![0xBB; 32]),
            },
            chain: Chain {
                catching_up: true,
                latest_block_hash: vec![0x11; 32],
                latest_app_hash: vec![0x22; 32],
                earliest_block_hash: vec![0x33; 32],
                earliest_app_hash: vec![0x44; 32],
                latest_block_height: 5000,
                earliest_block_height: 10,
                max_peer_block_height: 5001,
                core_chain_locked_height: Some(750),
            },
            network: Network {
                chain_id: "dash-mainnet".to_string(),
                peers_count: 50,
                listening: true,
            },
            state_sync: StateSync {
                total_synced_time: 7200,
                remaining_time: 60,
                total_snapshots: 3,
                chunk_process_avg_time: 25,
                snapshot_height: 4500,
                snapshot_chunks_count: 200,
                backfilled_blocks: 1000,
                backfill_blocks_total: 2000,
            },
            time: Time {
                local: 1_700_000_000_000,
                block: Some(1_699_999_900_000),
                genesis: Some(1_690_000_000_000),
                epoch: Some(42),
            },
        };

        let json = evonode_status_json(&status);

        assert_eq!(json["version"]["software"]["dapi"], "1.2.3");
        assert_eq!(json["version"]["software"]["drive"], "4.5.6");
        assert_eq!(json["version"]["software"]["tenderdash"], "0.14.0-dev.1");
        assert_eq!(json["version"]["protocol"]["tenderdash"]["p2p"], 9);
        assert_eq!(json["version"]["protocol"]["tenderdash"]["block"], 12);
        assert_eq!(json["version"]["protocol"]["drive"]["latest"], 7);
        assert_eq!(json["version"]["protocol"]["drive"]["current"], 6);
        assert_eq!(json["version"]["protocol"]["drive"]["nextEpoch"], 7);

        assert_eq!(json["node"]["id"], "aa".repeat(20));
        assert_eq!(json["node"]["proTxHash"], "bb".repeat(32));

        assert_eq!(json["chain"]["catchingUp"], true);
        assert_eq!(json["chain"]["latestBlockHash"], "11".repeat(32));
        assert_eq!(json["chain"]["latestAppHash"], "22".repeat(32));
        assert_eq!(json["chain"]["earliestBlockHash"], "33".repeat(32));
        assert_eq!(json["chain"]["earliestAppHash"], "44".repeat(32));
        assert_eq!(json["chain"]["latestBlockHeight"], 5000);
        assert_eq!(json["chain"]["earliestBlockHeight"], 10);
        assert_eq!(json["chain"]["maxPeerBlockHeight"], 5001);
        assert_eq!(json["chain"]["coreChainLockedHeight"], 750);

        assert_eq!(json["network"]["chainId"], "dash-mainnet");
        assert_eq!(json["network"]["peersCount"], 50);
        assert_eq!(json["network"]["listening"], true);

        assert_eq!(json["stateSync"]["totalSyncedTime"], 7200);
        assert_eq!(json["stateSync"]["remainingTime"], 60);
        assert_eq!(json["stateSync"]["totalSnapshots"], 3);
        assert_eq!(json["stateSync"]["chunkProcessAvgTime"], 25);
        assert_eq!(json["stateSync"]["snapshotHeight"], 4500);
        assert_eq!(json["stateSync"]["snapshotChunksCount"], 200);
        assert_eq!(json["stateSync"]["backfilledBlocks"], 1000);
        assert_eq!(json["stateSync"]["backfillBlocksTotal"], 2000);

        assert_eq!(json["time"]["local"], 1_700_000_000_000u64);
        assert_eq!(json["time"]["block"], 1_699_999_900_000u64);
        assert_eq!(json["time"]["genesis"], 1_690_000_000_000u64);
        assert_eq!(json["time"]["epoch"], 42);
    }

    /// Live: ask a real mainnet evonode through the FFI entry point. Needs
    /// network access, so it is ignored by default:
    /// `cargo test -p rs-sdk-ffi --lib evonode::queries::status -- --ignored --nocapture`
    #[test]
    #[ignore = "needs network access to a mainnet evonode"]
    fn live_mainnet_evonode_status() {
        use std::sync::Arc;

        // Same wiring as `dash_sdk_create_trusted` for mainnet: the builder
        // needs a context provider even though getStatus never uses one.
        let provider = Arc::new(
            rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new(
                dash_sdk::dpp::dashcore::Network::Mainnet,
                None,
                std::num::NonZeroUsize::new(100).unwrap(),
            )
            .expect("trusted context provider"),
        );
        let sdk = dash_sdk::SdkBuilder::new_mainnet()
            .with_context_provider(provider)
            .build()
            .expect("mainnet sdk");
        // The SDK's own bootstrap list is built from the evo seeds, so its
        // first entry is a reachable mainnet evonode DAPI address.
        let address = sdk
            .address_list()
            .get_live_address()
            .expect("a mainnet evonode address")
            .uri()
            .to_string();
        let wrapper = Box::new(crate::sdk::SDKWrapper {
            sdk,
            runtime: Arc::new(crate::runtime::BigStackRuntime::build_shared().expect("runtime")),
            trusted_provider: None,
        });
        let handle = Box::into_raw(wrapper) as *mut SDKHandle;

        unsafe {
            let c_address = CString::new(address.clone()).unwrap();
            let result = dash_sdk_evonode_get_status(handle, c_address.as_ptr());
            if !result.error.is_null() {
                let message = CStr::from_ptr((*result.error).message)
                    .to_string_lossy()
                    .into_owned();
                crate::dash_sdk_error_free(result.error);
                panic!("getStatus from {address} failed: {message}");
            }
            let json = CStr::from_ptr(result.data as *const c_char)
                .to_str()
                .expect("utf-8 json")
                .to_string();
            println!("{address} -> {json}");
            let value: Value = serde_json::from_str(&json).expect("json object");
            assert!(
                value["chain"]["latestBlockHeight"].as_u64().unwrap_or(0) > 0,
                "a live node reports a positive height: {json}"
            );
            assert!(
                value["network"]["chainId"]
                    .as_str()
                    .is_some_and(|c| !c.is_empty()),
                "a live node reports its chain id: {json}"
            );
            crate::dash_sdk_string_free(result.data as *mut c_char);
            crate::sdk::dash_sdk_destroy(handle);
        }
    }

    /// Optional fields the node omitted are `null`, never a fabricated zero
    /// or empty string — the wallet renders them as "not reported".
    #[test]
    fn test_evonode_status_json_omitted_fields_are_null() {
        let status = EvoNodeStatus::default();
        let json = evonode_status_json(&status);

        assert!(json["version"]["software"].is_null());
        assert!(json["version"]["protocol"].is_null());
        assert!(json["node"]["proTxHash"].is_null());
        assert!(json["chain"]["coreChainLockedHeight"].is_null());
        assert!(json["time"]["block"].is_null());
        assert!(json["time"]["genesis"].is_null());
        assert!(json["time"]["epoch"].is_null());
        // Required fields are still present.
        assert_eq!(json["node"]["id"], "");
        assert_eq!(json["chain"]["latestBlockHeight"], 0);
        assert_eq!(json["network"]["chainId"], "");
        assert_eq!(json["time"]["local"], 0);
    }
}
