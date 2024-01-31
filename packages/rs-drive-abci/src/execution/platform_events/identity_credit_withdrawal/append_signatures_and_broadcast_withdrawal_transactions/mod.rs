use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Appends signatures to unsigned withdrawal transactions and broadcast them to Core
    pub(in crate::execution) fn append_signatures_and_broadcast_withdrawal_transactions(
        &self,
        unsigned_withdrawal_transactions: UnsignedWithdrawalTxs,
        signatures: Vec<BLSSignature>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .identity_credit_withdrawal
            .append_signatures_and_broadcast_withdrawal_transactions
        {
            0 => self.append_signatures_and_broadcast_withdrawal_transactions_v0(
                unsigned_withdrawal_transactions,
                signatures,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "append_signatures_and_broadcast_withdrawal_transactions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
