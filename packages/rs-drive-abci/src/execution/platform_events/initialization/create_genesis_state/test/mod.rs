use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

mod tokens;

impl<C> Platform<C> {
    /// Creates testing data for SDK functional and e2e tests
    ///
    /// This is function must be used only for testing
    /// purposes on local networks and DISABLED BY DEFAULT
    #[allow(dead_code)]
    pub(super) fn create_sdk_test_data(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if self.config.network != Network::Regtest {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "create_sdk_test_data must be called only on local network",
            )));
        }

        self.create_data_for_group_token_queries(block_info, transaction, platform_version)?;
        self.create_data_for_token_direct_prices(block_info, transaction, platform_version)?;

        Ok(())
    }
}
