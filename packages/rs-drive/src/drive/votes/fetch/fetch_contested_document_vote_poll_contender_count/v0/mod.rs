use crate::drive::votes::paths::{VotePollPaths, RESOURCE_LOCK_VOTE_TREE_KEY_U8_32};
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::GroveError;
use crate::util::grove_operations::DirectQueryType;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

/// The entries beside the contenders in the tree holding a poll's choices: the poll's stored
/// info, written with its first contender and kept after it ends, and its abstain and lock vote
/// trees, written with every contender when missing and removed when the poll ends. A poll with
/// a contender holds all three; one cleaned up after it ended holds its stored info alone.
const NON_CONTENDER_CHOICE_ENTRIES: u64 = 3;

impl Drive {
    /// Fetches how many contenders a contested document resource vote poll holds, counting at
    /// most `count_limit` of them
    #[inline(always)]
    pub(super) fn fetch_contested_document_vote_poll_contender_count_v0(
        &self,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        count_limit: u16,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, u16), Error> {
        let (parent_path, Some(choices_key)) = vote_poll.last_index_path(platform_version)? else {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a contested index has at least one property",
            )));
        };
        let mut cost_operations = vec![];
        let choices = match self.grove_get_raw_optional(
            parent_path.as_slice().into(),
            choices_key.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut cost_operations,
            &platform_version.drive,
        ) {
            Ok(choices) => choices,
            Err(Error::GroveDB(error))
                if matches!(
                    *error,
                    GroveError::PathNotFound(_)
                        | GroveError::PathParentLayerNotFound(_)
                        | GroveError::PathKeyNotFound(_)
                ) =>
            {
                None
            }
            Err(error) => return Err(error),
        };

        let contenders = match choices {
            None => 0,
            // A poll started from protocol version 14
            Some(Element::CountTree(_, count, _)) => count
                .saturating_sub(NON_CONTENDER_CHOICE_ENTRIES)
                .min(count_limit as u64)
                as u16,
            // A poll started before protocol version 14: its contenders are the keys after the
            // lock tree's, the last of the reserved keys
            Some(Element::Tree(..)) => {
                let mut query = Query::new();
                query.insert_range_after(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32.to_vec()..);
                let path_query = PathQuery::new(
                    vote_poll.contenders_path(platform_version)?,
                    SizedQuery::new(query, Some(count_limit), None),
                );
                let (keys, _) = self.grove_get_raw_path_query(
                    &path_query,
                    transaction,
                    QueryResultType::QueryKeyElementPairResultType,
                    &mut cost_operations,
                    &platform_version.drive,
                )?;
                u16::try_from(keys.len()).map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "a query limited to a u16 returns at most a u16 of results",
                    ))
                })?
            }
            Some(_) => {
                return Err(Error::Drive(DriveError::CorruptedDriveState(
                    "the choices of a contested vote poll must be held in a tree".to_string(),
                )))
            }
        };

        let fee_result = Drive::calculate_fee(
            None,
            Some(cost_operations),
            epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok((fee_result, contenders))
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::drive::Drive;
    use crate::util::object_size_info::DataContractOwnedResolvedInfo;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::add_dpns_name_contenders;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::DataContract;
    use dpp::identifier::Identifier;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use platform_version::version::PlatformVersion;

    /// A drive holding the DPNS contract
    fn drive_with_dpns() -> (Drive, DataContract) {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let dpns_contract = get_dpns_data_contract_fixture(
            Some(Identifier::from([7; 32])),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        drive
            .apply_contract(
                &dpns_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply the DPNS contract");
        (drive, dpns_contract)
    }

    #[test]
    fn should_count_the_contenders_of_a_poll_up_to_the_limit() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns_contract) = drive_with_dpns();
        let vote_poll = add_dpns_name_contenders(
            &drive,
            &dpns_contract,
            "quantum",
            0..5,
            |_| 1,
            &BlockInfo::default(),
            platform_version,
        );

        for (count_limit, expected) in [(10, 5), (5, 5), (3, 3)] {
            let (_, contenders) = drive
                .fetch_contested_document_vote_poll_contender_count(
                    &vote_poll,
                    count_limit,
                    &Default::default(),
                    None,
                    platform_version,
                )
                .expect("expected the contender count");
            assert_eq!(contenders, expected, "counting at most {count_limit}");
        }
    }

    /// The count is one element read, billed the same whatever the number of contenders
    #[test]
    fn should_read_the_count_of_a_poll_in_one_fetch_whatever_its_contenders() {
        let platform_version = PlatformVersion::latest();
        let [few, many] = [2, 40].map(|contenders| {
            let (drive, dpns_contract) = drive_with_dpns();
            let vote_poll = add_dpns_name_contenders(
                &drive,
                &dpns_contract,
                "quantum",
                0..contenders,
                |_| 1,
                &BlockInfo::default(),
                platform_version,
            );
            drive
                .fetch_contested_document_vote_poll_contender_count(
                    &vote_poll,
                    100,
                    &Default::default(),
                    None,
                    platform_version,
                )
                .expect("expected the contender count")
        });
        assert_eq!((few.1, many.1), (2, 40));
        assert_eq!(few.0, many.0);
    }

    #[test]
    fn should_count_no_contenders_for_a_poll_that_does_not_exist() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns_contract) = drive_with_dpns();
        add_dpns_name_contenders(
            &drive,
            &dpns_contract,
            "quantum",
            0..2,
            |_| 1,
            &BlockInfo::default(),
            platform_version,
        );
        // Another label under the same parent, and a parent nothing is under
        for (parent, label) in [("dash", "coolio"), ("nothing", "coolio")] {
            let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
                contract: DataContractOwnedResolvedInfo::OwnedDataContract(dpns_contract.clone()),
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![parent.into(), label.into()],
            };
            let (_, contenders) = drive
                .fetch_contested_document_vote_poll_contender_count(
                    &vote_poll,
                    10,
                    &Default::default(),
                    None,
                    platform_version,
                )
                .expect("expected the contender count");
            assert_eq!(contenders, 0, "{parent}/{label}");
        }
    }

    /// A poll started before protocol version 14 keeps a plain tree, and has its contenders
    /// counted by their keys, the ones joining at 14 included
    #[test]
    fn should_count_the_contenders_of_a_poll_started_before_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        let protocol_version_13 = PlatformVersion::get(13).expect("expected version 13");
        let (drive, dpns_contract) = drive_with_dpns();
        add_dpns_name_contenders(
            &drive,
            &dpns_contract,
            "quantum",
            0..3,
            |_| 1,
            &BlockInfo::default(),
            protocol_version_13,
        );
        let vote_poll = add_dpns_name_contenders(
            &drive,
            &dpns_contract,
            "quantum",
            3..5,
            |_| 1,
            &BlockInfo::default(),
            platform_version,
        );

        for (count_limit, expected) in [(10, 5), (4, 4)] {
            let (_, contenders) = drive
                .fetch_contested_document_vote_poll_contender_count(
                    &vote_poll,
                    count_limit,
                    &Default::default(),
                    None,
                    platform_version,
                )
                .expect("expected the contender count");
            assert_eq!(contenders, expected, "counting at most {count_limit}");
        }
    }
}
