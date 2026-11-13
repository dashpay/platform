use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

mod v0;

impl Drive {
    /// Fetches a contract's history as [`Drive::fetch_contract_with_history`] does, each
    /// revision serialized as the contract it holds serializes at `platform_version`
    /// (`contract_serialization_version`), the bytes a query answers with, without parsing it:
    /// a revision stored in schema form would build every document type, its patterns
    /// compiled, each time it is read, and the contract cache holds only a contract's latest
    /// revision.
    ///
    /// # Returns
    ///
    /// The revisions by the time, in milliseconds, they were stored at.
    ///
    /// # Errors
    ///
    /// The errors of [`Drive::fetch_contract_with_history`], except that a revision whose
    /// schemas do not parse at `platform_version` is returned rather than refused.
    pub fn fetch_serialized_contract_history(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        start_at_date: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<u64, Vec<u8>>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .get
            .fetch_serialized_contract_history
        {
            0 => self.fetch_serialized_contract_history_v0(
                contract_id,
                transaction,
                start_at_date,
                limit,
                offset,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_serialized_contract_history".to_string(),
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
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::platform_value::platform_value;
    use dpp::serialization::PlatformSerializableWithPlatformVersion;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use grovedb::query_result_type::QueryResultType;
    use grovedb::Element;

    /// A revision stored before protocol version 15 is in schema form, one stored from it
    /// compiled: both answer as the contract read and serialized again does.
    #[test]
    fn should_serialize_each_revision_as_the_contract_it_holds() {
        let latest = PlatformVersion::latest();
        let before_compiled = PlatformVersion::get(14).expect("protocol version 14 must exist");
        let drive = setup_drive_with_initial_state_structure(None);
        let mut contract =
            get_data_contract_fixture(None, 0, latest.protocol_version).data_contract_owned();
        contract.config_mut().set_keeps_history(true);
        contract.config_mut().set_readonly(false);
        let block_at = |time_ms| BlockInfo {
            time_ms,
            ..Default::default()
        };
        drive
            .apply_contract(&contract, block_at(1000), true, None, None, before_compiled)
            .expect("expected to apply the contract");

        contract
            .set_document_schema(
                "niceDocument",
                platform_value!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "position": 0},
                        "newProp": {"type": "integer", "minimum": 0, "position": 1}
                    },
                    "required": ["$createdAt"],
                    "additionalProperties": false
                }),
                true,
                &mut vec![],
                latest,
            )
            .expect("expected to set the document schema");
        contract.increment_version();
        drive
            .apply_contract(&contract, block_at(2000), true, None, None, latest)
            .expect("expected to apply the update");

        let id = contract.id().to_buffer();
        let (stored, _) = drive
            .grove_get_path_query(
                &Drive::fetch_contract_history_query(id, 0, Some(10), None)
                    .expect("expected a history query"),
                None,
                QueryResultType::QueryElementResultType,
                &mut vec![],
                &latest.drive,
            )
            .expect("expected to read the stored revisions");
        // A contract's bytes open with its format version
        let stored_formats: Vec<u8> = stored
            .to_elements()
            .into_iter()
            .map(|element| match element {
                Element::Item(bytes, _) => bytes[0],
                _ => panic!("expected a stored revision"),
            })
            .collect();
        assert_eq!(
            stored_formats,
            vec![2, 1],
            "newest first: compiled, then schema form"
        );

        let serialized = drive
            .fetch_serialized_contract_history(id, None, 0, Some(10), None, latest)
            .expect("expected to fetch the serialized history");
        let read = drive
            .fetch_contract_with_history(id, None, 0, Some(10), None, latest)
            .expect("expected to fetch the history");

        assert_eq!(serialized.len(), 2);
        assert!(serialized.keys().eq(read.keys()));
        for ((time, bytes), contract) in serialized.iter().zip(read.values()) {
            assert_eq!(
                bytes,
                &contract
                    .serialize_to_bytes_with_platform_version(latest)
                    .expect("expected to serialize"),
                "the revision stored at {time}"
            );
        }
    }
}
