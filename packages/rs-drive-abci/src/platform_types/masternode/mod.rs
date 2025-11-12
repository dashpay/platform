use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::masternode::v0::MasternodeV0;
use bincode::{Decode, Encode};
use dpp::dashcore_rpc::json::MasternodeListItem;
use dpp::version::{PlatformVersion, TryFromPlatformVersioned};

mod accessors;
/// Version 0
pub mod v0;

/// `Masternode` represents a masternode on the network.
#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum Masternode {
    /// Version 0
    V0(MasternodeV0),
}

impl TryFromPlatformVersioned<MasternodeListItem> for Masternode {
    type Error = Error;

    fn try_from_platform_versioned(
        value: MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version.drive_abci.structs.masternode {
            0 => Ok(Self::V0(value.into())),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "Masternode::try_from_platform_versioned(MasternodeListItem)".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl From<Masternode> for MasternodeListItem {
    fn from(value: Masternode) -> Self {
        match value {
            Masternode::V0(v0) => v0.into(),
        }
    }
}
