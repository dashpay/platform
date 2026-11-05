use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::validator::v0::{NewValidatorIfMasternodeInState, ValidatorV0};
use crate::test::helpers::setup::TestPlatformBuilder;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore_rpc::json::{MasternodeAddresses, MasternodeListDiff};
use dpp::version::PlatformVersion;
use serde_json::json;

/// An RPC response with legacy identity fields so the port version gate is
/// exercised independently of the owner-identity version gate.
fn conflicting_ports() -> MasternodeListDiff {
    let mut diff: MasternodeListDiff = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/supporting_files/core_v24_protx_list_diffs/before_v24/1-1000.json"
    )))
    .expect("masternode fixture");
    assert_eq!(diff.added_mns.len(), 1);
    let state = &mut diff.added_mns[0].state;
    #[allow(deprecated)]
    {
        state.legacy_platform_p2p_port = Some(26656);
        state.legacy_platform_http_port = Some(443);
    }
    state.addresses = Some(MasternodeAddresses {
        core_p2p: vec![state.service.to_string()],
        platform_p2p: vec!["192.0.2.40:36656".to_string()],
        platform_https: vec!["192.0.2.40:8443".to_string()],
    });
    diff
}

#[test]
fn should_use_nested_ports_only_after_the_protocol_upgrade() {
    for (platform_version, expected_ports) in [
        (PlatformVersion::get(13).expect("PV13"), (26656, 443)),
        (PlatformVersion::latest(), (36656, 8443)),
    ] {
        let diff = conflicting_ports();
        let pro_tx_hash = diff.added_mns[0].pro_tx_hash;
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();
        let update = MasternodeListDiff {
            base_height: 1000,
            block_height: 2000,
            added_mns: vec![],
            removed_mns: vec![],
            updated_mns: vec![(
                pro_tx_hash,
                serde_json::from_value(json!({
                    "addresses": { "platform_p2p": ["192.0.2.40:36657"] }
                }))
                .expect("nested-only diff"),
            )],
        };
        let flat_update = MasternodeListDiff {
            base_height: 2000,
            block_height: 2001,
            added_mns: vec![],
            removed_mns: vec![],
            updated_mns: vec![(
                pro_tx_hash,
                serde_json::from_value(json!({
                    "platformP2PPort": 27656,
                    "platformHTTPPort": 1443
                }))
                .expect("flat-only diff"),
            )],
        };
        platform
            .core_rpc
            .expect_get_protx_diff_with_masternodes()
            .returning(move |_, block| {
                Ok(if block == 1000 {
                    diff.clone()
                } else if block == 2000 {
                    update.clone()
                } else {
                    flat_update.clone()
                })
            });
        let mut state = platform.state.load().as_ref().clone();
        let transaction = platform.drive.grove.start_transaction();
        platform
            .update_masternode_list(
                None,
                &mut state,
                1000,
                true,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("update masternode list");
        let validator =
            ValidatorV0::new_validator_if_masternode_in_state(pro_tx_hash, None, &state)
                .expect("validator");
        assert_eq!(
            (validator.platform_p2p_port, validator.platform_http_port),
            expected_ports,
            "protocol version {}",
            platform_version.protocol_version
        );
        #[allow(deprecated)]
        let stored_ports = (
            state.hpmn_masternode_list()[&pro_tx_hash]
                .state
                .legacy_platform_p2p_port,
            state.hpmn_masternode_list()[&pro_tx_hash]
                .state
                .legacy_platform_http_port,
        );
        assert_eq!(
            stored_ports,
            (Some(expected_ports.0.into()), Some(expected_ports.1.into()))
        );

        let previous_state = state.clone();
        platform
            .update_masternode_list(
                Some(&previous_state),
                &mut state,
                2000,
                false,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("nested-only update");
        let validator =
            ValidatorV0::new_validator_if_masternode_in_state(pro_tx_hash, None, &state)
                .expect("updated validator");
        assert_eq!(
            (validator.platform_p2p_port, validator.platform_http_port),
            (
                if platform_version.protocol_version == 13 {
                    26656
                } else {
                    36657
                },
                expected_ports.1
            ),
            "nested-only diff at protocol version {}",
            platform_version.protocol_version,
        );

        let previous_state = state.clone();
        platform
            .update_masternode_list(
                Some(&previous_state),
                &mut state,
                2001,
                false,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("flat-only update");
        let validator =
            ValidatorV0::new_validator_if_masternode_in_state(pro_tx_hash, None, &state)
                .expect("updated validator");
        assert_eq!(
            (validator.platform_p2p_port, validator.platform_http_port),
            (27656, 1443),
            "flat-only diff at protocol version {}",
            platform_version.protocol_version,
        );
    }
}

/// Historical freeze: protocol versions 1 to 14 executed their blocks with
/// `update_masternode_list` 0 or 1 and `update_operator_identity` 0, which ignore the operator
/// payouts of Core's diffs, so they must keep selecting them for those blocks to replay to the
/// same state. The generations that apply the payouts are exercised through the dispatcher at
/// the latest protocol version (`operator_payout_tests`).
#[test]
fn should_keep_the_masternode_generations_protocol_versions_1_to_14_executed_with() {
    for protocol_version in 1..=14 {
        let core_based_updates = &PlatformVersion::get(protocol_version)
            .expect("expected a protocol version")
            .drive_abci
            .methods
            .core_based_updates;
        let update_masternode_list = if protocol_version == 14 { 1 } else { 0 };
        assert_eq!(
            (
                core_based_updates.update_masternode_list,
                core_based_updates
                    .masternode_updates
                    .update_operator_identity
            ),
            (update_masternode_list, 0),
            "protocol version {protocol_version}"
        );
    }
}
