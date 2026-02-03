mod v0_tests;
mod v1_tests;

use crate::config::{ExecutionConfig, PlatformConfig, PlatformTestConfig, ValidatorSetConfig};
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::fee::Credits;
pub use dpp::identifier::Identifier;
pub use dpp::identity::{Identity, IdentityPublicKey, IdentityV0};
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use rand::prelude::StdRng;
use rand::SeedableRng;
use simple_signer::signer::SimpleSigner;
use std::collections::BTreeMap;

pub struct TestData<T> {
    pub data_contract: DataContract,
    pub platform: TempPlatform<T>,
}

pub fn setup_identity(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    seed: u64,
    credits: Credits,
) -> (Identity, SimpleSigner, IdentityPublicKey) {
    let platform_version = PlatformVersion::latest();
    let mut signer = SimpleSigner::default();

    let mut rng = StdRng::seed_from_u64(seed);

    let (master_key, master_private_key) =
        IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
            0,
            &mut rng,
            platform_version,
        )
        .expect("expected to get key pair");

    signer.add_identity_public_key(master_key.clone(), master_private_key);

    let (critical_public_key, private_key) =
        IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
            1,
            &mut rng,
            platform_version,
        )
        .expect("expected to get key pair");

    signer.add_identity_public_key(critical_public_key.clone(), private_key);

    let identity: Identity = IdentityV0 {
        id: Identifier::random_with_rng(&mut rng),
        public_keys: BTreeMap::from([(0, master_key.clone()), (1, critical_public_key.clone())]),
        balance: credits,
        revision: 0,
    }
    .into();

    // We just add this identity to the system first

    platform
        .drive
        .add_new_identity(
            identity.clone(),
            false,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to add a new identity");

    (identity, signer, critical_public_key)
}

pub fn apply_contract(
    platform: &TempPlatform<MockCoreRPCLike>,
    data_contract: &DataContract,
    block_info: BlockInfo,
) {
    let platform_version = PlatformVersion::latest();
    platform
        .drive
        .apply_contract(
            data_contract,
            block_info,
            true,
            None,
            None,
            platform_version,
        )
        .expect("to apply contract");
}

pub fn setup_test() -> TestData<MockCoreRPCLike> {
    let platform_version = PlatformVersion::latest();
    let data_contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();

    let config = PlatformConfig {
        validator_set: ValidatorSetConfig {
            quorum_size: 10,
            ..Default::default()
        },
        execution: ExecutionConfig {
            verify_sum_trees: true,
            ..Default::default()
        },
        block_spacing_ms: 300,
        testing_configs: PlatformTestConfig::default_minimal_verifications(),
        ..Default::default()
    };
    let platform = TestPlatformBuilder::new()
        .with_config(config)
        .build_with_mock_rpc();

    TestData {
        data_contract,
        platform: platform.set_initial_state_structure(),
    }
}
