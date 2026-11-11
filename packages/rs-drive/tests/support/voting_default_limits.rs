//! Shared deterministic voting state for the SDK proof and node-handler limit tests.

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::platform_value::Value;
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use drive::drive::votes::paths::VotePollPaths;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::drive::votes::resolved::vote_polls::ResolvedVotePoll;
use drive::drive::votes::resolved::votes::resolved_resource_vote::v0::ResolvedResourceVoteV0;
use drive::drive::votes::resolved::votes::resolved_resource_vote::ResolvedResourceVote;
use drive::drive::votes::resolved::votes::ResolvedVote;
use drive::drive::Drive;
use drive::grovedb::Element;
use drive::util::object_size_info::DataContractOwnedResolvedInfo;

#[path = "voting_default_limits_data.rs"]
mod voting_default_limits_data;
pub use voting_default_limits_data::{contract, poll, version, END_TIME, PROTOCOL_VERSION, VOTER};

pub fn populate(drive: &Drive, count: usize, version: &PlatformVersion) -> DataContract {
    let contract = contract(version);
    let index_name = contract
        .document_type_for_name("domain")
        .expect("DPNS domain")
        .find_contested_index()
        .expect("contested DPNS index")
        .name
        .clone();

    for i in 0..count {
        let poll = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract.clone()),
            document_type_name: "domain".to_string(),
            index_name: index_name.clone(),
            index_values: vec![
                Value::Text("dash".into()),
                Value::Text(format!("name{i:03}")),
            ],
        };
        let choice = ResourceVoteChoice::Abstain;
        let path = poll
            .contender_voting_path(&choice, version)
            .expect("vote storage path");
        for depth in 0..path.len() {
            let parent = &path[..depth];
            if !drive
                .grove
                .has_raw(parent, &path[depth], None, &version.drive.grove_version)
                .unwrap()
                .expect("tree existence")
            {
                let tree = if depth + 1 == path.len() {
                    Element::empty_sum_tree()
                } else {
                    Element::empty_tree()
                };
                drive
                    .grove
                    .insert(
                        parent,
                        &path[depth],
                        tree,
                        None,
                        None,
                        &version.drive.grove_version,
                    )
                    .unwrap()
                    .expect("vote subtree");
            }
        }
        let vote_poll = self::poll(&contract, i);
        let vote =
            ResolvedVote::ResolvedResourceVote(ResolvedResourceVote::V0(ResolvedResourceVoteV0 {
                resolved_vote_poll:
                    ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(poll),
                resource_vote_choice: choice,
            }));
        drive
            .register_identity_vote(VOTER, 1, vote, None, &BlockInfo::default(), None, version)
            .expect("stored identity vote");
        let mut operations = vec![];
        drive
            .add_vote_poll_end_date_query_operations(
                None,
                vote_poll,
                END_TIME + i as u64,
                &BlockInfo::default(),
                &mut None,
                &mut None,
                &mut operations,
                None,
                version,
            )
            .expect("poll end-date operations");
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &version.drive,
            )
            .expect("stored end-date poll");
    }
    contract
}
