use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dpp::check_validation_result_with_data;

use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_request::GetProtocolVersionUpgradeStateRequestV0;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_response::get_protocol_version_upgrade_state_response_v0::{VersionEntry, Versions};
use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_response::{get_protocol_version_upgrade_state_response_v0, GetProtocolVersionUpgradeStateResponseV0};

impl<C> Platform<C> {
    pub(super) fn query_version_upgrade_state_v0(
        &self,
        GetProtocolVersionUpgradeStateRequestV0 { prove }: GetProtocolVersionUpgradeStateRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetProtocolVersionUpgradeStateResponseV0>, Error> {
        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .fetch_proved_versions_with_counter(None, &platform_version.drive));

            let (metadata, proof) = self.response_metadata_and_proof_v0(proof);

            GetProtocolVersionUpgradeStateResponseV0 {
                result: Some(get_protocol_version_upgrade_state_response_v0::Result::Proof(proof)),
                metadata: Some(metadata),
            }
        } else {
            let protocol_versions_counter =
                self.drive.cache.protocol_versions_counter.read().unwrap();

            let versions = protocol_versions_counter
                .global_cache
                .iter()
                .map(|(protocol_version, count)| VersionEntry {
                    version_number: *protocol_version,
                    vote_count: *count as u32,
                })
                .collect();

            drop(protocol_versions_counter);

            GetProtocolVersionUpgradeStateResponseV0 {
                result: Some(
                    get_protocol_version_upgrade_state_response_v0::Result::Versions(Versions {
                        versions,
                    }),
                ),
                metadata: Some(self.response_metadata_v0()),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use drive::drive::grove_operations::BatchInsertApplyType;
    use drive::drive::object_size_info::PathKeyElementInfo;
    use drive::drive::protocol_upgrade::{
        desired_version_for_validators_path, versions_counter_path, versions_counter_path_vec,
    };
    use drive::drive::Drive;
    use drive::grovedb::{Element, GroveDb, PathQuery};
    use drive::query::{Query, QueryItem};
    use integer_encoding::VarInt;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::ops::RangeFull;

    #[test]
    fn test_query_empty_upgrade_state() {
        let (platform, version) = setup_platform();

        let request = GetProtocolVersionUpgradeStateRequestV0 { prove: false };

        let validation_result = platform
            .query_version_upgrade_state_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            validation_result.data,
            Some(GetProtocolVersionUpgradeStateResponseV0 {
                result: Some(get_protocol_version_upgrade_state_response_v0::Result::Versions(versions)),
                metadata: Some(_)
            }) if versions.versions.is_empty()
        ));
    }

    #[test]
    fn test_query_upgrade_state() {
        let (platform, version) = setup_platform();

        let mut rand = StdRng::seed_from_u64(10);

        let drive = &platform.drive;

        let mut version_counter = drive.cache.protocol_versions_counter.write().unwrap();

        let transaction = drive.grove.start_transaction();

        version_counter
            .load_if_needed(drive, Some(&transaction), &version.drive)
            .expect("expected to load version counter");

        let path = desired_version_for_validators_path();
        let version_bytes = version.protocol_version.encode_var_vec();
        let version_element = Element::new_item(version_bytes.clone());

        let validator_pro_tx_hash: [u8; 32] = rand.gen();

        let mut operations = vec![];
        drive
            .batch_insert_if_changed_value(
                PathKeyElementInfo::PathFixedSizeKeyRefElement((
                    path,
                    validator_pro_tx_hash.as_slice(),
                    version_element,
                )),
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&transaction),
                &mut operations,
                &version.drive,
            )
            .expect("expected batch to insert");

        let mut version_count = version_counter
            .get(&version.protocol_version)
            .cloned()
            .unwrap_or_default();

        version_count += 1;

        version_counter.set_block_cache_version_count(version.protocol_version, version_count); // push to block_cache

        drive
            .batch_insert(
                PathKeyElementInfo::PathFixedSizeKeyRefElement((
                    versions_counter_path(),
                    version_bytes.as_slice(),
                    Element::new_item(version_count.encode_var_vec()),
                )),
                &mut operations,
                &version.drive,
            )
            .expect("expected batch to insert");

        drive
            .apply_batch_low_level_drive_operations(
                None,
                Some(&transaction),
                operations,
                &mut vec![],
                &version.drive,
            )
            .expect("expected to apply operations");

        drive
            .commit_transaction(transaction, &version.drive)
            .expect("expected to commit");

        version_counter.merge_block_cache();
        drop(version_counter);

        let request = GetProtocolVersionUpgradeStateRequestV0 { prove: false };

        let validation_result = platform
            .query_version_upgrade_state_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            validation_result.data,
            Some(GetProtocolVersionUpgradeStateResponseV0 {
                result: Some(get_protocol_version_upgrade_state_response_v0::Result::Versions(versions)),
                metadata: Some(_)
            }) if versions.versions.len() == 1
        ));
    }

    #[test]
    fn test_prove_empty_upgrade_state() {
        let (platform, version) = setup_platform();

        let request = GetProtocolVersionUpgradeStateRequestV0 { prove: true };

        let validation_result = platform
            .query_version_upgrade_state_v0(request, version)
            .expect("expected query to succeed");

        let path_query = PathQuery::new_unsized(
            versions_counter_path_vec(),
            Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
        );

        // we just started chain, there should be no versions

        if let Some(GetProtocolVersionUpgradeStateResponseV0 {
            result: Some(get_protocol_version_upgrade_state_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) = validation_result.data
        {
            let elements = GroveDb::verify_query(proof.grovedb_proof.as_slice(), &path_query)
                .expect("expected to be able to verify query")
                .1;

            assert!(elements.is_empty());
        } else {
            panic!("expected a proof");
        }
    }

    #[test]
    fn test_prove_upgrade_state() {
        let (platform, version) = setup_platform();

        let mut rand = StdRng::seed_from_u64(10);

        let drive = &platform.drive;

        let version_counter = &mut drive.cache.protocol_versions_counter.write().unwrap();

        let transaction = drive.grove.start_transaction();

        version_counter
            .load_if_needed(drive, Some(&transaction), &version.drive)
            .expect("expected to load version counter");

        let path = desired_version_for_validators_path();
        let version_bytes = version.protocol_version.encode_var_vec();
        let version_element = Element::new_item(version_bytes.clone());

        let validator_pro_tx_hash: [u8; 32] = rand.gen();

        let mut operations = vec![];
        drive
            .batch_insert_if_changed_value(
                PathKeyElementInfo::PathFixedSizeKeyRefElement((
                    path,
                    validator_pro_tx_hash.as_slice(),
                    version_element,
                )),
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&transaction),
                &mut operations,
                &version.drive,
            )
            .expect("expected batch to insert");

        let mut version_count = version_counter
            .get(&version.protocol_version)
            .cloned()
            .unwrap_or_default();

        version_count += 1;

        version_counter.set_block_cache_version_count(version.protocol_version, version_count); // push to block_cache

        drive
            .batch_insert(
                PathKeyElementInfo::PathFixedSizeKeyRefElement((
                    versions_counter_path(),
                    version_bytes.as_slice(),
                    Element::new_item(version_count.encode_var_vec()),
                )),
                &mut operations,
                &version.drive,
            )
            .expect("expected batch to insert");

        drive
            .apply_batch_low_level_drive_operations(
                None,
                Some(&transaction),
                operations,
                &mut vec![],
                &version.drive,
            )
            .expect("expected to apply operations");

        drive
            .commit_transaction(transaction, &version.drive)
            .expect("expected to commit");

        version_counter.merge_block_cache();

        let request = GetProtocolVersionUpgradeStateRequestV0 { prove: true };

        let validation_result = platform
            .query_version_upgrade_state_v0(request, version)
            .expect("expected query to succeed");

        let Some(GetProtocolVersionUpgradeStateResponseV0 {
            result: Some(get_protocol_version_upgrade_state_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) = validation_result.data
        else {
            panic!("expected a proof");
        };

        let path_query = PathQuery::new_unsized(
            versions_counter_path_vec(),
            Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
        );

        let elements = GroveDb::verify_query(proof.grovedb_proof.as_slice(), &path_query)
            .expect("expected to be able to verify query")
            .1;

        // we just started chain, there should be no versions

        assert_eq!(elements.len(), 1);

        let (_, _, element) = elements.first().unwrap();

        assert!(element.is_some());

        let element = element.clone().unwrap();

        let count_bytes = element.as_item_bytes().expect("expected item bytes");

        let count = u16::decode_var(count_bytes)
            .expect("expected to decode var int")
            .0;

        assert_eq!(count, 1);

        let upgrade = Drive::verify_upgrade_state(proof.grovedb_proof.as_slice(), version)
            .expect("expected to verify the upgrade counts")
            .1;

        assert_eq!(upgrade.len(), 1);
        assert_eq!(upgrade.get(&1), Some(1).as_ref());
    }
}
