use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

mod v0;

impl Drive {
    /// Adds the estimation costs for the insertion of a non unique public key hash reference
    pub(in crate::drive::identity::key) fn add_estimation_costs_for_insert_non_unique_public_key_hash_reference(
        public_key_hash: [u8; 20],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .identity
            .keys
            .insert_key_hash_identity_reference
            .add_estimation_costs_for_insert_non_unique_public_key_hash_reference
        {
            0 => {
                Self::add_estimation_costs_for_insert_non_unique_public_key_hash_reference_v0(
                    public_key_hash,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_insert_non_unique_public_key_hash_reference"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
