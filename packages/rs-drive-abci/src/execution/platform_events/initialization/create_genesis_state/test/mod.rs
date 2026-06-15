use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

mod addresses;
// Shielded-pool seeding (incl. snapshot bake/apply tests) is gated separately
// from the base SDK fixtures — see the comment in `create_sdk_test_data`.
#[cfg(any(feature = "shielded_test_data", test))]
mod shielded;
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
        // Permit only non-production networks. Mainnet/Testnet must never
        // carry SDK test fixtures (random identities, seeded shielded pool,
        // pre-baked snapshot anchor that isn't a valid spend anchor).
        // Regtest = local dashmate; Devnet = developer test networks
        // (issue #3714 stress-test target).
        match self.config.network {
            Network::Regtest | Network::Devnet => {}
            _ => {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "create_sdk_test_data must be called only on local or devnet networks",
                )));
            }
        }

        self.create_data_for_group_token_queries(block_info, transaction, platform_version)?;
        self.create_data_for_token_direct_prices(block_info, transaction, platform_version)?;
        self.create_data_for_addresses(block_info, transaction, platform_version)?;
        // Shielded-pool seeding is gated separately so SDK_TEST_DATA-only
        // builds get the base fixtures (addresses, tokens) without paying the
        // shielded-seed setup cost or shipping the bake/apply code path. Enable
        // by building with `--features=shielded_test_data` — the Dockerfile
        // passes that flag when `SHIELDED_TEST_DATA=true`.
        #[cfg(feature = "shielded_test_data")]
        self.create_data_for_shielded_pool(block_info, transaction, platform_version)?;

        Ok(())
    }
}
