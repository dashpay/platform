//! `create_funded_asset_lock_proof_with_funding` takes an `AssetLockBuildAmount`,
//! so a caller outside the crate must be able to name one.

use platform_wallet::wallet::asset_lock::AssetLockBuildAmount;

#[test]
fn drain_all_is_nameable_outside_the_crate() {
    let amount = AssetLockBuildAmount::DrainAll {
        minimum_lock_duffs: Some(1),
    };
    assert_ne!(amount, AssetLockBuildAmount::Exact(1));
}
