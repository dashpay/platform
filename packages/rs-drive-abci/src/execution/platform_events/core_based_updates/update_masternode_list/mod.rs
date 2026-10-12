mod update_state_masternode_list;
mod v0;
mod v1;

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
            1 => self.update_masternode_list_v1(
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
                known_versions: vec![0, 1],
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
    use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;
    use crate::platform_types::validator_set::{ValidatorSet, ValidatorSetExt};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::bls_signatures::{Bls12381G2Impl, SecretKey};
    use dpp::core_types::validator_set::v0::ValidatorSetV0;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{Network, ProTxHash, PubkeyHash, QuorumHash};
    use dpp::dashcore_rpc::json::{
        DMNStateDiff, MasternodeListDiff, MasternodeListItem, MasternodeType,
    };
    use dpp::identifier::MasternodeIdentifiers;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{KeyID, Purpose};
    use dpp::prelude::Identifier;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use serde_json::{json, Value};
    use std::collections::{BTreeMap, BTreeSet};
    use std::env;
    use std::fs::File;
    use std::io::BufReader;
    use std::net::SocketAddr;
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

    /// A `protx listdiff` response from the mainnet fixtures. With
    /// `core_23_addresses`, every masternode and diff also lists the
    /// `addresses` Core 23 prints beside the legacy fields. `evonodes`
    /// collects the ProTx hashes of the evonodes listed so far.
    fn mainnet_list_diff(
        file_name: &str,
        core_23_addresses: bool,
        evonodes: &mut BTreeSet<String>,
    ) -> MasternodeListDiff {
        let json = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/supporting_files/mainnet_protx_list_diffs")
                .join(file_name),
        )
        .expect("expected to read the masternode list diff");
        let mut list_diff: Value = serde_json::from_str(&json).expect("expected JSON");
        for masternode in list_diff["addedMNs"]
            .as_array_mut()
            .expect("expected added masternodes")
        {
            let is_evonode = masternode["type"] == "Evo";
            if is_evonode {
                let pro_tx_hash = masternode["proTxHash"].as_str().expect("a ProTx hash");
                evonodes.insert(pro_tx_hash.to_string());
            }
            if core_23_addresses {
                let addresses = core_23_state_addresses(&masternode["state"], is_evonode);
                masternode["state"]["addresses"] = addresses;
            }
        }
        if core_23_addresses {
            for update in list_diff["updatedMNs"]
                .as_array_mut()
                .expect("expected updated masternodes")
            {
                for (pro_tx_hash, state_diff) in update.as_object_mut().expect("a diff") {
                    let is_evonode = evonodes.contains(pro_tx_hash);
                    if let Some(addresses) = core_23_diff_addresses(state_diff, is_evonode) {
                        state_diff["addresses"] = addresses;
                    }
                }
            }
        }
        serde_json::from_value(list_diff).expect("expected a masternode list diff")
    }

    /// A listed `service`, `None` for a masternode without an address, which
    /// Core lists as the unspecified address with port 0.
    fn listed_service(service: &Value) -> Option<SocketAddr> {
        let service: SocketAddr = service
            .as_str()
            .expect("a service")
            .parse()
            .expect("an ip:port service");
        (!(service.ip().is_unspecified() && service.port() == 0)).then_some(service)
    }

    /// `port` on the IP of `service`, as Core prints a network address.
    fn on_service_ip(service: SocketAddr, port: u16) -> String {
        SocketAddr::new(service.ip(), port).to_string()
    }

    fn listed_port(port: &Value) -> u16 {
        port.as_u64()
            .and_then(|port| u16::try_from(port).ok())
            .expect("a port")
    }

    /// The `addresses` Core 23 lists for a masternode that predates extended
    /// addresses (`GetNetInfoWithLegacyFields`): its core entry is `service`,
    /// and an evonode's platform entries pair the IP of `service` with the
    /// legacy ports. A masternode without an address lists no entry.
    fn core_23_state_addresses(state: &Value, is_evonode: bool) -> Value {
        let Some(service) = listed_service(&state["service"]) else {
            return json!({});
        };
        let mut addresses = json!({ "core_p2p": [service.to_string()] });
        if is_evonode {
            let http_port = listed_port(&state["platformHTTPPort"]);
            let p2p_port = listed_port(&state["platformP2PPort"]);
            addresses["platform_https"] = json!([on_service_ip(service, http_port)]);
            addresses["platform_p2p"] = json!([on_service_ip(service, p2p_port)]);
        }
        addresses
    }

    /// The `addresses` Core 23 lists in the diff of a masternode that
    /// predates extended addresses (`CDeterministicMNStateDiff::ToJson`): with
    /// a changed `service`, its core entry; for an evonode, each changed
    /// platform port, on the IP of the changed `service`, else on the
    /// placeholder host 255.255.255.255. `None` when that lists nothing.
    fn core_23_diff_addresses(state_diff: &Value, is_evonode: bool) -> Option<Value> {
        let service: Option<SocketAddr> = state_diff.get("service").map(|service| {
            service
                .as_str()
                .expect("a service")
                .parse()
                .expect("an ip:port service")
        });
        let mut addresses = serde_json::Map::new();
        if let Some(service) = state_diff.get("service").and_then(listed_service) {
            addresses.insert("core_p2p".to_string(), json!([service.to_string()]));
        }
        if is_evonode {
            for (port_field, purpose) in [
                ("platformP2PPort", "platform_p2p"),
                ("platformHTTPPort", "platform_https"),
            ] {
                if let Some(port) = state_diff.get(port_field).map(listed_port) {
                    let entry = match service {
                        Some(service) => on_service_ip(service, port),
                        None => format!("255.255.255.255:{port}"),
                    };
                    addresses.insert(purpose.to_string(), json!([entry]));
                }
            }
        }
        (!addresses.is_empty()).then_some(Value::Object(addresses))
    }

    /// The validator every binary before nested addresses builds for a listed
    /// evonode: its ports are the legacy port fields as listed, and it is a
    /// validator only with both of them and a platform node id.
    fn validator_from_legacy_ports(evonode: &MasternodeListItem) -> Option<ValidatorV0> {
        let state = &evonode.state;
        #[allow(deprecated)]
        let (p2p_port, http_port) = (
            state.legacy_platform_p2p_port?,
            state.legacy_platform_http_port?,
        );
        Some(ValidatorV0 {
            pro_tx_hash: evonode.pro_tx_hash,
            public_key: None,
            node_ip: state.service.ip().to_string(),
            node_id: PubkeyHash::from_byte_array(state.platform_node_id?),
            core_port: state.service.port(),
            platform_http_port: http_port as u16,
            platform_p2p_port: p2p_port as u16,
            is_banned: state.pose_ban_height.is_some(),
        })
    }

    /// How every binary before nested addresses refreshes a validator-set
    /// member from its evonode's diff: on a ban, `service` or legacy P2P port
    /// change, it takes the ban, the IP and the legacy ports the diff lists.
    fn refresh_from_legacy_ports(member: &mut ValidatorV0, state_diff: &DMNStateDiff) {
        #[allow(deprecated)]
        let (p2p_port, http_port) = (
            state_diff.legacy_platform_p2p_port,
            state_diff.legacy_platform_http_port,
        );
        if state_diff.pose_ban_height.is_none()
            && state_diff.service.is_none()
            && p2p_port.is_none()
        {
            return;
        }
        if let Some(pose_ban_height) = state_diff.pose_ban_height {
            member.is_banned = pose_ban_height.is_some();
        }
        if let Some(service) = state_diff.service {
            member.node_ip = service.ip().to_string();
        }
        if let Some(p2p_port) = p2p_port {
            member.platform_p2p_port = p2p_port as u16;
        }
        if let Some(http_port) = http_port {
            member.platform_http_port = http_port as u16;
        }
    }

    /// Asserts that `state` lists exactly the `listed` evonodes and gives each
    /// the validator binaries before nested addresses build for it.
    fn assert_validators_from_legacy_ports(
        state: &PlatformState,
        listed: &BTreeMap<ProTxHash, MasternodeListItem>,
        context: &str,
    ) {
        assert!(
            state.hpmn_masternode_list().keys().eq(listed.keys()),
            "{context}: the same evonodes are listed"
        );
        for (pro_tx_hash, evonode) in listed {
            assert_eq!(
                ValidatorV0::new_validator_if_masternode_in_state(*pro_tx_hash, None, state),
                validator_from_legacy_ports(evonode),
                "{context}: evonode {pro_tx_hash}"
            );
        }
    }

    /// Validator-set membership decides quorum rotation, and every binary
    /// before nested addresses built it from the legacy port fields alone.
    /// For every mainnet evonode, as the fixtures list it and with the
    /// `addresses` Core 23 lists beside the legacy fields, the validator a new
    /// quorum gets and the member an existing set refreshes from a diff must
    /// be exactly what those binaries build.
    #[test]
    fn should_build_the_validators_earlier_binaries_built_from_mainnet_masternode_lists() {
        for platform_version in [
            PlatformVersion::get(13).expect("protocol version 13"),
            PlatformVersion::latest(),
        ] {
            let init_core_height = 2128896;
            let next_core_height = 2129440;
            let quorum_hash = QuorumHash::from_byte_array([0x55; 32]);

            for core_23_addresses in [false, true] {
                let context = if core_23_addresses {
                    "with Core 23 addresses"
                } else {
                    "as listed"
                };
                let mut evonodes = BTreeSet::new();
                let init_list =
                    mainnet_list_diff("1-2128896.json", core_23_addresses, &mut evonodes);
                let list_diff =
                    mainnet_list_diff("2128896-2129440.json", core_23_addresses, &mut evonodes);
                assert!(
                    list_diff
                        .updated_mns
                        .iter()
                        .any(|(pro_tx_hash, state_diff)| {
                            evonodes.contains(&pro_tx_hash.to_string())
                                && state_diff.service.is_some()
                        }),
                    "the diff moves an evonode"
                );

                let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();
                let responses = [init_list.clone(), list_diff.clone()];
                platform
                    .core_rpc
                    .expect_get_protx_diff_with_masternodes()
                    .returning(move |_base_block, block| {
                        Ok(responses[usize::from(block != init_core_height)].clone())
                    });
                let mut state = PlatformState::default_with_protocol_versions(
                    platform_version.protocol_version,
                    platform_version.protocol_version,
                    &PlatformConfig::default_for_network(Network::Mainnet),
                )
                .expect("platform state");

                // The evonodes as every earlier binary holds them.
                let is_evonode =
                    |masternode: &&MasternodeListItem| masternode.node_type == MasternodeType::Evo;
                let mut listed: BTreeMap<ProTxHash, MasternodeListItem> = init_list
                    .added_mns
                    .iter()
                    .filter(is_evonode)
                    .map(|evonode| (evonode.pro_tx_hash, evonode.clone()))
                    .collect();

                let update = |state: &mut PlatformState, core_height, from_scratch| {
                    if platform_version.protocol_version == 13 {
                        platform.update_state_masternode_list_v0(state, core_height, from_scratch)
                    } else {
                        platform.update_state_masternode_list_v1(state, core_height, from_scratch)
                    }
                    .expect("expected to apply the masternode list")
                };
                update(&mut state, init_core_height, true);
                assert_validators_from_legacy_ports(&state, &listed, context);

                let members: BTreeMap<ProTxHash, ValidatorV0> = listed
                    .iter()
                    .filter_map(|(pro_tx_hash, evonode)| {
                        validator_from_legacy_ports(evonode).map(|member| (*pro_tx_hash, member))
                    })
                    .collect();
                assert_eq!(
                    members.len(),
                    listed.len(),
                    "{context}: every evonode is a member"
                );
                state.insert_validator_set(
                    quorum_hash,
                    ValidatorSet::V0(ValidatorSetV0 {
                        quorum_hash,
                        quorum_index: None,
                        core_height: init_core_height,
                        members: members.clone(),
                        threshold_public_key: SecretKey::<Bls12381G2Impl>::random(
                            &mut StdRng::seed_from_u64(1),
                        )
                        .public_key(),
                    }),
                );

                update(&mut state, next_core_height, false);

                let mut refreshed_members = members.clone();
                for evonode in list_diff.added_mns.iter().filter(is_evonode) {
                    listed.insert(evonode.pro_tx_hash, evonode.clone());
                }
                for (pro_tx_hash, state_diff) in &list_diff.updated_mns {
                    if let Some(evonode) = listed.get_mut(pro_tx_hash) {
                        evonode.state.apply_diff(state_diff.clone());
                    }
                    if let Some(member) = refreshed_members.get_mut(pro_tx_hash) {
                        refresh_from_legacy_ports(member, state_diff);
                    }
                }
                for pro_tx_hash in &list_diff.removed_mns {
                    listed.remove(pro_tx_hash);
                    refreshed_members.remove(pro_tx_hash);
                }

                assert_validators_from_legacy_ports(&state, &listed, context);
                assert_ne!(
                    refreshed_members, members,
                    "{context}: the diff refreshes members"
                );
                assert_eq!(
                    state.validator_sets()[&quorum_hash].members(),
                    &refreshed_members,
                    "{context}: the refreshed members"
                );
            }
        }
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

        // An Evo on an IPv6 primary address is advertised to Tenderdash with
        // its host in brackets, as a URL authority requires.
        let ipv6_evo_validator =
            validator("aecd2830b843e6a84283ba290492a213e355cea7c5026e6118a21e1bfbc36783")
                .expect("expected the IPv6 Evo to be a validator");
        assert_eq!(ipv6_evo_validator.node_ip, "2001:db8::4");
        assert_eq!(
            node_addresses([ipv6_evo_validator]),
            ["tcp://4bcc85253e395ec272998a0722ac3ee1dd3965f7@[2001:db8::4]:26656"]
        );

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

    /// The quorum of the validator set `state_after_core_v24_lists` adds.
    const VALIDATOR_QUORUM_HASH: [u8; 32] = [0x55; 32];

    /// The node addresses Drive hands Tenderdash for `validators`, as the
    /// members of one validator set.
    fn node_addresses(validators: impl IntoIterator<Item = ValidatorV0>) -> Vec<String> {
        let quorum_hash = QuorumHash::from_byte_array(VALIDATOR_QUORUM_HASH);
        ValidatorSet::V0(ValidatorSetV0 {
            quorum_hash,
            quorum_index: None,
            core_height: 1,
            members: validators
                .into_iter()
                .map(|validator| (validator.pro_tx_hash, validator))
                .collect(),
            threshold_public_key: SecretKey::<Bls12381G2Impl>::random(&mut StdRng::seed_from_u64(
                1,
            ))
            .public_key(),
        })
        .to_update()
        .validator_updates
        .into_iter()
        .map(|validator_update| validator_update.node_address)
        .collect()
    }

    /// Runs one masternode list update per `(core height, fixture)`, in order,
    /// as Drive runs one per block, with Core reporting the fixture's
    /// `protx listdiff` JSON from `core_v24_protx_list_diffs` for that height.
    /// Before the last update, one validator set lists every evonode that is a
    /// validator then. Returns the state after the last update.
    fn state_after_core_v24_lists(fixtures: &[(u32, &str)]) -> PlatformState {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default())
            .build_with_mock_rpc()
            .set_genesis_state();

        let lists: BTreeMap<u32, String> = fixtures
            .iter()
            .map(|(core_height, file_name)| {
                let json = std::fs::read_to_string(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/supporting_files/core_v24_protx_list_diffs")
                        .join(file_name),
                )
                .expect("expected to read the masternode list diff");
                (*core_height, json)
            })
            .collect();
        platform
            .core_rpc
            .expect_get_protx_diff_with_masternodes()
            .returning(move |_base_block, block| Ok(serde_json::from_str(&lists[&block])?));

        let transaction = platform.drive.grove.start_transaction();
        let mut platform_state: Option<PlatformState> = None;
        let mut block_platform_state = platform.state.load().as_ref().clone();
        for (index, (core_height, _)) in fixtures.iter().enumerate() {
            let block = index as u64 + 1;
            if block == fixtures.len() as u64 {
                let members = block_platform_state
                    .hpmn_masternode_list()
                    .keys()
                    .filter_map(|pro_tx_hash| {
                        ValidatorV0::new_validator_if_masternode_in_state(
                            *pro_tx_hash,
                            None,
                            &block_platform_state,
                        )
                        .map(|member| (*pro_tx_hash, member))
                    })
                    .collect();
                let quorum_hash = QuorumHash::from_byte_array(VALIDATOR_QUORUM_HASH);
                block_platform_state.insert_validator_set(
                    quorum_hash,
                    ValidatorSet::V0(ValidatorSetV0 {
                        quorum_hash,
                        quorum_index: None,
                        core_height: *core_height,
                        members,
                        threshold_public_key: SecretKey::<Bls12381G2Impl>::random(
                            &mut StdRng::seed_from_u64(1),
                        )
                        .public_key(),
                    }),
                );
            }
            platform
                .update_masternode_list(
                    platform_state.as_ref(),
                    &mut block_platform_state,
                    *core_height,
                    index == 0,
                    &BlockInfo {
                        height: block,
                        core_height: *core_height,
                        time_ms: block,
                        ..Default::default()
                    },
                    &transaction,
                    platform_version,
                )
                .expect("expected to process the Core v24 masternode list");
            platform_state = Some(block_platform_state.clone());
        }
        block_platform_state
    }

    /// The validator of `pro_tx_hash` in the validator set
    /// `state_after_core_v24_lists` adds, and the one a new quorum gets.
    fn existing_and_new_validator(
        state: &PlatformState,
        pro_tx_hash: &str,
    ) -> (Option<ValidatorV0>, Option<ValidatorV0>) {
        let pro_tx_hash = ProTxHash::from_str(pro_tx_hash).expect("expected a pro tx hash");
        let validator_set =
            &state.validator_sets()[&QuorumHash::from_byte_array(VALIDATOR_QUORUM_HASH)];
        (
            validator_set.members().get(&pro_tx_hash).cloned(),
            ValidatorV0::new_validator_if_masternode_in_state(pro_tx_hash, None, state),
        )
    }

    /// An ExtAddr evonode keeps its platform ports in its network info, so a
    /// ProUpServTx that only moves them reaches Drive as a diff with `service`
    /// and the whole `addresses`, and no legacy port field. Tenderdash must be
    /// told the new ports, by the existing validator set and by one built
    /// later, or it keeps dialling the old ones.
    #[test]
    fn should_move_the_validator_of_an_ext_addr_evonode_on_a_core_v24_port_only_diff() {
        let state = state_after_core_v24_lists(&[
            (1250, "1-1250.json"),
            (1260, "1250-1260.json"),
            (1270, "1260-1270.json"),
        ]);

        let expected = ValidatorV0 {
            pro_tx_hash: ProTxHash::from_str(
                "ca3d32c4f2ef62eaf736bf41bb3235a832ec1eb6d8dfaccd4d3555e70f6757b0",
            )
            .expect("expected a pro tx hash"),
            public_key: None,
            node_ip: "192.0.2.40".to_string(),
            node_id: PubkeyHash::from_byte_array(
                hex::decode("9e391c2c041a122a779bafe09d6c47ea600dfcbe")
                    .expect("expected hex")
                    .try_into()
                    .expect("expected 20 bytes"),
            ),
            core_port: 9999,
            platform_http_port: 8443,
            platform_p2p_port: 36657,
            is_banned: false,
        };
        assert_eq!(
            existing_and_new_validator(
                &state,
                "ca3d32c4f2ef62eaf736bf41bb3235a832ec1eb6d8dfaccd4d3555e70f6757b0"
            ),
            (Some(expected.clone()), Some(expected))
        );
    }

    /// Core lists an evonode whose primary core address is a Tor address with
    /// that address as `service`, which reads as `[::]:0`. It becomes a
    /// validator on the unspecified IP with the ports of its platform entries:
    /// unreachable, but its list does not fail the block.
    #[test]
    fn should_make_an_evonode_with_a_tor_primary_address_a_validator_on_the_unspecified_ip() {
        let state = state_after_core_v24_lists(&[
            (1250, "1-1250.json"),
            (1260, "1250-1260.json"),
            (1270, "1260-1270.json"),
        ]);

        let (_, new_quorum_validator) = existing_and_new_validator(
            &state,
            "99e719c608d603658f296acd00f271345a29e391e2d717d3615062f9a961920c",
        );
        assert_eq!(
            new_quorum_validator,
            Some(ValidatorV0 {
                pro_tx_hash: ProTxHash::from_str(
                    "99e719c608d603658f296acd00f271345a29e391e2d717d3615062f9a961920c",
                )
                .expect("expected a pro tx hash"),
                public_key: None,
                node_ip: "::".to_string(),
                node_id: PubkeyHash::from_byte_array(
                    hex::decode("9cea9116b333eba7631804e1be7efd62c56d5b58")
                        .expect("expected hex")
                        .try_into()
                        .expect("expected 20 bytes"),
                ),
                core_port: 0,
                platform_http_port: 443,
                platform_p2p_port: 26656,
                is_banned: false,
            })
        );
        assert_eq!(
            node_addresses(new_quorum_validator),
            ["tcp://9cea9116b333eba7631804e1be7efd62c56d5b58@[::]:26656"]
        );
    }

    /// Before V24, a ProUpServTx that only moves a legacy evonode's P2P port
    /// reaches Drive as the legacy port field and a platform entry on the
    /// placeholder host 255.255.255.255, since the diff carries no address.
    /// The validator takes the new port and keeps the evonode's IP.
    #[test]
    fn should_move_the_validator_of_a_legacy_evonode_on_a_core_v24_port_diff() {
        let state = state_after_core_v24_lists(&[
            (1000, "before_v24/1-1000.json"),
            (1010, "before_v24/1000-1010.json"),
        ]);

        let expected = ValidatorV0 {
            pro_tx_hash: ProTxHash::from_str(
                "86e4a4274da5610322340e2f34adf8f6f2899f78b2ce1c31281b6d1754a88d76",
            )
            .expect("expected a pro tx hash"),
            public_key: None,
            node_ip: "192.0.2.60".to_string(),
            node_id: PubkeyHash::from_byte_array(
                hex::decode("416e5b04d3047def070bea5175431ca7732b4b7b")
                    .expect("expected hex")
                    .try_into()
                    .expect("expected 20 bytes"),
            ),
            core_port: 9999,
            platform_http_port: 443,
            platform_p2p_port: 36657,
            is_banned: false,
        };
        assert_eq!(
            existing_and_new_validator(
                &state,
                "86e4a4274da5610322340e2f34adf8f6f2899f78b2ce1c31281b6d1754a88d76"
            ),
            (Some(expected.clone()), Some(expected))
        );
    }
}

#[cfg(test)]
mod versioning_tests;
