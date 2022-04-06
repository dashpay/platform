use crate::{
    errors::ProtocolError, mocks::JsonSchemaValidatorLike, state_repository::StateRepositoryLike,
};
use async_trait::async_trait;

#[async_trait]
pub trait DashPlatformProtocolLike<SR, SV> {
    /// Initializes the internal state of Dash Platform Protocol
    async fn initialize() -> Result<(), ProtocolError>;

    /// Returns the JSON schema validator which is used to validate the
    /// correctness of JSON data input
    fn get_json_schema_validator() -> SV
    where
        SV: JsonSchemaValidatorLike;

    /// Returns the State Repository instance. State Repository is handling
    /// IO communication necessary to verify the Platform Protocol's structures
    fn get_state_repository() -> SR
    where
        SR: StateRepositoryLike;

    /// Returns the version of the Dash Platform Protocol
    fn get_protocol_version() -> u32;

    /// Sets the version of the Dash Platform Protocol
    fn set_protocol_version(&mut self, version: u32);
}
