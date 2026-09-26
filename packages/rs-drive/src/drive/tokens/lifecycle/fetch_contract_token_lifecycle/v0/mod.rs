use crate::drive::tokens::lifecycle::estimated_costs::ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES;
use crate::drive::tokens::paths::token_contract_lifecycles_root_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::serialization::PlatformDeserializable;
use dpp::tokens::contract_lifecycle::ContractTokenLifecycle;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    pub(super) fn fetch_contract_token_lifecycle_v0(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractTokenLifecycle>, Error> {
        self.fetch_contract_token_lifecycle_operations_v0(
            contract_id,
            true,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_contract_token_lifecycle_operations_v0(
        &self,
        contract_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractTokenLifecycle>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES),
            }
        };

        let lifecycles_path = token_contract_lifecycles_root_path();

        match self.grove_get_raw_optional(
            (&lifecycles_path).into(),
            &contract_id,
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(bytes, _))) => Ok(Some(ContractTokenLifecycle::deserialize_from_bytes(
                bytes.as_slice(),
            )?)),
            Ok(None) => Ok(None),
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_)) => {
                Ok(None)
            }
            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "contract token lifecycle was present but was not an item",
            ))),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::tokens::paths::token_contract_lifecycles_root_path;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::Identifier;
    use dpp::tokens::contract_lifecycle::v0::ContractTokenLifecycleV0Accessors;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    #[test]
    fn should_return_none_for_a_contract_without_tokens() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let record = drive
            .fetch_contract_token_lifecycle([9u8; 32], None, platform_version)
            .expect("expected to read the ledger");

        assert_eq!(record, None);
    }

    #[test]
    fn should_return_the_record_created_with_the_token_trees() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([3u8; 32]);

        drive
            .create_token_trees(
                contract_id,
                0,
                [1u8; 32],
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to create token trees");

        let record = drive
            .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
            .expect("expected to read the ledger")
            .expect("expected a record");

        assert_eq!(record.issued_supply(), 0);
        assert!(!record.is_wiped());
    }

    #[test]
    fn should_price_the_read_without_touching_state() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut drive_operations = vec![];
        let record = drive
            .fetch_contract_token_lifecycle_operations(
                [5u8; 32],
                false,
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected an estimated read");

        assert_eq!(record, None);
        assert!(!drive_operations.is_empty());
    }

    #[test]
    fn should_reject_a_record_that_is_not_an_item() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = [7u8; 32];

        drive
            .grove
            .insert(
                &token_contract_lifecycles_root_path(),
                &contract_id,
                Element::empty_tree(),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to insert a tree");

        let result = drive.fetch_contract_token_lifecycle(contract_id, None, platform_version);

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedElementType(_)))
        ));
    }

    #[test]
    fn should_not_be_active_before_the_ledger_exists() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        let result = drive.fetch_contract_token_lifecycle([1u8; 32], None, platform_version);

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
    }
}
