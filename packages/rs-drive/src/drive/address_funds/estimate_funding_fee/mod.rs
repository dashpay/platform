mod v0;

pub use v0::AddressFundingFeeEstimate;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::platform_value::Bytes36;
use dpp::version::PlatformVersion;

impl Drive {
    /// Estimates the fee of a 0-input / 1-output address funding from a fresh
    /// asset lock, using the current shape of the state trees.
    ///
    /// The estimate builds the exact production drive operations (through the
    /// action converter, with stateful reads so the insert-vs-replace branch
    /// and element bytes come from committed state), then prices them with the
    /// server's own average-case layer models where the two data-dependent
    /// layer counts are replaced by search-path levels measured from locally
    /// generated proofs.
    ///
    /// Read-only: no transaction is opened and nothing is written. The whole
    /// estimate reads committed state (GroveDB proving does not support
    /// transactions). The result covers the GroveDB batch only — validation
    /// operations and `user_fee_increase` are the caller's concern.
    ///
    /// Fails with [`DriveError::AssetLockOutpointAlreadyPresent`] when the
    /// outpoint is already in the state: a spent or partially used lock would
    /// execute through the partial-use path, which this estimator does not
    /// model.
    pub fn estimate_address_funding_fee(
        &self,
        recipient: &PlatformAddress,
        asset_lock_outpoint: Bytes36,
        lock_credits: Credits,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<AddressFundingFeeEstimate, Error> {
        match platform_version
            .drive
            .methods
            .address_funds
            .estimate_funding_fee
        {
            0 => self.estimate_address_funding_fee_v0(
                recipient,
                asset_lock_outpoint,
                lock_credits,
                block_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "Drive::estimate_address_funding_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
