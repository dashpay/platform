use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::batch::DriveOperation;
use drive::drive::identity::withdrawals::WithdrawalTransactionIdAndBytes;
use drive::grovedb::TransactionArg;
use std::collections::HashMap;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Builds a list of Core transactions from withdrawal documents. This function is a version handler that
    /// directs to specific version implementations of the `build_withdrawal_transactions_from_documents` function.
    ///
    /// # Arguments
    ///
    /// * `documents` - A slice of `Document`.
    /// * `drive_operation_types` - A mutable reference to `Vec<DriveOperation>`.
    /// * `transaction` - A `TransactionArg` reference.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<HashMap<Identifier, WithdrawalTransactionIdAndBytes>, Error>` - Returns a HashMap containing withdrawal transactions if found, otherwise returns an `Error`.
    pub(in crate::execution::platform_events::identity_credit_withdrawal) fn build_withdrawal_transactions_from_documents(
        &self,
        documents: &[Document],
        drive_operation_types: &mut Vec<DriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<HashMap<Identifier, WithdrawalTransactionIdAndBytes>, Error> {
        match platform_version
            .drive_abci
            .methods
            .identity_credit_withdrawal
            .build_withdrawal_transactions_from_documents
        {
            0 => self.build_withdrawal_transactions_from_documents_v0(
                documents,
                drive_operation_types,
                transaction,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "build_withdrawal_transactions_from_documents".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
