pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DriveAbciCheckpointParameters {
    pub frequency_seconds: u32,
    pub num_checkpoints: u32,
}
