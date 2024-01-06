//! Test mock wallet

use crate::common::setup_logs;
use crate::config::Config;
use dash_platform_sdk::{mock::wallet::MockWallet, wallet::Wallet};
use dashcore_rpc::dashcore::Network;
use dpp::version::PlatformVersion;
use tokio_util::sync::CancellationToken;

// TODO: implement mocking for wallet operations
#[cfg(all(feature = "network-testing", not(feature = "offline-testing")))]
#[tokio::test]
async fn test_asset_lock() {
    setup_logs();

    let cfg = Config::new();
    let cancel = CancellationToken::new();
    let version = PlatformVersion::latest();

    let wallet = MockWallet::new_mock(
        Network::Devnet,
        &cfg.platform_host,
        cfg.core_port,
        &cfg.core_user,
        &cfg.core_password,
        cancel,
        version,
    )
    .expect("create mock wallet");

    tokio::select! {
        asset_lock = wallet.lock_assets(1) => {
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
            panic!("timeout");
        }
    }
}
