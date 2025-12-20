use crate::version::drive_abci_versions::drive_abci_checkpoint_parameters::DriveAbciCheckpointParameters;

pub const DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1: DriveAbciCheckpointParameters =
    DriveAbciCheckpointParameters {
        frequency_seconds: 600, // 10 minutes
        num_checkpoints: 3,     //
    };
