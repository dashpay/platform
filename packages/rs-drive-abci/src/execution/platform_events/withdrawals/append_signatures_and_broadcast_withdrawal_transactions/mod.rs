use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::Transaction;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;
use tenderdash_abci::proto::types::VoteExtension;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Appends signatures to unsigned withdrawal transactions and broadcast them to Core
    pub(in crate::execution) fn append_signatures_and_broadcast_withdrawal_transactions(
        &self,
        withdrawal_transactions_with_vote_extensions: BTreeMap<&Transaction, &VoteExtension>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .append_signatures_and_broadcast_withdrawal_transactions
        {
            0 => self.append_signatures_and_broadcast_withdrawal_transactions_v0(
                withdrawal_transactions_with_vote_extensions,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "append_signatures_and_broadcast_withdrawal_transactions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
