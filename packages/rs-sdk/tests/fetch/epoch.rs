use std::collections::BTreeMap;

use super::{common::setup_logs, config::Config};
use dapi_grpc::platform::{
    v0::{get_identity_request::GetIdentityRequestV0, GetIdentityRequest},
    VersionedGrpcResponse,
};
use dpp::block::epoch::EpochIndex;
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use rs_dapi_client::{DapiRequestExecutor, RequestSettings};
use rs_sdk::{
    platform::{
        types::epoch::ExtendedEpochInfoEx, Fetch, FetchMany, LimitQuery, DEFAULT_EPOCH_QUERY_LIMIT,
    },
    Sdk,
};

/// Get current epoch index from DAPI response metadata
async fn get_current_epoch(sdk: &Sdk, cfg: &Config) -> EpochIndex {
    //  We need existing epoch from metadata, so we'll use low-level API here to get it
    let identity_request: GetIdentityRequest = GetIdentityRequestV0 {
        id: cfg.existing_identity_id.to_vec(),
        prove: true,
    }
    .into();

    let response = sdk
        .execute(identity_request, RequestSettings::default())
        .await
        .expect("get identity");

    response.metadata().expect("metadata").epoch as EpochIndex
}
/// Check some assertions on returned epochs list
fn assert_epochs(
    epochs: BTreeMap<u16, Option<ExtendedEpochInfo>>,
    starting_epoch: EpochIndex,
    current_epoch: EpochIndex,
    limit: u16,
) {
    tracing::info!(
        starting_epoch,
        current_epoch,
        limit,
        len = epochs.len(),
        "checking epoch assertions"
    );
    // When starting_epoch in future, we should get nothing
    if starting_epoch > current_epoch {
        assert!(epochs.is_empty());
        return;
    }

    epochs.get(&starting_epoch).unwrap_or_else(|| {
        panic!(
            "starting epoch {} when current is {} should be available: {:?}",
            starting_epoch, current_epoch, epochs
        )
    });

    // last retrieved epoch
    let (last_epoch, expected_len) = if starting_epoch + limit <= current_epoch {
        // we have more epochs than we requested
        (starting_epoch + limit - 1, limit as usize)
    } else {
        // we have less epochs than we requested
        (current_epoch, (current_epoch - starting_epoch + 1) as usize)
    };
    assert_eq!(epochs.len(), expected_len);

    epochs
        .get(&last_epoch)
        .unwrap_or_else(|| panic!("last_epoch {} should be available", last_epoch));
}

/// When I fetch multiple epochs starting with genesis, I get all of them up to query limit
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_epoch_list() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_epoch_list").await;

    // Given some starting epoch and current epoch
    // Note the devnet does not necessarily start with epoch 0
    let starting_epoch: EpochIndex = 185;
    let current_epoch = get_current_epoch(&sdk, &cfg).await;

    // When we fetch epochs from the server, starting with `starting_epoch`
    let epochs: BTreeMap<u16, Option<ExtendedEpochInfo>> =
        ExtendedEpochInfo::fetch_many(&sdk, starting_epoch)
            .await
            .expect("list epochs");

    assert_epochs(
        epochs,
        starting_epoch,
        current_epoch,
        DEFAULT_EPOCH_QUERY_LIMIT as u16,
    )
}

/// Given that we have multiple epochs, I can fetch them starting from epoch 1
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_epoch_list_limit() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_epoch_list_limit").await;

    // Given some starting epoch and current epoch
    // Note the devnet does not necessarily start with epoch 0
    let starting_epoch: EpochIndex = 193;
    let current_epoch = get_current_epoch(&sdk, &cfg).await;
    let limit = 2;

    let query: LimitQuery<EpochIndex> = LimitQuery {
        query: starting_epoch,
        limit: Some(limit),
    };

    let epochs = ExtendedEpochInfo::fetch_many(&sdk, query)
        .await
        .expect("list epochs");

    assert_epochs(epochs, starting_epoch, current_epoch, limit as u16)
}

/// Given current epoch, when I fetch it, I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_epoch_fetch() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_epoch_fetch").await;

    // Given some current epoch
    let current_epoch = get_current_epoch(&sdk, &cfg).await;

    let epoch = ExtendedEpochInfo::fetch(&sdk, current_epoch)
        .await
        .expect("list epochs")
        .expect("epoch found");

    assert_eq!(epoch.index(), current_epoch);
}

/// Given future epoch, when I fetch it, I get nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_epoch_fetch_future() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_epoch_fetch_future").await;

    // Given some current epoch
    let current_epoch = get_current_epoch(&sdk, &cfg).await;

    let epoch = ExtendedEpochInfo::fetch(&sdk, current_epoch + 10)
        .await
        .expect("list epochs");

    assert!(epoch.is_none());
}

/// Fetch current epoch from the platform.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_epoch_fetch_current() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_epoch_fetch_current").await;

    // Given some current epoch
    let expected_epoch = get_current_epoch(&sdk, &cfg).await;

    let epoch = ExtendedEpochInfo::fetch_current(&sdk)
        .await
        .expect("fetch current epoch");

    assert_eq!(epoch.index(), expected_epoch);

    tracing::info!(epoch = ?epoch, "current epoch");
}
