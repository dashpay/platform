mod v0;
mod v1;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::dashcore::ChainLock;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use crate::execution::platform_events::core_chain_lock::make_sure_core_is_synced_to_chain_lock::CoreSyncStatus;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Verify the chain lock through core, for a chain lock Platform could not verify against
    /// the chain lock quorums it holds.
    ///
    /// # Parameters
    ///
    /// * `chain_lock` - The chain lock to verify
    /// * `submit` - Whether to submit the chain lock to Core to find out if Core is synced to it
    /// * `platform_version` - The platform version selecting the implementation
    ///
    /// # Returns
    ///
    /// * `Ok((valid, None))` - Without `submit`, Core's `verifychainlock` verdict. From version
    ///   1 also with `submit`, when `verifychainlock` fails the chain lock, which is then not
    ///   submitted
    /// * `Ok((true, Some(status)))` - With `submit`, whether Core is synced to the chain lock,
    ///   from `submitchainlock`. Version 0 submits without checking the signature, and
    ///   `submitchainlock` skips that check for a height Core already has a chain lock at or
    ///   above; version 1 submits only a chain lock `verifychainlock` accepted
    /// * `Err(Error::CoreRpc)` - A Core RPC call failed
    /// * `Err(Error::Execution)` - An unknown method version, here or in
    ///   `make_sure_core_is_synced_to_chain_lock`, which version 1 submits through
    pub fn verify_chain_lock_through_core(
        &self,
        chain_lock: &ChainLock,
        submit: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(bool, Option<CoreSyncStatus>), Error> {
        match platform_version
            .drive_abci
            .methods
            .core_chain_lock
            .verify_chain_lock_through_core
        {
            0 => self.verify_chain_lock_through_core_v0(chain_lock, submit, platform_version),
            1 => self.verify_chain_lock_through_core_v1(chain_lock, submit, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "verify_chain_lock_through_core".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::execution::platform_events::core_chain_lock::make_sure_core_is_synced_to_chain_lock::CoreSyncStatus;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::chain_lock_quorums::{
        chain_lock_test_platform, sign_chain_lock, TestQuorum,
    };
    use dpp::version::PlatformVersion;

    #[test]
    fn should_check_signature_before_submitting_from_protocol_version_15() {
        // A forged chain lock left to Core, with Core already holding a chain lock at 1140, so
        // submitchainlock answers that height without checking the signature. Protocol versions
        // up to 14 submit it without asking for a signature check, and must keep doing so for
        // the blocks committed under them to replay the same way. From 15, Core's
        // verifychainlock checks the signature first, and its failure keeps the chain lock from
        // being submitted.
        const CORE_BEST_CHAIN_LOCK_HEIGHT: u32 = 1140;
        let mut platform = chain_lock_test_platform();
        let forged = sign_chain_lock(&TestQuorum::new(9, 0), 1128 + 8);

        for protocol_version in [13, 14] {
            let mut core_rpc = MockCoreRPCLike::new();
            core_rpc.expect_verify_chain_lock().times(0);
            core_rpc
                .expect_submit_chain_lock()
                .times(1)
                .returning(|_| Ok(CORE_BEST_CHAIN_LOCK_HEIGHT));
            platform.core_rpc = core_rpc;

            let (valid, status) = platform
                .platform
                .verify_chain_lock_through_core(
                    &forged,
                    true,
                    PlatformVersion::get(protocol_version).expect("protocol version exists"),
                )
                .expect("verification runs");

            assert!(valid, "protocol version {protocol_version}");
            assert!(
                matches!(status, Some(CoreSyncStatus::Done)),
                "protocol version {protocol_version} must keep its verdict"
            );
        }

        for platform_version in [
            PlatformVersion::get(15).expect("protocol version 15 exists"),
            PlatformVersion::latest(),
        ] {
            let mut core_rpc = MockCoreRPCLike::new();
            let asked = forged.clone();
            core_rpc
                .expect_verify_chain_lock()
                .withf(move |chain_lock| *chain_lock == asked)
                .times(1)
                .returning(|_| Ok(false));
            core_rpc.expect_submit_chain_lock().times(0);
            platform.core_rpc = core_rpc;

            let (valid, status) = platform
                .platform
                .verify_chain_lock_through_core(&forged, true, platform_version)
                .expect("verification runs");

            assert!(
                !valid,
                "protocol version {}",
                platform_version.protocol_version
            );
            assert!(status.is_none(), "submitchainlock must not be asked");
        }
    }
}
