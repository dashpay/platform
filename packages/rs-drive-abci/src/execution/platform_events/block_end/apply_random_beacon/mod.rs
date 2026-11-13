mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_random_requests::BlockRandomRequests;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Draws a block's random values from its random beacon (protocol version 15), once every
    /// transition of the block is accepted: it fills the `sys.random` properties of the
    /// documents the block created (`Drive::draw_random_document_properties`), then settles
    /// the block's random token transfers and mints (`Drive::settle_random_token_amounts`).
    /// The state it leaves is the one the block's precommits sign. Nobody pays: each
    /// transition paid for its execution with the placeholder or the reservation.
    ///
    /// The beacon is the threshold signature the block's prevotes combine into, which
    /// Tenderdash recovered and checked against the quorum's public key, and checks again in
    /// every commit.
    ///
    /// # Parameters
    /// - `beacon`: the block's random beacon.
    /// - `random_requests`: what the block's transitions ask the beacon for, in the order the
    ///   block executed them.
    /// - `block_info`: the block.
    /// - `transaction`: the block's transaction, holding the state its execution left.
    /// - `platform_version`: selects the method version; `None` before protocol version 15.
    ///
    /// # Returns
    /// `Ok(())` once the values are drawn; the caller reads the root hash.
    pub(crate) fn apply_random_beacon(
        &self,
        beacon: &[u8],
        random_requests: &BlockRandomRequests,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .apply_random_beacon
        {
            Some(0) => self.apply_random_beacon_v0(
                beacon,
                random_requests,
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "apply_random_beacon".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "apply_random_beacon".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
