use crate::drive::votes::storage_form::contested_document_resource_storage_form::ContestedDocumentResourceVoteStorageForm;
use crate::drive::votes::tree_path_storage_form::TreePathStorageForm;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use crate::query::ContractLookupFn;
use crate::verify::bounded_decode::decode_vote_reference;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::voting::votes::resource_vote::ResourceVote;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl ContestedResourceVotesGivenByIdentityQuery {
    #[inline(always)]
    pub(crate) fn verify_identity_votes_given_proof_v0<I>(
        &self,
        proof: &[u8],
        contract_lookup_fn: &ContractLookupFn,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, I), Error>
    where
        I: FromIterator<(Identifier, (ResourceVote, u16))>,
    {
        let path_query = self.construct_path_query()?;
        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let voters = proved_key_values
            .into_iter()
            .filter_map(|(path, key, element)| element.map(|element| (path, key, element)))
            .map(|(path, key, element)| {
                let serialized_reference = element.into_item_bytes()?;
                let reference_storage_form = decode_vote_reference(&serialized_reference)?;
                let absolute_path = reference_storage_form
                    .reference_path_type
                    .absolute_path(path.as_slice(), Some(key.as_slice()))?;
                let vote_id = Identifier::from_vec(key)?;
                let vote_storage_form =
                    ContestedDocumentResourceVoteStorageForm::try_from_tree_path(absolute_path)?;
                let data_contract = contract_lookup_fn(&vote_storage_form.contract_id)?.ok_or(
                    Error::Drive(DriveError::DataContractNotFound(format!(
                        "data contract with id {} not found when verifying vote {}",
                        vote_storage_form.contract_id, vote_id
                    ))),
                )?;
                let resource_vote =
                    vote_storage_form.resolve_with_contract(&data_contract, platform_version)?;
                // The count is read from the vote reference the proof already carries, and
                // only clients verify proofs: no protocol version's consensus sees this value.
                Ok((
                    vote_id,
                    (resource_vote, reference_storage_form.identity_vote_times),
                ))
            })
            .collect::<Result<I, Error>>()?;

        Ok((root_hash, voters))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::drive::Drive;
    use crate::query::contract_lookup_fn_for_contract;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::{add_dpns_name_contenders, dpns_name_contender_id};
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::DataContract;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_polls::VotePoll;
    use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const VOTER_PRO_TX_HASH: [u8; 32] = [0xAB; 32];

    fn query() -> ContestedResourceVotesGivenByIdentityQuery {
        ContestedResourceVotesGivenByIdentityQuery {
            identity_id: Identifier::from(VOTER_PRO_TX_HASH),
            offset: None,
            limit: Some(10),
            start_at: None,
            order_ascending: true,
        }
    }

    /// A drive at `platform_version` holding the DPNS contract and a contest of two contenders
    /// on the name `quantum`.
    fn setup_drive_with_dpns_name_contest(
        platform_version: &PlatformVersion,
    ) -> (
        Drive,
        DataContract,
        ContestedDocumentResourceVotePollWithContractInfo,
    ) {
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
        let vote_poll = add_dpns_name_contenders(
            &drive,
            &dpns_contract,
            "quantum",
            0..2,
            |_| 1,
            &BlockInfo::default(),
            platform_version,
        );
        (drive, dpns_contract, vote_poll)
    }

    /// Proves the voter's votes on `drive` and verifies the proof, checking on the way that
    /// the verified root hash is the one of the drive's current state.
    fn prove_and_verify_votes(
        drive: &Drive,
        dpns_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> BTreeMap<Identifier, (ResourceVote, u16)> {
        let (proof, _) = query()
            .execute_with_proof(drive, None, None, platform_version)
            .expect("expected to execute query with proof");

        let (root_hash, votes) = query()
            .verify_identity_votes_given_proof(
                proof.as_slice(),
                &contract_lookup_fn_for_contract(Arc::new(dpns_contract.clone())),
                platform_version,
            )
            .expect("expected proof verification to succeed");

        let expected_root_hash = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected the root hash");
        assert_eq!(root_hash, expected_root_hash);

        votes
    }

    /// Votes the way a masternode vote state transition is applied: the stored vote, if any,
    /// is read first and handed to the registration with its count.
    fn vote(
        drive: &Drive,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        choice: ResourceVoteChoice,
        platform_version: &PlatformVersion,
    ) {
        let previous_vote = drive
            .fetch_identity_contested_resource_vote(
                Identifier::from(VOTER_PRO_TX_HASH),
                vote_poll.unique_id().expect("expected the vote poll id"),
                None,
                &mut vec![],
                platform_version,
            )
            .expect("expected to read the stored vote");
        drive
            .register_contested_resource_identity_vote(
                VOTER_PRO_TX_HASH,
                1,
                vote_poll.clone(),
                choice,
                previous_vote,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected to register the vote");
    }

    /// The entry the verifier is expected to read for a vote of `choice` on `vote_poll`.
    fn expected_votes(
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        choice: ResourceVoteChoice,
        vote_count: u16,
    ) -> BTreeMap<Identifier, (ResourceVote, u16)> {
        BTreeMap::from([(
            vote_poll.unique_id().expect("expected the vote poll id"),
            (
                ResourceVote::V0(ResourceVoteV0 {
                    vote_poll: VotePoll::ContestedDocumentResourceVotePoll(vote_poll.into()),
                    resource_vote_choice: choice,
                }),
                vote_count,
            ),
        )])
    }

    #[test]
    fn should_prove_and_verify_empty_identity_votes_given() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_id = Identifier::random();

        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset: None,
            limit: Some(10),
            start_at: None,
            order_ascending: true,
        };

        let (proof, _) = query
            .clone()
            .execute_with_proof(&drive, None, None, platform_version)
            .expect("expected to execute query with proof");

        let contract_lookup_fn: &ContractLookupFn = &|_id| Ok(None);

        let (_, votes): (_, BTreeMap<Identifier, (ResourceVote, u16)>) = query
            .verify_identity_votes_given_proof(
                proof.as_slice(),
                contract_lookup_fn,
                platform_version,
            )
            .expect("expected proof verification to succeed");

        assert!(votes.is_empty());
    }

    /// A poll the identity has not voted on has no entry; a first vote is counted as 1 and
    /// each changed vote adds 1, the entry always holding the current choice.
    #[test]
    fn should_prove_how_many_times_an_identity_voted_on_a_vote_poll() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns_contract, vote_poll) =
            setup_drive_with_dpns_name_contest(platform_version);

        // The poll exists, and the identity has not voted on it
        assert_eq!(
            prove_and_verify_votes(&drive, &dpns_contract, platform_version),
            BTreeMap::new()
        );

        let first_choice = ResourceVoteChoice::TowardsIdentity(dpns_name_contender_id(0));
        vote(&drive, &vote_poll, first_choice, platform_version);
        assert_eq!(
            prove_and_verify_votes(&drive, &dpns_contract, platform_version),
            expected_votes(&vote_poll, first_choice, 1)
        );

        let second_choice = ResourceVoteChoice::TowardsIdentity(dpns_name_contender_id(1));
        vote(&drive, &vote_poll, second_choice, platform_version);
        assert_eq!(
            prove_and_verify_votes(&drive, &dpns_contract, platform_version),
            expected_votes(&vote_poll, second_choice, 2)
        );

        vote(
            &drive,
            &vote_poll,
            ResourceVoteChoice::Lock,
            platform_version,
        );
        assert_eq!(
            prove_and_verify_votes(&drive, &dpns_contract, platform_version),
            expected_votes(&vote_poll, ResourceVoteChoice::Lock, 3)
        );
    }

    /// Protocol version 13 is the last released one (v4.1.1). This generation was edited in
    /// place to return the count, and there it still verifies a proof against the state's root
    /// hash and reads the vote it read before the edit.
    #[test]
    fn should_verify_the_votes_given_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        let (drive, dpns_contract, vote_poll) =
            setup_drive_with_dpns_name_contest(platform_version);

        let first_choice = ResourceVoteChoice::TowardsIdentity(dpns_name_contender_id(0));
        vote(&drive, &vote_poll, first_choice, platform_version);
        vote(
            &drive,
            &vote_poll,
            ResourceVoteChoice::Abstain,
            platform_version,
        );

        assert_eq!(
            prove_and_verify_votes(&drive, &dpns_contract, platform_version),
            expected_votes(&vote_poll, ResourceVoteChoice::Abstain, 2)
        );
    }
}
