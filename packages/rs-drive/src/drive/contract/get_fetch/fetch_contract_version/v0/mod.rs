use crate::drive::contract::paths::{contract_root_path, CONTRACT_VERSION_KEY};
use crate::drive::contract::version_item::decode_contract_version;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::TransactionArg;

impl Drive {
    /// Reads the version item under the contract's root subtree. A missing contract subtree
    /// (an id no contract has) reads as no item, like a contract stored before the item
    /// existed.
    #[inline(always)]
    pub(super) fn fetch_contract_version_v0(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<u32>, Error> {
        let contract_root_path = contract_root_path(&contract_id);

        match self.grove_get_raw_optional(
            (&contract_root_path).into(),
            &[CONTRACT_VERSION_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        ) {
            Ok(Some(Item(encoded_version, _))) => decode_contract_version(&encoded_version)
                .map(Some)
                .ok_or(Error::Drive(DriveError::CorruptedElementType(
                    "contract version item was not 4 bytes as expected",
                ))),
            Ok(Some(..)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "contract version was present but was not an item",
            ))),
            Ok(None) => Ok(None),
            Err(Error::GroveDB(error))
                if matches!(
                    *error,
                    grovedb::Error::PathParentLayerNotFound(_) | grovedb::Error::PathNotFound(_)
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_read_no_version_for_an_unknown_contract() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let version = drive
            .fetch_contract_version([9; 32], None, platform_version)
            .expect("an unknown contract reads as no version");

        assert_eq!(version, None);
    }
}
