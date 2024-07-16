use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::DataContract;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

mod v0;

impl Drive {
    /// Fetches a contract along with its history.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A 32-byte array representing the unique identifier of the contract.
    ///
    /// * `transaction` - A transaction that requests the contract.
    ///
    /// * `start_at_date` - A `u64` representing the timestamp in Unix Epoch format from which to
    /// start fetching the contract's history.
    ///
    /// * `limit` - An `Option<u16>` that sets the maximum number of contract history entries
    /// to return. If `None`, the limit is set to 10. Should be between 1 and 10.
    ///
    /// * `offset` - An `Option<u16>` that sets the number of contract history entries to skip
    /// before starting to return them. If `None`, no entries are skipped.
    ///
    /// * `drive_version` - The version of the drive used to select the correct method version.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<u64,DataContract>, Error>` - A `Result` type, where `Ok` variant contains
    /// a `BTreeMap` with Unix timestamp as the key and contract as the value, representing
    /// the contract's history. The `Err` variant contains an `Error` in case of a failure.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` in the following situations:
    ///
    /// * If the drive version does not match any of the implemented method versions.
    ///
    /// * If any of the parameters are invalid for querying contract history.
    ///
    /// * If the contract cannot be deserialized due to protocol errors.
    ///
    /// * If the queried contract path does not refer to a contract element.
    pub fn fetch_contract_with_history(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        start_at_date: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<u64, DataContract>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .get
            .fetch_contract_with_history
        {
            0 => self.fetch_contract_with_history_v0(
                contract_id,
                transaction,
                start_at_date,
                limit,
                offset,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_with_history".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {

    use super::*;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::{platform_value, ValueMapHelper};
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;

    struct TestData {
        data_contract: DataContract,
        drive: Drive,
    }

    fn apply_contract(drive: &Drive, data_contract: &DataContract, block_info: BlockInfo) {
        let platform_version = PlatformVersion::latest();
        drive
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

    fn insert_n_contract_updates(
        data_contract: &DataContract,
        drive: &Drive,
        n: u64,
        platform_version: &PlatformVersion,
    ) {
        let mut updated_document = platform_value!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0
                },
                "newProp": {
                    "type": "integer",
                    "minimum": 0,
                    "position": 1
                }
            },
            "required": [
            "$createdAt"
            ],
            "additionalProperties": false
        });

        let mut data_contract = data_contract.clone();
        for i in 0..n {
            updated_document
                .to_map_mut()
                .expect("document to be an object")
                .get_key_mut("properties")
                .expect("properties to be present")
                .to_map_mut()
                .expect("properties to be an object")
                .insert_string_key_value(
                    format!("newProp{}", i),
                    platform_value!({"type": "integer", "minimum": 0, "position": i + 2}),
                );

            data_contract
                .set_document_schema(
                    "niceDocument",
                    updated_document.clone(),
                    true,
                    &mut vec![],
                    platform_version,
                )
                .expect("to be able to set document schema");

            data_contract.increment_version();

            apply_contract(
                drive,
                &data_contract,
                BlockInfo {
                    time_ms: 1000 * (i + 2),
                    height: 100 + i,
                    core_height: (10 + i) as u32,
                    epoch: Default::default(),
                },
            );
        }
    }

    /// Sets up history test with a given number of updates.
    ///
    /// # Arguments
    /// * `data_contract` - The data contract.
    /// * `drive` - The drive instance.
    /// * `n` - Number of updates.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    /// * `DataContract` - The data contract.
    pub fn setup_history_test_with_n_updates(
        mut data_contract: DataContract,
        drive: &Drive,
        n: u64,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        data_contract.config_mut().set_keeps_history(true);
        data_contract.config_mut().set_readonly(false);

        let original_data_contract = data_contract.clone();

        apply_contract(
            drive,
            &data_contract,
            BlockInfo {
                time_ms: 1000,
                height: 100,
                core_height: 10,
                epoch: Default::default(),
            },
        );

        insert_n_contract_updates(&data_contract, drive, n, platform_version);

        original_data_contract
    }

    /// Asserts that a property exists in the data contract.
    ///
    /// # Arguments
    /// * `data_contract` - The data contract.
    /// * `property` - The property to check.
    pub fn assert_property_exists(data_contract: &DataContract, property: &str) {
        let document_schema = data_contract
            .document_type_for_name("niceDocument")
            .expect("document should exist")
            .schema_owned();

        let document_schema_map = document_schema.to_map().expect("to be an object");

        let properties = document_schema_map
            .get_key("properties")
            .expect("to have properties")
            .to_map()
            .expect("properties to be an object");

        let property_keys = properties
            .iter()
            .map(|(key, _)| key.to_string())
            .collect::<Vec<String>>();

        assert!(
            properties.get_optional_key(property).is_some(),
            "expect property {} to exist. Instead found properties {:?}",
            property,
            property_keys
        );
    }

    fn setup_test() -> TestData {
        let data_contract =
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
                .data_contract_owned();

        TestData {
            data_contract,
            drive: setup_drive_with_initial_state_structure(),
        }
    }

    /// Tests fetching the 10 latest contracts without offset, limit, and start date set to 0.
    #[test]
    pub fn should_fetch_10_latest_contract_without_offset_and_limit_and_start_date_0() {
        let test_case = TestCase {
            total_updates_to_apply: 20,
            start_at_date: 0,
            limit: None,
            offset: None,
            expected_length: 10,
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            //DataContract created at 1000, 20 updates applied. The last update is at 21000
            // The 5th update from the latest update is 21000 - 10000 = 11000, plus since
            // the latest update is included into result, the expected oldest update date
            // is 12000.
            expected_oldest_update_date_in_result_ms: 12000,
            // 10th oldest update after 20 is 10.
            expected_oldest_update_index_in_result: 10,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    /// Test to ensure contract history can be fetched with a specified limit but without an offset.
    #[test]
    pub fn should_fetch_with_limit_without_offset() {
        let test_case = TestCase {
            total_updates_to_apply: 20,
            start_at_date: 0,
            limit: Some(5),
            offset: None,
            expected_length: 5,
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 17000,
            expected_oldest_update_index_in_result: 15,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    /// Test to ensure contract history can be fetched without a specified limit but with an offset.
    #[test]
    pub fn should_fetch_without_limit_with_offset() {
        let test_case = TestCase {
            total_updates_to_apply: 20,
            start_at_date: 0,
            limit: None,
            offset: Some(5),
            expected_length: 10,
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            // Same as test case above, but with offset 5
            expected_oldest_update_date_in_result_ms: 7000,
            expected_oldest_update_index_in_result: 5,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    /// Test to ensure contract history can be fetched with both a specified limit and offset.
    #[test]
    pub fn should_fetch_with_limit_with_offset() {
        let test_case = TestCase {
            total_updates_to_apply: 20,
            start_at_date: 0,
            limit: Some(5),
            offset: Some(5),
            expected_length: 5,
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 12000,
            expected_oldest_update_index_in_result: 10,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    /// Test to ensure contract history can be fetched with a non-zero start date.
    #[test]
    pub fn should_fetch_with_non_zero_start_date() {
        let test_case = TestCase {
            total_updates_to_apply: 20,
            start_at_date: 5000,
            limit: None,
            offset: None,
            expected_length: 10,
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 12000,
            expected_oldest_update_index_in_result: 10,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    /// Test that the function should fail when the limit is higher than the acceptable maximum value of 10.
    #[test]
    pub fn should_fail_with_limit_higher_than_10() {
        let test_case = TestCase {
            total_updates_to_apply: 20,
            start_at_date: 5000,
            limit: Some(11),
            offset: None,
            expected_length: 0,
            expected_error: Some(Error::Drive(DriveError::InvalidContractHistoryFetchLimit(
                11,
            ))),
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 0,
            expected_oldest_update_index_in_result: 0,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    /// Test that the function should fail when the limit is set to a value smaller than the minimum acceptable value of 1.
    #[test]
    pub fn should_fail_with_limit_smaller_than_1() {
        let test_case = TestCase {
            total_updates_to_apply: 20,
            start_at_date: 5000,
            limit: Some(0),
            offset: None,
            expected_length: 0,
            expected_error: Some(Error::Drive(DriveError::InvalidContractHistoryFetchLimit(
                0,
            ))),
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 0,
            expected_oldest_update_index_in_result: 0,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    /// Test that when querying with a start date after the latest update, the function should return an empty result set.
    #[test]
    pub fn should_fetch_empty_with_start_date_after_latest_update() {
        let test_case = TestCase {
            total_updates_to_apply: 20,
            start_at_date: 21001,
            limit: None,
            offset: None,
            expected_length: 0,
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 0,
            expected_oldest_update_index_in_result: 0,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    /// Test that querying with a non-existent contract ID should result in an empty return set.
    #[test]
    pub fn should_return_empty_result_with_non_existent_contract_id() {
        let test_case = TestCase {
            total_updates_to_apply: 20,
            start_at_date: 5000,
            limit: None,
            offset: None,
            expected_length: 0,
            expected_error: None,
            query_non_existent_contract_id: true,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 0,
            expected_oldest_update_index_in_result: 0,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    /// Test that when the number of available updates is fewer than the combined sum of the limit and offset,
    /// the function should only return the oldest available updates, including the original contract.
    #[test]
    pub fn should_fetch_only_oldest_updates_with_offset_regardless_of_limit_when_not_enough_updates(
    ) {
        let test_case = TestCase {
            total_updates_to_apply: 15,
            start_at_date: 0,
            limit: Some(10),
            offset: Some(10),
            // 5 updates and the original contract
            expected_length: 6,
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            // The same as created date, since we only have 5 updates with such offset
            expected_oldest_update_date_in_result_ms: 1000,
            expected_oldest_update_index_in_result: 0,
            expect_result_to_include_original_contract: true,
        };

        run_single_test_case(test_case);
    }

    /// Test that when the offset is so large that it exceeds the number of total updates, the function should return an empty result set.
    #[test]
    pub fn should_fetch_empty_history_when_offset_is_so_large_that_no_updates_can_be_fetched() {
        let test_case = TestCase {
            total_updates_to_apply: 15,
            start_at_date: 0,
            limit: Some(10),
            offset: Some(20),
            // With offset being larger than total updates, we should offset - total_updates
            // results, even if limit is set to 10
            expected_length: 0,
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 0,
            expected_oldest_update_index_in_result: 0,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    /// Test that when the limit is set equal to the total number of updates, the function should fetch and return all of them.
    #[test]
    pub fn should_fetch_with_limit_equals_total_updates() {
        let test_case = TestCase {
            total_updates_to_apply: 10,
            start_at_date: 0,
            limit: Some(10), // limit equals to total updates
            offset: None,
            expected_length: 10, // still should return 10 due to the constraint of maximum 10 results
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 2000,
            expected_oldest_update_index_in_result: 0,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    /// Test that when the set limit exceeds the total number of available updates, the function should only fetch the latest updates and the original contract.
    #[test]
    pub fn should_fetch_only_latest_updates_if_updates_count_lower_than_the_limit() {
        let test_case = TestCase {
            total_updates_to_apply: 7,
            start_at_date: 0,
            limit: Some(10), // limit larger than total updates
            offset: None,
            expected_length: 8,
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 1000,
            expected_oldest_update_index_in_result: 0,
            expect_result_to_include_original_contract: true,
        };

        run_single_test_case(test_case);
    }

    /// Test to verify that the system correctly handles a scenario when no updates have been applied.
    #[test]
    pub fn should_handle_when_no_updates_at_all() {
        let test_case = TestCase {
            total_updates_to_apply: 0,
            start_at_date: 0,
            limit: None,
            offset: None,
            expected_length: 1,
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 1000,
            expected_oldest_update_index_in_result: 0,
            expect_result_to_include_original_contract: true,
        };

        run_single_test_case(test_case);
    }

    /// Test to verify the system returns an empty history when querying with a future start date.
    #[test]
    pub fn should_fetch_empty_when_start_date_is_in_future() {
        let test_case = TestCase {
            total_updates_to_apply: 10,
            start_at_date: 20000, // future date
            limit: None,
            offset: None,
            expected_length: 0,
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 0,
            expected_oldest_update_index_in_result: 0,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    /// Test to validate fetching behavior when the start date matches the date of the latest update.
    #[test]
    pub fn should_fetch_when_start_date_is_same_as_latest_update() {
        let test_case = TestCase {
            total_updates_to_apply: 10,
            // TODO: important! This date is exclusive, that's why we can't query
            //  with the same date as the latest update. Check if this is the correct
            //  behavior
            start_at_date: 10999,
            limit: None,
            offset: None,
            expected_length: 1,
            expected_error: None,
            query_non_existent_contract_id: false,
            contract_created_date_ms: 1000,
            update_period_interval_ms: 1000,
            expected_oldest_update_date_in_result_ms: 11000,
            expected_oldest_update_index_in_result: 9,
            expect_result_to_include_original_contract: false,
        };

        run_single_test_case(test_case);
    }

    struct TestCase {
        // Test set up parameters
        total_updates_to_apply: usize,
        contract_created_date_ms: u64,
        update_period_interval_ms: u64,

        // The query parameters
        start_at_date: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        query_non_existent_contract_id: bool,

        // Expected outcomes
        expected_length: usize,
        expected_error: Option<Error>,
        expected_oldest_update_date_in_result_ms: u64,
        // The index of the oldest update in the result. So if we expect the oldest result
        // to be 10th update, then this value should be 9, because the index starts from 0
        // and not 1. It is used to generate property names in the updated contract, so we
        // can verify that the result is correct.
        expected_oldest_update_index_in_result: u64,

        expect_result_to_include_original_contract: bool,
    }

    fn run_single_test_case(test_case: TestCase) {
        let TestData {
            data_contract,
            drive,
        } = setup_test();

        let platform_version = PlatformVersion::latest();

        let contract_id = if test_case.query_non_existent_contract_id {
            [0u8; 32]
        } else {
            *data_contract.id_ref().as_bytes()
        };
        let original_data_contract = setup_history_test_with_n_updates(
            data_contract,
            &drive,
            test_case.total_updates_to_apply as u64,
            platform_version,
        );

        let contract_history_result = drive.fetch_contract_with_history(
            contract_id,
            None,
            test_case.start_at_date,
            test_case.limit,
            test_case.offset,
            platform_version,
        );

        match &test_case.expected_error {
            Some(expected_error) => {
                assert!(contract_history_result.is_err());
                // Error doesn't implement PartialEq, so we have to compare the strings
                assert_eq!(
                    contract_history_result.unwrap_err().to_string(),
                    expected_error.to_string()
                );
            }
            None => {
                assert!(contract_history_result.is_ok());
                let contract_history = contract_history_result.unwrap();
                assert_eq!(contract_history.len(), test_case.expected_length);

                for (i, (key, contract)) in contract_history.iter().enumerate() {
                    if i == 0 && test_case.expect_result_to_include_original_contract {
                        // TODO: this doesn't work because when we deserialize the contract
                        //  keeps_history is false for some reason!
                        assert_eq!(key, &test_case.contract_created_date_ms);
                        assert_eq!(contract, &original_data_contract);
                        continue;
                    }

                    let expected_key: u64 = test_case.expected_oldest_update_date_in_result_ms
                        + i as u64 * test_case.update_period_interval_ms;
                    assert_eq!(key, &expected_key);

                    let prop_index = if test_case.expect_result_to_include_original_contract {
                        // If we expect the result to include the original contract, then
                        // the first update will be the original contract, so we need to
                        // offset the index by 1
                        i - 1 + test_case.expected_oldest_update_index_in_result as usize
                    } else {
                        i + test_case.expected_oldest_update_index_in_result as usize
                    };

                    // When updating a contract, we add a new property to it
                    // TODO: this test actually applies incompatible updates to the contract
                    //  because we don't validate the contract in the apply function
                    assert_property_exists(contract, format!("newProp{}", prop_index).as_str());
                }
            }
        }
    }
}
