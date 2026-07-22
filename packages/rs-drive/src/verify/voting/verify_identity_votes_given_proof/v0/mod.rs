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
        I: FromIterator<(Identifier, ResourceVote)>,
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
                Ok((vote_id, resource_vote))
            })
            .collect::<Result<I, Error>>()?;

        Ok((root_hash, voters))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

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

        let (_, votes): (_, BTreeMap<Identifier, ResourceVote>) = query
            .verify_identity_votes_given_proof(
                proof.as_slice(),
                contract_lookup_fn,
                platform_version,
            )
            .expect("expected proof verification to succeed");

        assert!(votes.is_empty());
    }
}
