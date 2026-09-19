mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

/// Estimated serialized size of a lifecycle record: the largest `u128` rollup, a wipe marker
/// with two `u64` fields and the version discriminants, all varint encoded.
pub(crate) const ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES: u32 = 40;

impl Drive {
    /// Adds the layer estimation for writes under the token contract lifecycle ledger.
    ///
    /// # Parameters
    ///
    /// * `estimated_costs_only_with_layer_info` - The layer map estimation writes into.
    /// * `drive_version` - The drive version selecting the estimation generation.
    ///
    /// # Returns
    ///
    /// * `Err(DriveError::VersionNotActive)` on a drive version without the ledger.
    pub(crate) fn add_estimation_costs_for_token_contract_lifecycles(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .token
            .lifecycle
            .add_estimation_costs_for_token_contract_lifecycles
        {
            Some(0) => {
                Self::add_estimation_costs_for_token_contract_lifecycles_v0(
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_estimation_costs_for_token_contract_lifecycles".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_token_contract_lifecycles".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::serialization::PlatformSerializable;
    use dpp::tokens::contract_lifecycle::v0::ContractTokenLifecycleV0Accessors;
    use dpp::tokens::contract_lifecycle::{ContractTokenLifecycle, ContractWipe};
    use dpp::version::PlatformVersion;

    #[test]
    fn should_cover_the_largest_record() {
        let platform_version = PlatformVersion::latest();
        let mut record =
            ContractTokenLifecycle::new(u128::MAX, platform_version).expect("expected a record");
        record.set_wiped(
            ContractWipe::new(u64::MAX, u64::MAX, platform_version).expect("expected a wipe"),
        );

        let bytes = record.serialize_to_bytes().expect("expected bytes");

        assert!(
            bytes.len() as u32 <= ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES,
            "the largest record is {} bytes",
            bytes.len()
        );
    }
}
