mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;
use drive::util::batch::DriveOperation;

use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    // This function is used to handle versioning and dispatch to the appropriate version of `disable_identity_keys`
    pub(in crate::execution::platform_events::core_based_updates::update_masternode_identities) fn disable_identity_keys(
        &self,
        old_masternode: &MasternodeListItem,
        block_info: &BlockInfo,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .disable_identity_keys
        {
            0 => self.disable_identity_keys_v0(
                old_masternode,
                block_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "disable_identity_keys".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
