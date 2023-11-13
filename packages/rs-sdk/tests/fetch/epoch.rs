use crate::{common::setup_logs, config::Config};
use dpp::block::epoch::EpochIndex;
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use rs_sdk::platform::{Fetch, FetchMany, LimitQuery};

/// Given some existing identity ID, when I fetch the identity, and I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_epoch_list() {
    setup_logs();

    let cfg = Config::new();
    let id: EpochIndex = 1;

    let mut api = cfg.setup_api().await;

    let epoch = ExtendedEpochInfo::fetch_many(&mut api, id)
        .await
        .expect("list epochs");

    assert_ne!(epoch.len(), 0);
}

/// Given some existing identity ID, when I fetch the identity, and I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_epoch_list_limit() {
    setup_logs();

    let cfg = Config::new();

    let query: LimitQuery<EpochIndex> = LimitQuery {
        query: 1,
        limit: Some(2),
        offset: None,
    };

    let mut sdk = cfg.setup_api().await;

    let epoch = ExtendedEpochInfo::fetch_many(&mut sdk, query)
        .await
        .expect("list epochs");

    assert_ne!(epoch.len(), 0);
}

/// Given some existing identity ID, when I fetch the identity, and I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_epoch_fetch() {
    setup_logs();

    let cfg = Config::new();
    let id: EpochIndex = 1;

    let mut sdk = cfg.setup_api().await;

    let epoch = ExtendedEpochInfo::fetch(&mut sdk, id)
        .await
        .expect("list epochs")
        .expect("epoch found");

    assert_eq!(epoch.index(), 1);
}
