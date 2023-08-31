use dpp::ProtocolError;
use drive::error::drive::DriveError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Drive error: {0}")]
    Drive(#[from] drive::error::Error),
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
}
