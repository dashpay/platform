mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use grovedb::TransactionArg;

impl Drive {
    /// Opens a yes/no vote poll that takes votes until `end_date`: its tree with the stored
    /// info and the three vote sum trees, and its entry in the end date index. The prefunded
    /// balance the votes are paid from is the caller's to fund, under the poll's
    /// `specialized_balance_id`.
    ///
    /// Refuses parameters no poll can run with, and a poll that already exists.
    pub fn open_yes_no_vote_poll(
        &self,
        vote_poll: &YesNoVotePoll,
        end_date: TimestampMillis,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .open_yes_no_vote_poll
        {
            0 => self.open_yes_no_vote_poll_v0(
                vote_poll,
                end_date,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "open_yes_no_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operations of [`Self::open_yes_no_vote_poll`], added to `batch_operations`.
    ///
    /// They are stateful and have no estimated-costs path: a poll opens as a system action,
    /// outside any fee estimate. A caller that runs them inside a fee estimate must first give
    /// them one.
    #[allow(clippy::too_many_arguments)]
    pub fn open_yes_no_vote_poll_operations(
        &self,
        vote_poll: &YesNoVotePoll,
        end_date: TimestampMillis,
        block_info: &BlockInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .open_yes_no_vote_poll
        {
            0 => self.open_yes_no_vote_poll_operations_v0(
                vote_poll,
                end_date,
                block_info,
                previous_batch_operations,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "open_yes_no_vote_poll_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::votes::paths::{
        vote_decisions_tree_path, ACTIVE_POLLS_TREE_KEY, IDENTITY_VOTES_TREE_KEY,
    };
    use crate::util::grove_operations::DirectQueryType;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::test_yes_no_vote_poll as poll;
    use crate::util::test_helpers::vote_poll_end_dates;
    use dpp::block::block_info::BlockInfo;
    use dpp::platform_value::BinaryData;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_refuse_to_open_a_poll_a_batch_already_opens() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let vote_poll = poll();
        let mut batch_operations = vec![];
        drive
            .open_yes_no_vote_poll_operations(
                &vote_poll,
                1_000,
                &BlockInfo::default(),
                &mut None,
                &mut batch_operations,
                None,
                platform_version,
            )
            .expect("expected to open the poll");
        // The same poll with another end date: the end date is not part of its id.
        drive
            .open_yes_no_vote_poll_operations(
                &vote_poll,
                2_000,
                &BlockInfo::default(),
                &mut None,
                &mut batch_operations,
                None,
                platform_version,
            )
            .expect_err("expected the second opening in the batch to be refused");
    }

    #[test]
    fn should_create_the_decisions_trees_a_chain_lacks_when_opening_a_poll() {
        // A chain born at protocol version 14 has no decisions trees until it upgrades.
        let drive = setup_drive_with_initial_state_structure(Some(
            PlatformVersion::get(14).expect("expected protocol version 14"),
        ));
        let platform_version = PlatformVersion::latest();
        let has_decisions_tree = |key: char| {
            drive
                .grove_has_raw(
                    (&vote_decisions_tree_path()).into(),
                    &[key as u8],
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to read the decisions tree")
        };
        assert!(!has_decisions_tree(ACTIVE_POLLS_TREE_KEY));

        let vote_poll = poll();
        drive
            .open_yes_no_vote_poll(
                &vote_poll,
                1_000,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected to open the poll");

        assert!(has_decisions_tree(ACTIVE_POLLS_TREE_KEY));
        assert!(has_decisions_tree(IDENTITY_VOTES_TREE_KEY));
        assert!(drive
            .fetch_yes_no_vote_poll_stored_info(&vote_poll, None, platform_version)
            .expect("expected to fetch the stored info")
            .is_some());
    }

    #[test]
    fn should_refuse_to_open_a_poll_before_protocol_version_15() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        drive
            .open_yes_no_vote_poll(
                &poll(),
                1_000,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect_err("expected protocol version 14 to refuse opening a yes/no poll");
    }

    #[test]
    fn should_refuse_a_poll_that_ends_by_the_block_that_opens_it() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo {
            time_ms: 1_000,
            ..Default::default()
        };
        for end_date in [999, 1_000] {
            drive
                .open_yes_no_vote_poll(&poll(), end_date, &block_info, None, platform_version)
                .expect_err("expected a poll ending by the opening block to be refused");
        }
        drive
            .open_yes_no_vote_poll(&poll(), 1_001, &block_info, None, platform_version)
            .expect("expected a poll ending after the opening block to open");
    }

    #[test]
    fn should_queue_the_trees_two_openings_in_a_batch_share_once() {
        // A chain born at 14 lacks the decisions trees, which both openings need, and both polls
        // end at the same time. The test drive verifies batch consistency, which refuses a tree
        // queued twice.
        let drive = setup_drive_with_initial_state_structure(Some(
            PlatformVersion::get(14).expect("expected protocol version 14"),
        ));
        let platform_version = PlatformVersion::latest();
        let mut other_poll = poll();
        other_poll
            .resource_path
            .push(BinaryData::new(b"other".to_vec()));
        let mut batch_operations = vec![];
        for vote_poll in [poll(), other_poll.clone()] {
            drive
                .open_yes_no_vote_poll_operations(
                    &vote_poll,
                    1_000,
                    &BlockInfo::default(),
                    &mut None,
                    &mut batch_operations,
                    None,
                    platform_version,
                )
                .expect("expected to open the poll");
        }
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                batch_operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the batch to apply");
        let end_dates = vote_poll_end_dates(&drive, platform_version);
        assert_eq!(end_dates.len(), 1);
        assert_eq!(
            end_dates[&1_000],
            [
                poll().unique_id().expect("id"),
                other_poll.unique_id().expect("id")
            ]
            .into_iter()
            .collect()
        );
    }
}
