use crate::drive::votes::paths::{
    vote_contested_resource_end_date_queries_at_time_tree_path_vec,
    vote_end_date_queries_tree_path_vec,
};
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::common::encode::encode_u64;
use crate::util::grove_operations::BatchDeleteApplyType;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::query_result_type::QueryResultType;
use grovedb::{MaybeTree, PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

impl Drive {
    /// We add votes poll references by end date in order to be able to check on every new block if
    /// any vote polls should be closed. This removes the references of the given vote polls, and
    /// removes an end date once none of its vote polls remain.
    // TODO: Use type of struct
    #[allow(clippy::type_complexity)]
    pub(in crate::drive::votes) fn remove_contested_resource_vote_poll_end_date_query_operations_v2(
        &self,
        vote_polls: &[(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // This is a GroveDB Tree (Not Sub Tree Merk representation)
        //                         End Date queries
        //              /                                  \
        //       15/08/2025 5PM                                   15/08/2025 6PM
        //          /              \                                    |
        //     VotePoll Info 1   VotePoll Info 2                 VotePoll Info 3

        let delete_apply_type = BatchDeleteApplyType::StatefulBatchDelete {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
        };

        let mut by_end_date: BTreeMap<TimestampMillis, BTreeSet<Identifier>> = BTreeMap::new();

        for (vote_poll, end_date, _) in vote_polls {
            by_end_date
                .entry(**end_date)
                .or_default()
                .insert(vote_poll.unique_id()?);
        }

        for (end_date, unique_ids) in by_end_date {
            let time_path =
                vote_contested_resource_end_date_queries_at_time_tree_path_vec(end_date);

            for unique_id in &unique_ids {
                self.batch_delete(
                    time_path.as_slice().into(),
                    unique_id.as_bytes(),
                    delete_apply_type,
                    transaction,
                    batch_operations,
                    &platform_version.drive,
                )?;
            }

            // The vote polls given here can be any subset of the ones under this end date, so
            // how many there are says nothing about what is left. Read one entry more than this
            // batch removes: the end date is left empty only when every entry read is removed
            // here, and then the read found all of them. A count too large for a query limit
            // reads without one.
            let limit = u16::try_from(unique_ids.len().saturating_add(1)).ok();
            let mut query = Query::new();
            query.insert_all();
            let path_query = PathQuery::new(time_path, SizedQuery::new(query, limit, None));
            let (entries, _) = self.grove_get_raw_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )?;
            let none_remain = entries.to_keys().into_iter().all(|key| {
                Identifier::from_bytes(&key).is_ok_and(|entry_id| unique_ids.contains(&entry_id))
            });

            if none_remain {
                // The end date holds nothing once this batch applies, so it goes in the same
                // batch, without an emptiness check of its own
                self.batch_delete(
                    vote_end_date_queries_tree_path_vec().as_slice().into(),
                    encode_u64(end_date).as_slice(),
                    delete_apply_type,
                    transaction,
                    batch_operations,
                    &platform_version.drive,
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
pub(super) mod tests {
    use crate::drive::votes::paths::{
        vote_contested_resource_end_date_queries_at_time_tree_path_vec,
        vote_end_date_queries_tree_path_vec,
    };
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::drive::Drive;
    use crate::error::Error;
    use crate::util::common::encode::decode_u64;
    use crate::util::object_size_info::DataContractOwnedResolvedInfo;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::DataContract;
    use dpp::identifier::Identifier;
    use dpp::identity::TimestampMillis;
    use dpp::platform_value::Value;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_polls::VotePoll;
    use grovedb::query_result_type::QueryResultType;
    use grovedb::{PathQuery, Query};
    use platform_version::version::PlatformVersion;
    use std::collections::{BTreeMap, BTreeSet};

    pub(in crate::drive::votes::cleanup) const T1: TimestampMillis = 1_000_000;
    pub(in crate::drive::votes::cleanup) const T2: TimestampMillis = 2_000_000;

    pub(in crate::drive::votes::cleanup) fn setup(
        platform_version: &PlatformVersion,
    ) -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let dpns_contract =
            get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version)
                .data_contract_owned();
        (drive, dpns_contract)
    }

    /// The DPNS name contest on `label`
    pub(in crate::drive::votes::cleanup) fn vote_poll(
        dpns_contract: &DataContract,
        label: &str,
    ) -> ContestedDocumentResourceVotePollWithContractInfo {
        ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(dpns_contract.clone()),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text(label.to_string()),
            ],
        }
    }

    /// Lists the vote poll under its end date, as a contested document create does
    pub(in crate::drive::votes::cleanup) fn add_end_date(
        drive: &Drive,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        end_date: TimestampMillis,
        platform_version: &PlatformVersion,
    ) {
        let mut operations = vec![];
        drive
            .add_vote_poll_end_date_query_operations(
                None,
                VotePoll::ContestedDocumentResourceVotePoll(vote_poll.into()),
                end_date,
                &BlockInfo::default(),
                &mut None,
                &mut None,
                &mut operations,
                None,
                platform_version,
            )
            .expect("expected end date operations");
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to list the vote poll under its end date");
    }

    /// Removes the end date entries of the given vote polls in one batch, as the cleanup after
    /// ended vote polls does
    pub(in crate::drive::votes::cleanup) fn remove_end_dates(
        drive: &Drive,
        ended: &[(
            &ContestedDocumentResourceVotePollWithContractInfo,
            TimestampMillis,
        )],
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let no_votes = BTreeMap::<ResourceVoteChoice, Vec<Identifier>>::new();
        let vote_polls = ended
            .iter()
            .map(|(vote_poll, end_date)| (*vote_poll, end_date, &no_votes))
            .collect::<Vec<_>>();
        let mut operations = vec![];
        drive.remove_contested_resource_vote_poll_end_date_query_operations(
            &vote_polls,
            &mut operations,
            None,
            platform_version,
        )?;
        drive.apply_batch_low_level_drive_operations(
            None,
            None,
            operations,
            &mut vec![],
            &platform_version.drive,
        )
    }

    /// Every end date with the vote polls listed under it; an end date with none shows as an
    /// empty set
    pub(in crate::drive::votes::cleanup) fn end_dates(
        drive: &Drive,
        platform_version: &PlatformVersion,
    ) -> BTreeMap<TimestampMillis, BTreeSet<Identifier>> {
        let keys_at = |path: Vec<Vec<u8>>| {
            let mut query = Query::new();
            query.insert_all();
            drive
                .grove_get_raw_path_query(
                    &PathQuery::new_unsized(path, query),
                    None,
                    QueryResultType::QueryKeyElementPairResultType,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to read the end date queries")
                .0
                .to_keys()
        };
        keys_at(vote_end_date_queries_tree_path_vec())
            .into_iter()
            .map(|date_key| {
                let end_date = decode_u64(&date_key).expect("expected an encoded end date");
                let unique_ids = keys_at(
                    vote_contested_resource_end_date_queries_at_time_tree_path_vec(end_date),
                )
                .into_iter()
                .map(|key| Identifier::from_bytes(&key).expect("expected a vote poll id"))
                .collect();
                (end_date, unique_ids)
            })
            .collect()
    }

    fn unique_id(vote_poll: &ContestedDocumentResourceVotePollWithContractInfo) -> Identifier {
        vote_poll.unique_id().expect("expected a vote poll id")
    }

    #[test]
    fn should_keep_an_end_date_while_one_of_its_vote_polls_remains() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns_contract) = setup(platform_version);
        let a = vote_poll(&dpns_contract, "a0000");
        let b = vote_poll(&dpns_contract, "b0000");
        let c = vote_poll(&dpns_contract, "c0000");
        add_end_date(&drive, &a, T1, platform_version);
        add_end_date(&drive, &b, T2, platform_version);
        add_end_date(&drive, &c, T2, platform_version);

        // One block ends A and B: they are the first two due, across both end dates
        remove_end_dates(&drive, &[(&a, T1), (&b, T2)], platform_version)
            .expect("expected the cleanup of A and B to apply");

        assert_eq!(
            end_dates(&drive, platform_version),
            BTreeMap::from([(T2, BTreeSet::from([unique_id(&c)]))]),
            "T1 goes with A, T2 stays with C under it"
        );

        // The next block ends C
        remove_end_dates(&drive, &[(&c, T2)], platform_version)
            .expect("expected the cleanup of C to apply");

        assert_eq!(end_dates(&drive, platform_version), BTreeMap::new());
    }

    #[test]
    fn should_remove_an_end_date_with_its_only_vote_poll() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns_contract) = setup(platform_version);
        let a = vote_poll(&dpns_contract, "a0000");
        add_end_date(&drive, &a, T1, platform_version);

        remove_end_dates(&drive, &[(&a, T1)], platform_version)
            .expect("expected the cleanup of A to apply");

        assert_eq!(end_dates(&drive, platform_version), BTreeMap::new());
    }

    #[test]
    fn should_remove_an_end_date_whose_vote_polls_all_end_at_the_limit() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns_contract) = setup(platform_version);
        let maximum_vote_polls_to_process = platform_version
            .drive_abci
            .validation_and_processing
            .event_constants
            .maximum_vote_polls_to_process as usize;
        let vote_polls = (0..maximum_vote_polls_to_process)
            .map(|i| vote_poll(&dpns_contract, &format!("a{i:04}")))
            .collect::<Vec<_>>();
        for vote_poll in &vote_polls {
            add_end_date(&drive, vote_poll, T1, platform_version);
        }

        let ended = vote_polls
            .iter()
            .map(|vote_poll| (vote_poll, T1))
            .collect::<Vec<_>>();
        remove_end_dates(&drive, &ended, platform_version)
            .expect("expected the cleanup of a full block of vote polls to apply");

        assert_eq!(end_dates(&drive, platform_version), BTreeMap::new());
    }

    #[test]
    fn should_keep_an_end_date_with_more_vote_polls_than_the_limit() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns_contract) = setup(platform_version);
        let a = vote_poll(&dpns_contract, "a0000");
        let b = vote_poll(&dpns_contract, "b0000");
        let c = vote_poll(&dpns_contract, "c0000");
        add_end_date(&drive, &a, T1, platform_version);
        add_end_date(&drive, &b, T1, platform_version);
        add_end_date(&drive, &c, T1, platform_version);

        remove_end_dates(&drive, &[(&a, T1), (&b, T1)], platform_version)
            .expect("expected the cleanup of A and B to apply");

        assert_eq!(
            end_dates(&drive, platform_version),
            BTreeMap::from([(T1, BTreeSet::from([unique_id(&c)]))])
        );

        remove_end_dates(&drive, &[(&c, T1)], platform_version)
            .expect("expected the cleanup of C to apply");

        assert_eq!(end_dates(&drive, platform_version), BTreeMap::new());
    }

    #[test]
    fn should_leave_other_end_dates_untouched() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns_contract) = setup(platform_version);
        let a = vote_poll(&dpns_contract, "a0000");
        let b = vote_poll(&dpns_contract, "b0000");
        add_end_date(&drive, &a, T1, platform_version);
        add_end_date(&drive, &b, T2, platform_version);

        remove_end_dates(&drive, &[(&a, T1)], platform_version)
            .expect("expected the cleanup of A to apply");

        assert_eq!(
            end_dates(&drive, platform_version),
            BTreeMap::from([(T2, BTreeSet::from([unique_id(&b)]))])
        );
    }
}
