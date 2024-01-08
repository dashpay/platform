use crate::config::PlatformConfig;
use crate::error::Error;
use crate::execution::platform_events::core_chain_lock::verify_chain_lock::VerifyChainLockResult;
use dpp::dashcore::ChainLock;
use dpp::version::PlatformVersion;

use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;

use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn verify_chain_lock_v0(
        &self,
        platform_state: &PlatformState,
        chain_lock: &ChainLock,
        make_sure_core_is_synced: bool,
        platform_version: &PlatformVersion,
    ) -> Result<VerifyChainLockResult, Error> {
        // first we try to verify the chain lock locally
        match self.verify_chain_lock_locally(platform_state, chain_lock, platform_version) {
            Ok(Some(valid)) => {
                if valid && make_sure_core_is_synced {
                    //ToDO (important), separate errors from core, connection Errors -> Err, others should be part of the result
                    let submission_result =
                        self.make_sure_core_is_synced_to_chain_lock(chain_lock, platform_version)?;

                    Ok(VerifyChainLockResult {
                        chain_lock_signature_is_deserializable: true,
                        found_valid_locally: Some(valid),
                        submitted: Some(submission_result),
                        found_valid_by_core: None,
                    })
                } else {
                    Ok(VerifyChainLockResult {
                        chain_lock_signature_is_deserializable: true,
                        found_valid_locally: Some(valid),
                        submitted: None,
                        found_valid_by_core: None,
                    })
                }
            }
            Ok(None) => {
                let verified = self.verify_chain_lock_through_core(
                    chain_lock,
                    make_sure_core_is_synced,
                    platform_version,
                )?;

                Ok(VerifyChainLockResult {
                    chain_lock_signature_is_deserializable: true,
                    found_valid_locally: None,
                    submitted: None,
                    found_valid_by_core: Some(verified),
                })
            }
            Err(Error::BLSError(_)) => Ok(VerifyChainLockResult {
                chain_lock_signature_is_deserializable: false,
                found_valid_locally: None,
                submitted: None,
                found_valid_by_core: None,
            }),
            Err(e) => Err(e),
        }
    }
}
