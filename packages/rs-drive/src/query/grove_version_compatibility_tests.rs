#![cfg(feature = "full")]

use crate::config::DriveConfig;
use crate::drive::Drive;
use crate::query::{
    DriveDocumentQuery, SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus,
};
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use crate::util::test_helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::document::DocumentV0Setters;
use dpp::platform_value::Value;
use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType;
use grovedb::PathQuery;
use grovedb_version::version::{v4::GROVE_V4, v5::GROVE_V5};
use std::collections::BTreeMap;

fn merged_public_queries(
    drive: &Drive,
    platform_version: &PlatformVersion,
) -> Vec<(&'static str, PathQuery)> {
    let grove_version = &platform_version.drive.grove_version;
    let identity_id = [1; 32];
    let identity_ids = [identity_id, [2; 32]];
    let key_hash = [3; 20];
    let contract_id = [4; 32];
    let single_document_query = SingleDocumentDriveQuery {
        contract_id,
        document_type_name: "note".to_string(),
        document_type_keeps_history: false,
        document_id: [5; 32],
        block_time_ms: None,
        contested_status: SingleDocumentDriveQueryContestedStatus::MaybeContested,
    };

    let mut queries = vec![
        (
            "identity revision and balance",
            Drive::revision_and_balance_path_query(identity_id, grove_version)
                .expect("revision and balance query"),
        ),
        (
            "full identity",
            Drive::full_identity_query(&identity_id, grove_version).expect("full identity query"),
        ),
        (
            "identity keys and revision",
            Drive::identity_all_keys_query(&identity_id, grove_version)
                .expect("identity keys and revision query"),
        ),
        (
            "full identities",
            Drive::full_identities_query(&identity_ids, grove_version)
                .expect("full identities query"),
        ),
        (
            "full identity and unique hash",
            Drive::full_identity_with_public_key_hash_query(key_hash, identity_id, grove_version)
                .expect("full identity and unique hash query"),
        ),
        (
            "full identity and nonunique hash",
            Drive::full_identity_with_non_unique_public_key_hash_query(
                key_hash,
                identity_id,
                None,
                grove_version,
            )
            .expect("full identity and nonunique hash query"),
        ),
        (
            "full identities and unique hashes",
            Drive::full_identities_with_keys_hashes_query(
                &identity_ids,
                &[key_hash, [6; 20]],
                grove_version,
            )
            .expect("full identities and unique hashes query"),
        ),
        (
            "identity balance and revision",
            Drive::balance_and_revision_for_identity_id_query(identity_id, grove_version),
        ),
        (
            "all identities keys without a limit",
            drive
                .fetch_identities_all_keys_query(&identity_ids, None, grove_version)
                .expect("all identities keys query"),
        ),
        (
            "current and historical contracts",
            Drive::fetch_contracts_query(&[contract_id], &[[7; 32]], platform_version)
                .expect("current and historical contracts query"),
        ),
        (
            "contract fee pots and claims",
            Drive::contract_fee_pots_query(
                contract_id,
                &[ContractFeePot::Owner, ContractFeePot::Moderators],
                grove_version,
            )
            .expect("contract fee pots query"),
        ),
        (
            "contract moderation lists",
            Drive::contract_moderation_status_query(
                contract_id,
                identity_id,
                &[
                    ContractModerationList::Banlist,
                    ContractModerationList::Suspensions,
                    ContractModerationList::Warnings,
                ],
                grove_version,
            )
            .expect("contract moderation status query"),
        ),
        (
            "token supply and aggregate balances",
            Drive::token_total_supply_and_aggregated_identity_balances_query(
                [8; 32],
                platform_version,
            )
            .expect("token supply and aggregate balances query"),
        ),
        (
            "contested and uncontested document",
            single_document_query
                .construct_path_query(platform_version)
                .expect("contested and uncontested document query"),
        ),
    ];

    queries.push((
        "full identity and nonunique hash after a cursor",
        Drive::full_identity_with_non_unique_public_key_hash_query(
            key_hash,
            identity_id,
            Some([0; 32]),
            grove_version,
        )
        .expect("full identity and nonunique hash query after a cursor"),
    ));
    queries.push((
        "all identities keys with branch limits",
        drive
            .fetch_identities_all_keys_query(&identity_ids, Some(2), grove_version)
            .expect("limited all identities keys query"),
    ));
    queries.push((
        "contested and uncontested history-keeping document",
        SingleDocumentDriveQuery {
            document_type_keeps_history: true,
            ..single_document_query
        }
        .construct_path_query(platform_version)
        .expect("history-keeping document query"),
    ));
    queries
}

#[test]
fn should_preserve_public_merged_queries_and_their_bytes_across_grove_versions() {
    let drive = setup_drive(None);
    for selected in [
        PlatformVersion::get(14).unwrap(),
        PlatformVersion::get(15).unwrap(),
    ] {
        let mut previous = selected.clone();
        previous.drive.grove_version = GROVE_V4;
        let mut current = selected.clone();
        current.drive.grove_version = GROVE_V5;
        let previous_queries = merged_public_queries(&drive, &previous);
        let current_queries = merged_public_queries(&drive, &current);
        assert_eq!(previous_queries.len(), current_queries.len());

        for ((name, previous_query), (current_name, current_query)) in
            previous_queries.into_iter().zip(current_queries)
        {
            assert_eq!(name, current_name);
            assert_eq!(
                previous_query, current_query,
                "protocol {}: {name}",
                selected.protocol_version,
            );
            let config = bincode::config::standard().with_big_endian();
            assert_eq!(
                bincode::encode_to_vec(&previous_query, config).expect("previous query bytes"),
                bincode::encode_to_vec(&current_query, config).expect("current query bytes"),
                "protocol {} query bytes: {name}",
                selected.protocol_version,
            );
        }
    }
}

#[test]
fn should_accept_explicit_zero_cursor_offsets_only_after_the_grove_upgrade() {
    for version in [
        PlatformVersion::get(14).unwrap(),
        PlatformVersion::get(15).unwrap(),
    ] {
        let drive = setup_drive_with_initial_state_structure(Some(version));
        let contract = json_document_to_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            version,
        )
        .expect("profile contract");
        drive
            .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
            .expect("registered profile contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("profile type");
        for id in [1u8, 2] {
            let mut document = json_document_to_document(
                "tests/supporting_files/contract/dashpay/profile0.json",
                Some([id; 32].into()),
                document_type,
                version,
            )
            .expect("profile document");
            document.set_id([id; 32].into());
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    true,
                    BlockInfo::default(),
                    true,
                    None,
                    version,
                    None,
                )
                .expect("stored profile document");
        }

        for cursor in ["startAt", "startAfter"] {
            let mut input = BTreeMap::from([
                (cursor.to_string(), Value::Identifier([1; 32])),
                ("limit".to_string(), Value::U16(2)),
            ]);
            let without_offset = DriveDocumentQuery::from_btree_map_value(
                input.clone(),
                &contract,
                document_type,
                &DriveConfig::default(),
                version,
            )
            .expect("cursor query without an offset");
            input.insert("offset".to_string(), Value::U16(0));
            let with_zero_offset = DriveDocumentQuery::from_btree_map_value(
                input,
                &contract,
                document_type,
                &DriveConfig::default(),
                version,
            )
            .expect("cursor query with an explicit zero offset");
            assert_eq!(with_zero_offset.offset, Some(0));

            let without_offset = without_offset
                .construct_path_query_operations(&drive, true, None, &mut vec![], version)
                .expect("merged cursor query without an offset");
            let with_zero_offset = with_zero_offset.construct_path_query_operations(
                &drive,
                true,
                None,
                &mut vec![],
                version,
            );
            if version.protocol_version == 14 {
                assert!(
                    with_zero_offset
                        .expect_err("released GroveDB refuses explicit cursor offsets")
                        .to_string()
                        .contains("can not merge pathqueries with offsets"),
                    "{cursor} preserves the released offset refusal",
                );
                continue;
            }

            let with_zero_offset = with_zero_offset.expect("zero offset is a no-op after upgrade");
            assert_eq!(with_zero_offset, without_offset, "{cursor} merged query");
            let read = |query: &PathQuery| {
                let (rows, skipped) = drive
                    .grove
                    .query_raw(
                        query,
                        false,
                        true,
                        true,
                        QueryPathKeyElementTrioResultType,
                        None,
                        &version.drive.grove_version,
                    )
                    .unwrap()
                    .expect("merged cursor query results");
                (rows.to_path_key_elements(), skipped)
            };
            let expected = read(&without_offset);
            assert!(!expected.0.is_empty(), "{cursor} reads the stored profiles");
            assert_eq!(read(&with_zero_offset), expected, "{cursor} results");
        }
    }
}
