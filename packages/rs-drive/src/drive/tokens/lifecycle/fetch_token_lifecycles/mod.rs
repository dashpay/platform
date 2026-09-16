mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::tokens::contract_lifecycle::TokenLifecycle;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Resolves token ids to the state of their issuers: live, or wiped at a block height.
    ///
    /// A token id that has no contract info leaf is not a token this node knows and is left
    /// out of the result; callers treat a missing entry as an unknown token, not as a live
    /// one.
    ///
    /// # Parameters
    ///
    /// * `token_ids` - The tokens to resolve.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * One entry per known token.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without the ledger.
    pub fn fetch_token_lifecycles(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], TokenLifecycle>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .lifecycle
            .fetch_token_lifecycles
        {
            Some(0) => self.fetch_token_lifecycles_v0(token_ids, transaction, platform_version),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_token_lifecycles".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_lifecycles".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
