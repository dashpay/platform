mod v0;

use crate::drive::identity::withdrawals::WithdrawalTransactionIndex;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contracts::withdrawals_contract;
use dpp::document::Document;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Find withdrawal documents by status and transaction indices
    pub fn find_withdrawal_documents_by_status_and_transaction_indices(
        &self,
        status: withdrawals_contract::WithdrawalStatus,
        transaction_indices: &[WithdrawalTransactionIndex],
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .document
            .find_up_to_100_withdrawal_documents_by_status_and_transaction_indices
        {
            0 => self.find_withdrawal_documents_by_status_and_transaction_indices_v0(
                status,
                transaction_indices,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "find_up_to_100_withdrawal_documents_by_status_and_transaction_indices"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
