mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::block_credit_mints::BlockCreditMints;
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
    /// transitions, and the epoch Core block rewards on an epoch change) as credit inflows
    /// the net daily withdrawal limit adds to its daily maximum, so money that entered
    /// Platform within the window may leave again without consuming the withdrawal budget of
    /// other users. Asset lock credits are dated by the Core block that mined each asset lock,
    /// the way Core's own unlock limit counts them; one Core has not mined at or below the
    /// block's chain locked height waits as pending. The other mints are dated by the block.
    ///
    /// Runs as a system event once per block, so nobody pays fees for the write; a block that
    /// minted nothing writes nothing.
    ///
    /// # Parameters
    ///
    /// * `state_transition_mints`: The credits the block's state transitions minted, per asset
    ///   lock.
    /// * `block_fee_mints`: The credits the block's fee processing minted (epoch Core rewards).
    /// * `block_info`: The block being executed; its time dates the other mints, and its Core
    ///   chain locked height bounds which Core blocks count as mined.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the inflows are recorded, or at once when nothing was minted or the
    ///   protocol version has no credit inflows (the method version is `None`).
    /// * `Err(Error)` when the method version (or a Drive method it calls) is unknown or not
    ///   active, Core cannot be asked, or a write fails.
    pub(in crate::execution) fn record_credit_inflows_for_withdrawals(
        &self,
        state_transition_mints: &BlockCreditMints,
        block_fee_mints: Credits,
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
                state_transition_mints,
                block_fee_mints,
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
