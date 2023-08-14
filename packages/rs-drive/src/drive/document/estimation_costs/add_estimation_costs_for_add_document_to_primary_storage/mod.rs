use crate::drive::object_size_info::DocumentAndContractInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

mod v0;

impl Drive {
    pub(crate) fn add_estimation_costs_for_add_document_to_primary_storage(
        document_and_contract_info: &DocumentAndContractInfo,
        primary_key_path: [&[u8]; 5],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .estimation_costs
            .add_estimation_costs_for_add_document_to_primary_storage
        {
            0 => Self::add_estimation_costs_for_add_document_to_primary_storage_v0(
                document_and_contract_info,
                primary_key_path,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "Drive::add_estimation_costs_for_add_document_to_primary_storage_v0"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
