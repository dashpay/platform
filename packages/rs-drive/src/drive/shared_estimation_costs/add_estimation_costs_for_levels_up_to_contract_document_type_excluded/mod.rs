mod v0;

use crate::drive::Drive;

use dpp::data_contract::DataContract;
use dpp::version::drive_versions::DriveVersion;

use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerInformation;

use crate::error::drive::DriveError;
use crate::error::Error;
use std::collections::HashMap;

impl Drive {
    /// This function calls the versioned `add_estimation_costs_for_levels_up_to_contract_document_type_excluded`
    /// function based on the version provided in the `DriveVersion` parameter. It panics if the
    /// version doesn't match any existing versioned functions.
    ///
    /// # Parameters
    /// - `contract`: The `DataContract` object to process.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a `HashMap` that holds the estimated layer information.
    /// - `drive_version`: A reference to the `DriveVersion` object that specifies the version of the function to call.
    pub(in crate::drive) fn add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
        contract: &DataContract,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .estimated_costs
            .add_estimation_costs_for_levels_up_to_contract_document_type_excluded
        {
            0 => Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded_v0(
                contract,
                estimated_costs_only_with_layer_info,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_levels_up_to_contract_document_type_excluded"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
