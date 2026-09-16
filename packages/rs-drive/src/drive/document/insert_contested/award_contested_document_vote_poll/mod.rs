mod v0;

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::FinalizedContenderWithSerializedDocument;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use grovedb::TransactionArg;

/// What the native award decided for an ended contested resource vote poll.
#[derive(Debug, Clone, PartialEq)]
pub struct ContestedDocumentVotePollAwardOutcome {
    /// The poll as the award resolved it against state: the identity the caller named, with
    /// the contract fetched from state by its id. The caller records and cleans the poll up
    /// with this, never with contract metadata of its own.
    pub vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
    /// The outcome the native rules selected: the awarded identity, a lock, or no winner.
    pub winner: ContestedDocumentVotePollWinnerInfo,
    /// Every contender the selection considered, with the vote tally each one finished with,
    /// sorted by tally descending. The caller records them with the finalized poll.
    pub contenders: Vec<FinalizedContenderWithSerializedDocument>,
}

impl Drive {
    /// Awards an ended contested resource vote poll: selects the winner with the native rules
    /// and inserts the winning document, in one operation.
    ///
    /// The operation takes no contender and no contract. The caller names the poll (contract
    /// id, document type name, index name and index values) and the end date; the contract,
    /// its document type and its index definition are fetched from state by that id inside
    /// the operation, so no caller-supplied metadata reaches the write. Everything else it
    /// awards is read from state too: the poll's stored status must be `Started`, the poll
    /// must be queued for finalization at `end_date`
    /// and the block time must have reached that end date, the winner is the contender with
    /// the highest vote tally (ties broken by the greatest creation time, then the greatest
    /// document id, exactly as the block executor selected before this operation existed), a lock tally
    /// above the top contender locks the poll, and the awarded bytes are the contender
    /// document as it was stored when the contest was joined. No data trigger, creation
    /// restriction or other document rule runs on the insert: the award is a native block
    /// event outside every ordinary-action rule scope.
    ///
    /// Any call that does not describe a live, ended, queued poll on a contract in state is
    /// rejected with `DriveError::ContestedAwardRejected` before anything is written, so a
    /// caller cannot award early, award a poll that was already finalized, name an end date
    /// the poll was never queued under, or name a contract that does not exist. The end-date queue entry is written once when the contest
    /// starts and removed by the cleanup that follows a legitimate award, which is what makes
    /// a second award of the same poll impossible after cleanup; between the award and that
    /// cleanup, a second call fails on the primary storage existence check of the insert.
    ///
    /// The operation is reachable only from the block executor's finalization event: no
    /// state transition, batched action or Drive batch operation maps to it.
    ///
    /// # Parameters
    /// * `vote_poll`: The identity of the poll to award; its contract is fetched from state.
    /// * `end_date`: The end date the poll is queued under, as the finalization sweep found it.
    /// * `block_info`: The block the award is applied in.
    /// * `transaction`: The transaction argument.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(ContestedDocumentVotePollAwardOutcome)` with the poll resolved against state, the
    ///   winner the native rules selected and the contenders they considered.
    /// * `Err(DriveError::ContestedAwardRejected)` if the poll names a contract that is not in
    ///   state, is not a started contest, has not ended, or is not queued at `end_date`;
    ///   nothing was applied.
    /// * `Err(DriveError::VersionNotActive)` if the platform version predates the operation.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known
    ///   versions.
    pub fn award_contested_document_vote_poll(
        &self,
        vote_poll: &ContestedDocumentResourceVotePoll,
        end_date: TimestampMillis,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentVotePollAwardOutcome, Error> {
        match platform_version
            .drive
            .methods
            .document
            .insert_contested
            .award_contested_document_vote_poll
        {
            Some(0) => self.award_contested_document_vote_poll_v0(
                vote_poll,
                end_date,
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "award_contested_document_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "award_contested_document_vote_poll".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
    use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
    use crate::util::batch::{DocumentOperationType, DriveOperation};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::platform_value::{Identifier, Value};

    /// A protocol version whose document method table predates the operation cannot
    /// dispatch it: the slot is `None`, and the dispatcher refuses before reading any
    /// state.
    #[test]
    fn should_refuse_to_dispatch_on_a_table_that_predates_the_operation() {
        let platform_version = PlatformVersion::get(14).expect("protocol version 14 exists");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id: Identifier::new([7; 32]),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text("quantum".to_string()),
            ],
        };

        let error = drive
            .award_contested_document_vote_poll(
                &vote_poll,
                1,
                &BlockInfo::default_with_time(2),
                None,
                platform_version,
            )
            .expect_err("a table without the slot must refuse the award");

        assert!(
            matches!(
                error,
                Error::Drive(DriveError::VersionNotActive { ref method, .. })
                    if method == "award_contested_document_vote_poll"
            ),
            "unexpected error: {error:?}"
        );
    }

    /// The award has no guest-reachable entry point. Every batched document action and
    /// every document batch operation is enumerated here without a wildcard, so adding a
    /// variant that could carry an award into either enum forces this test to be revisited.
    #[test]
    fn should_not_be_reachable_from_any_document_action_or_batch_operation() {
        fn document_action_is_ordinary(action: &DocumentTransitionAction) -> bool {
            match action {
                DocumentTransitionAction::CreateAction(_)
                | DocumentTransitionAction::ReplaceAction(_)
                | DocumentTransitionAction::DeleteAction(_)
                | DocumentTransitionAction::TransferAction(_)
                | DocumentTransitionAction::PurchaseAction(_)
                | DocumentTransitionAction::UpdatePriceAction(_)
                | DocumentTransitionAction::IndexOnlyDeleteAction(_) => true,
            }
        }

        fn batched_action_is_ordinary(action: &BatchedTransitionAction) -> bool {
            match action {
                BatchedTransitionAction::DocumentAction(action) => {
                    document_action_is_ordinary(action)
                }
                BatchedTransitionAction::TokenAction(_)
                | BatchedTransitionAction::BumpIdentityDataContractNonce(_) => true,
            }
        }

        fn document_operation_is_ordinary(operation: &DocumentOperationType) -> bool {
            match operation {
                DocumentOperationType::AddDocument { .. }
                | DocumentOperationType::AddContestedDocument { .. }
                | DocumentOperationType::UpdateDocument { .. }
                | DocumentOperationType::DeleteDocument { .. }
                | DocumentOperationType::DeleteIndexOnlyDocument { .. }
                | DocumentOperationType::AddWithdrawalDocument { .. }
                | DocumentOperationType::MultipleDocumentOperationsForSameContractDocumentType {
                    ..
                }
                | DocumentOperationType::DocumentHistory { .. } => true,
            }
        }

        fn drive_operation_is_ordinary(operation: &DriveOperation) -> bool {
            match operation {
                DriveOperation::DocumentOperation(operation) => {
                    document_operation_is_ordinary(operation)
                }
                DriveOperation::DataContractOperation(_)
                | DriveOperation::TokenOperation(_)
                | DriveOperation::WithdrawalOperation(_)
                | DriveOperation::IdentityOperation(_)
                | DriveOperation::PrefundedSpecializedBalanceOperation(_)
                | DriveOperation::SystemOperation(_)
                | DriveOperation::GroupOperation(_)
                | DriveOperation::AddressFundsOperation(_)
                | DriveOperation::ShieldedPoolOperation(_)
                | DriveOperation::GroveDBOperation(_)
                | DriveOperation::GroveDBOpBatch(_)
                | DriveOperation::FinalizeOperation(_) => true,
            }
        }

        // The functions above are the assertion: they compile only while no variant of
        // either enum is an award. They are referenced so the compiler keeps them.
        let _: fn(&BatchedTransitionAction) -> bool = batched_action_is_ordinary;
        let _: fn(&DriveOperation) -> bool = drive_operation_is_ordinary;
    }
}
