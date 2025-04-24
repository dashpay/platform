use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::identity::TimestampMillis;
use dpp::prelude::CoreBlockHeight;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

mod common;
#[cfg(any(create_sdk_test_data, test))]
mod test;
pub mod v0;
pub mod v1;

impl<C> Platform<C> {
    /// Creates trees and populates them with necessary identities, contracts and documents
    pub fn create_genesis_state(
        &self,
        genesis_core_height: CoreBlockHeight,
        genesis_time: TimestampMillis,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .initialization
            .create_genesis_state
        {
            0 => self.create_genesis_state_v0(
                genesis_core_height,
                genesis_time,
                transaction,
                platform_version,
            ),
            1 => self.create_genesis_state_v1(
                genesis_core_height,
                genesis_time,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "create_genesis_state".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }?;

        // We aren't using features here because we don't want this thing to be activated when we are running tests
        // with --all-features flag
        #[cfg(create_sdk_test_data)]
        {
            let block_info = dpp::block::block_info::BlockInfo::default_with_time(genesis_time);
            self.create_sdk_test_data(&block_info, transaction, platform_version)?;
        }

        Ok(())
    }
}
