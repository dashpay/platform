mod update_state_masternode_list;
mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;

use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the masternode list in the platform state based on changes in the masternode list
    /// from Dash Core between two block heights.
    ///
    /// This function fetches the masternode list difference between the current core block height
    /// and the previous core block height, then updates the full masternode list and the
    /// HPMN (high performance masternode) list in the platform state accordingly.
    ///
    /// # Arguments
    ///
    /// * `state` - A mutable reference to the platform state to be updated.
    /// * `core_block_height` - The current block height in the Dash Core.
    /// * `transaction` - The current groveDB transaction.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Returns `Ok(())` if the update is successful. Returns an error if
    ///   there is a problem fetching the masternode list difference or updating the state.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_masternode_list(
        &self,
        platform_state: Option<&PlatformState>,
        block_platform_state: &mut PlatformState,
        core_block_height: u32,
        is_init_chain: bool,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .update_masternode_list
        {
            0 => self.update_masternode_list_v0(
                platform_state,
                block_platform_state,
                core_block_height,
                is_init_chain,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "update_masternode_list".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::PlatformConfig;
    use crate::platform_types::platform_state::platform_state_for_saving::v2::{
        deserialize_masternode_entry, serialize_masternode_entry,
    };
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::platform_types::validator::v0::{NewValidatorIfMasternodeInState, ValidatorV0};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::ProTxHash;
    use dpp::dashcore_rpc::json::MasternodeListDiff;
    use dpp::identifier::MasternodeIdentifiers;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{KeyID, Purpose};
    use dpp::prelude::Identifier;
    use std::collections::BTreeMap;
    use std::env;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::PathBuf;
    use std::str::FromStr;

    #[test]
    fn test_update_masternode_list() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig::default();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();

        let mut init_chain_platform_state = platform_state.as_ref().clone();

        let genesis_core_block_height = 2128896;
        let first_block_core_block_height = 2129440;
        let genesis_time = 1;
        let first_block_time = 2;

        let genesis_block_info = BlockInfo {
            height: 1,
            core_height: genesis_core_block_height,
            time_ms: genesis_time,
            ..Default::default()
        };

        let first_block_info = BlockInfo {
            height: 1,
            core_height: first_block_core_block_height,
            time_ms: first_block_time,
            ..Default::default()
        };

        fn adjust_path_based_on_current_dir(relative_path: &str) -> PathBuf {
            let current_dir = env::current_dir().expect("expected to get current directory");
            // Check if the current directory ends with "platform"

            if current_dir.ends_with("platform") {
                current_dir
                    .join("packages/rs-drive-abci")
                    .join(relative_path)
            } else {
                current_dir.join(relative_path)
            }
        }

        platform
            .core_rpc
            .expect_get_protx_diff_with_masternodes()
            .returning(move |_base_block, block| {
                if block == 2128896 {
                    let file_path = adjust_path_based_on_current_dir(
                        "tests/supporting_files/mainnet_protx_list_diffs/1-2128896.json",
                    );
                    // println!(
                    //     "Current directory: {:?}, using {:?}",
                    //     std::env::current_dir(),
                    //     &file_path
                    // );
                    // Deserialize the first JSON file
                    let file = File::open(file_path).expect("expected to open file");
                    let reader = BufReader::new(file);
                    let init_chain_masternode_list_diff: MasternodeListDiff =
                        serde_json::from_reader(reader)
                            .expect("expected to deserialize into a masternode list diff");

                    Ok(init_chain_masternode_list_diff)
                } else {
                    // Deserialize the second JSON file
                    let file = File::open(adjust_path_based_on_current_dir(
                        "tests/supporting_files/mainnet_protx_list_diffs/2128896-2129440.json",
                    ))
                    .expect("expected to open file");
                    let reader = BufReader::new(file);
                    let block_1_masternode_list_diff: MasternodeListDiff =
                        serde_json::from_reader(reader)
                            .expect("expected to deserialize into a masternode list diff");

                    Ok(block_1_masternode_list_diff)
                }
            });

        let transaction = platform.drive.grove.start_transaction();

        platform
            .update_masternode_list(
                None,
                &mut init_chain_platform_state,
                genesis_core_block_height,
                true,
                &genesis_block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to update masternode list");

        let platform_state = init_chain_platform_state.clone();

        let mut block_platform_state = platform_state.clone();

        platform
            .update_masternode_list(
                Some(&platform_state),
                &mut block_platform_state,
                first_block_core_block_height,
                false,
                &first_block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to update masternode list");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
    }

    /// A platform at `platform_version` whose mock Core returns Dash Core v24 `protx listdiff`
    /// output: the diff from height 1 to 1250 adds a legacy Evo, a shared masternode, Regular
    /// masternodes with one and with two payouts (one of them with a Tor primary address), an
    /// Evo without addresses and an IPv6 Evo with two payouts; the diff to 1260 moves the
    /// legacy Evo to extended addresses and changes two payout lists.
    fn platform_with_core_v24_masternode_list(
        platform_version: &PlatformVersion,
    ) -> TempPlatform<MockCoreRPCLike> {
        let mut platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default())
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        platform
            .core_rpc
            .expect_get_protx_diff_with_masternodes()
            .returning(move |_base_block, block| {
                let file_name = if block == 1250 {
                    "1-1250.json"
                } else {
                    "1250-1260.json"
                };
                let json = std::fs::read_to_string(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/supporting_files/core_v24_protx_list_diffs")
                        .join(file_name),
                )
                .expect("expected to read the masternode list diff");
                // Parse it as the Core RPC client parses the `protx listdiff` response, so a
                // response the client rejects fails the update with the same error.
                Ok(serde_json::from_str(&json)?)
            });

        platform
    }

    /// Up to protocol version 13 an added masternode needs a payout and an owner address for
    /// its owner identity. A Dash Core v24 list adds shared and extended-address masternodes
    /// that have neither or no payout address, so the update fails with
    /// `DashCoreBadResponseError` and the block with it, as on every release that cannot parse
    /// such a list.
    #[test]
    fn should_fail_on_a_core_v24_masternode_list_up_to_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        let platform = platform_with_core_v24_masternode_list(platform_version);

        let transaction = platform.drive.grove.start_transaction();
        let mut init_chain_platform_state = platform.state.load().as_ref().clone();

        match platform.update_masternode_list(
            None,
            &mut init_chain_platform_state,
            1250,
            true,
            &BlockInfo {
                height: 1,
                core_height: 1250,
                time_ms: 1,
                ..Default::default()
            },
            &transaction,
            platform_version,
        ) {
            Err(Error::Execution(ExecutionError::DashCoreBadResponseError(message))) => assert!(
                message.ends_with("has no payout address"),
                "unexpected message: {message}"
            ),
            other => panic!("expected DashCoreBadResponseError, got {other:?}"),
        }
    }

    /// Dash Core v24 prints masternodes that differ from every earlier list: a shared
    /// masternode has no owner, payout or collateral address; an extended-address masternode
    /// has a `payouts` list instead of `payoutAddress`; a Tor primary address is not an
    /// `ip:port` `service`; an Evo without addresses has `-1` platform ports. From protocol
    /// version 14 the list is processed rather than failing the block, and the masternode
    /// identities are:
    /// - shared: voter and operator identities, no owner identity (it has no owner key);
    /// - one payout: the same owner identity as a legacy payout address (TRANSFER key 0,
    ///   OWNER key 1);
    /// - several payouts: an owner identity with only the OWNER key 1;
    /// - a later payout-list change: revoke obsolete keys and authorize only a sole P2PKH
    ///   recipient, preserving the OWNER key.
    #[test]
    fn should_update_masternode_identities_from_a_core_v24_masternode_list() {
        let platform_version = PlatformVersion::latest();

        let platform = platform_with_core_v24_masternode_list(platform_version);

        let genesis_core_block_height = 1250;
        let first_block_core_block_height = 1260;

        let transaction = platform.drive.grove.start_transaction();

        let mut init_chain_platform_state = platform.state.load().as_ref().clone();

        platform
            .update_masternode_list(
                None,
                &mut init_chain_platform_state,
                genesis_core_block_height,
                true,
                &BlockInfo {
                    height: 1,
                    core_height: genesis_core_block_height,
                    time_ms: 1,
                    ..Default::default()
                },
                &transaction,
                platform_version,
            )
            .expect("expected to process the Core v24 masternode list");

        let platform_state = init_chain_platform_state.clone();
        let mut block_platform_state = platform_state.clone();

        platform
            .update_masternode_list(
                Some(&platform_state),
                &mut block_platform_state,
                first_block_core_block_height,
                false,
                &BlockInfo {
                    height: 1,
                    core_height: first_block_core_block_height,
                    time_ms: 2,
                    ..Default::default()
                },
                &transaction,
                platform_version,
            )
            .expect("expected to process the Core v24 masternode list diff");

        assert_eq!(block_platform_state.full_masternode_list().len(), 7);
        assert_eq!(block_platform_state.hpmn_masternode_list().len(), 3);

        // An Evo without addresses prints `-1` platform ports, which read as no port, so it
        // cannot be a validator, like any Evo without ports. The legacy Evo is one, with the
        // ports and address the diff gave it.
        let validator = |pro_tx_hash: &str| {
            ValidatorV0::new_validator_if_masternode_in_state(
                ProTxHash::from_str(pro_tx_hash).expect("expected a pro tx hash"),
                None,
                &block_platform_state,
            )
        };
        assert!(
            validator("6f1757595185032c808321af3e2e8468fae10b8f91e2a657d7a4c7122f4b2706").is_none()
        );
        let legacy_evo_validator =
            validator("ca3d32c4f2ef62eaf736bf41bb3235a832ec1eb6d8dfaccd4d3555e70f6757b0")
                .expect("expected the legacy Evo to be a validator");
        assert_eq!(legacy_evo_validator.node_ip, "192.0.2.40");
        assert_eq!(legacy_evo_validator.platform_p2p_port, 36656);
        assert_eq!(legacy_evo_validator.platform_http_port, 1443);

        // Every parsed masternode survives a store and reload, except for the payout list and
        // nested addresses, which are not stored.
        for masternode in block_platform_state.full_masternode_list().values() {
            let bytes = serialize_masternode_entry(masternode, platform_version)
                .expect("expected to serialize the masternode entry");
            let mut expected = masternode.clone();
            expected.state.payouts = None;
            expected.state.addresses = None;
            assert_eq!(
                deserialize_masternode_entry(&bytes)
                    .expect("expected to deserialize the masternode entry"),
                expected
            );
        }

        let hash160 = |hex: &str| -> Vec<u8> { hex::decode(hex).expect("expected hex") };

        // The keys and disabled state of an identity; `None` if there is no identity.
        let identity_keys =
            |identity_id: Identifier| -> Option<BTreeMap<KeyID, (Purpose, Vec<u8>, bool)>> {
                platform
                    .drive
                    .fetch_full_identity(
                        identity_id.to_buffer(),
                        Some(&transaction),
                        platform_version,
                    )
                    .expect("expected to fetch an identity")
                    .map(|identity| {
                        identity
                            .public_keys()
                            .iter()
                            .map(|(key_id, key)| {
                                (
                                    *key_id,
                                    (key.purpose(), key.data().to_vec(), key.is_disabled()),
                                )
                            })
                            .collect()
                    })
            };

        let owner_key = |hex: &str| (Purpose::OWNER, hash160(hex), false);
        let transfer_key = |hex: &str| (Purpose::TRANSFER, hash160(hex), false);

        let voting_421c = "421c03add2c804421451c4e022258778175e60d8";
        let voting_064c = "064cd21ff210c1a92adaea49a4768a41c2aef1bc";

        // (pro_tx_hash, voting key hash, operator public key, owner identity keys)
        let expected = [
            (
                // Legacy Evo whose payout address the diff moves into `payouts`
                "ca3d32c4f2ef62eaf736bf41bb3235a832ec1eb6d8dfaccd4d3555e70f6757b0",
                voting_421c,
                "8ed3f0c208efbcfc815cbfb94490dc68cf2e29d44dd9f8a91e20e06057aa110d7062c8ab7ccc85a9ff0c88760157f563".to_string(),
                Some(BTreeMap::from([
                    (0, transfer_key("75d57974b6e29a4a70df57a3b11195ce0a0dc817")),
                    (1, owner_key("1f67d90f35e3c5070c368ae6f3635aac357e47df")),
                ])),
            ),
            (
                // One payout, which the diff replaces with two
                "27978dd892b7c876c238be1a6141461c2824f3497dd0058e160979bc8f0a0bef",
                voting_421c,
                "a792ce1af5f7bb9281053b3934cb8b08d00d075a56498e1a525388ce467f188e8a80911fd96a20982baa9b9678452534".to_string(),
                Some(BTreeMap::from([
                    (0, (Purpose::TRANSFER, hash160("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"), true)),
                    (1, owner_key("b740a2ab3f631e4dfe15debf746364f1d8352c50")),
                ])),
            ),
            (
                // Shared
                "a4d26868017c0ccffe2efe50944ef4211834660cca834c6e9f86dec6a88246fa",
                voting_421c,
                "b1".repeat(48),
                None,
            ),
            (
                // One payout, which the diff replaces with another single payout
                "813a7c3f28817988a8e6ce66e07e43e261e78398373bfbaae94c898645111d6b",
                voting_421c,
                "b2".repeat(48),
                Some(BTreeMap::from([
                    (0, (Purpose::TRANSFER, hash160("5555555555555555555555555555555555555555"), true)),
                    (2, transfer_key("f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1")),
                    (1, owner_key("0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a")),
                ])),
            ),
            (
                // Tor primary address, two payouts; the owner and voting keys are the same
                "3023ffd989974768b0dfc347410ad923fa6d3f1eee90180bd0c435e81cb1a82f",
                voting_064c,
                "b3".repeat(48),
                Some(BTreeMap::from([(1, owner_key(voting_064c))])),
            ),
            (
                // Evo without addresses, one payout
                "6f1757595185032c808321af3e2e8468fae10b8f91e2a657d7a4c7122f4b2706",
                voting_421c,
                "b4".repeat(48),
                Some(BTreeMap::from([
                    (0, transfer_key("9999999999999999999999999999999999999999")),
                    (1, owner_key("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")),
                ])),
            ),
            (
                // Evo with an IPv6 primary address, two payouts
                "aecd2830b843e6a84283ba290492a213e355cea7c5026e6118a21e1bfbc36783",
                voting_421c,
                "b5".repeat(48),
                Some(BTreeMap::from([(
                    1,
                    owner_key("0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c"),
                )])),
            ),
        ];

        for (pro_tx_hash, voting_key_hash, operator_public_key, owner_identity_keys) in expected {
            let pro_tx_hash = ProTxHash::from_str(pro_tx_hash)
                .expect("expected a pro tx hash")
                .to_byte_array();
            let voting_key_hash: [u8; 20] = hash160(voting_key_hash)
                .try_into()
                .expect("expected 20 bytes");
            let operator_public_key = hex::decode(operator_public_key).expect("expected hex");

            assert_eq!(
                identity_keys(pro_tx_hash.into()),
                owner_identity_keys,
                "owner identity of {}",
                hex::encode(pro_tx_hash)
            );
            assert!(
                identity_keys(Identifier::create_voter_identifier(
                    &pro_tx_hash,
                    &voting_key_hash
                ))
                .is_some(),
                "voter identity of {}",
                hex::encode(pro_tx_hash)
            );
            assert!(
                identity_keys(Identifier::create_operator_identifier(
                    &pro_tx_hash,
                    &operator_public_key
                ))
                .is_some(),
                "operator identity of {}",
                hex::encode(pro_tx_hash)
            );
        }
    }
}
