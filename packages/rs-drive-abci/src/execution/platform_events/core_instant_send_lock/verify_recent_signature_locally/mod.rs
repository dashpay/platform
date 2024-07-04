mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::dashcore::InstantLock;

use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;

/// Traits implements a method for signature verification using platform execution state
pub trait VerifyInstantLockSignature {
    fn verify_recent_signature_locally(
        &self,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error>;
}

impl VerifyInstantLockSignature for InstantLock {
    /// Verify instant lock signature with limited quorum set what we store in Platform state
    ///
    /// This is a limited verification and will work properly only for recently signed instant locks.
    /// Even valid instant locks that was signed some time ago will be considered invalid due to limited
    /// quorum information in the platform state. In turn, this verification doesn't use Core RPC or any other
    /// IO. This is done to prevent DoS attacks on slow verify instant lock signature Core RPC method.
    /// In case of failed signature verification (or any knowing the fact that signing quorum is old),
    /// we expect clients to use ChainAssetLockProof.
    fn verify_recent_signature_locally(
        &self,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_instant_send_lock
            .verify_recent_signature_locally
        {
            0 => v0::verify_recent_instant_lock_signature_locally_v0(self, platform_state),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "InstantLock.verify_recent_signature_locally".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
