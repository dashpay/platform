mod v0;

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::dashcore::Network;
use dpp::data_contract::config::moderation::ElectedModerators;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

/// The windows a contest runs on, in milliseconds from the block that started it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContestWindows {
    /// How long other contenders may join once the first one applied. A contest resolved
    /// without locking ends here while it has a single contender.
    pub join_window_ms: TimestampMillis,
    /// How long the whole poll runs once a second contender joined: the join window and the
    /// vote window.
    pub poll_duration_ms: TimestampMillis,
}

impl ContestWindows {
    /// The generic windows of the version tables, which every contest but a moderation election
    /// runs on: the mainnet ones on mainnet, the shorter test ones on every other network.
    pub fn generic(network: Network, platform_version: &PlatformVersion) -> Self {
        let validation = &platform_version.dpp.validation.voting;
        let voting = &platform_version.dpp.voting_versions;
        match network {
            Network::Mainnet => ContestWindows {
                join_window_ms: validation.allow_other_contenders_time_mainnet_ms,
                poll_duration_ms: voting.default_vote_poll_time_duration_mainnet_ms,
            },
            _ => ContestWindows {
                join_window_ms: validation.allow_other_contenders_time_testing_ms,
                poll_duration_ms: voting.default_vote_poll_time_duration_test_network_ms,
            },
        }
    }

    /// The windows an elected moderation declaration gives the elections for its contract. Its
    /// windows are seconds, each one day to four weeks.
    pub fn of_elected_moderators(elected: &ElectedModerators) -> Self {
        let join_window_ms = u64::from(elected.join_window).saturating_mul(1000);
        let vote_window_ms = u64::from(elected.vote_window).saturating_mul(1000);
        ContestWindows {
            join_window_ms,
            poll_duration_ms: join_window_ms.saturating_add(vote_window_ms),
        }
    }
}

impl Drive {
    /// Reads the windows of a moderation election: an `electedCharter` contest of the moderation
    /// charters contract runs on the join window and the vote window of the elected moderation
    /// declaration of the contract it contends for, the one value of its contested index.
    /// Every other contest keeps the generic windows of the version tables, and is answered
    /// without a read.
    ///
    /// # Parameters
    /// * `contested_document_resource_vote_poll`: the contest.
    /// * `epoch`: the epoch the read of the target contract is billed in.
    /// * `transaction`: the transaction to read in.
    /// * `platform_version`: the platform version.
    ///
    /// # Returns
    /// The fee of reading the target contract, `Some` whenever it was read, and the windows,
    /// `Some` only for a moderation election whose target exists and declares elected
    /// moderation. A missing target, or one that declares something else, gives `None`, so
    /// the contest keeps the generic windows instead of failing: the application's reference
    /// validation refuses both. Before protocol version 14 no moderation election exists,
    /// nothing is read and both are `None`.
    pub fn fetch_charter_election_windows(
        &self,
        contested_document_resource_vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<FeeResult>, Option<ContestWindows>), Error> {
        match platform_version
            .drive
            .methods
            .document
            .insert_contested
            .fetch_charter_election_windows
        {
            None => Ok((None, None)),
            Some(0) => self.fetch_charter_election_windows_v0(
                contested_document_resource_vote_poll,
                epoch,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_charter_election_windows".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
