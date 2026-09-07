mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Records the credits this block minted into Platform (asset locks funding state
    /// transitions, and the epoch Core block rewards on an epoch change) as a credit inflow
    /// the net daily withdrawal limit adds to its daily maximum, so money that entered
    /// Platform within the window may leave again without consuming the withdrawal budget of
    /// other users.
    ///
    /// Runs as a system event once per block, so nobody pays fees for the write; a block that
    /// minted nothing writes nothing.
    pub(in crate::execution) fn record_credit_inflows_for_withdrawals(
        &self,
        credit_mints: Credits,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .record_credit_inflows_for_withdrawals
        {
            None => Ok(()),
            Some(0) => self.record_credit_inflows_for_withdrawals_v0(
                credit_mints,
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "record_credit_inflows_for_withdrawals".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
