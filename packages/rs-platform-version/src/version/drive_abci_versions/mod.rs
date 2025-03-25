pub mod drive_abci_method_versions;
pub mod drive_abci_query_versions;
pub mod drive_abci_state_sync_versions;
pub mod drive_abci_structure_versions;
pub mod drive_abci_validation_versions;
pub mod drive_abci_withdrawal_constants;

use drive_abci_method_versions::DriveAbciMethodVersions;
use drive_abci_query_versions::DriveAbciQueryVersions;
use drive_abci_state_sync_versions::DriveAbciStateSyncVersions;
use drive_abci_structure_versions::DriveAbciStructureVersions;
use drive_abci_validation_versions::DriveAbciValidationVersions;
use drive_abci_withdrawal_constants::DriveAbciWithdrawalConstants;

#[derive(Clone, Debug, Default)]
pub struct DriveAbciVersion {
    pub structs: DriveAbciStructureVersions,
    pub methods: DriveAbciMethodVersions,
    pub validation_and_processing: DriveAbciValidationVersions,
    pub withdrawal_constants: DriveAbciWithdrawalConstants,
    pub query: DriveAbciQueryVersions,
    pub state_sync: DriveAbciStateSyncVersions,
}
