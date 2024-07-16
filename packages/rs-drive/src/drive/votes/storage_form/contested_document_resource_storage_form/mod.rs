use crate::drive::votes::paths::{
    ACTIVE_POLLS_TREE_KEY, RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, RESOURCE_LOCK_VOTE_TREE_KEY_U8_32,
};
use crate::drive::votes::tree_path_storage_form::TreePathStorageForm;
use crate::error::contract::DataContractError::{CorruptedDataContract, ProvidedContractMismatch};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::type_constants::DEFAULT_HASH_SIZE_USIZE;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::prelude::DataContract;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

/// Represents the storage form of a contested document resource vote.
#[derive(Debug, Clone, PartialEq)]
pub struct ContestedDocumentResourceVoteStorageForm {
    /// The identifier of the contract associated with the resource vote.
    pub contract_id: Identifier,

    /// The name of the document type associated with the resource vote.
    pub document_type_name: String,

    /// The index values associated with the resource vote, stored as a vector of byte vectors.
    pub index_values: Vec<Vec<u8>>,

    /// The choice of the resource vote, represented by a `ResourceVoteChoice` enum.
    pub resource_vote_choice: ResourceVoteChoice,
}

impl ContestedDocumentResourceVoteStorageForm {
    /// Resolves to a resource vote
    pub fn resolve_with_contract(
        self,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<ResourceVote, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .storage_form
            .resolve_with_contract
        {
            0 => self.resolve_with_contract_v0(data_contract),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "ContestedDocumentResourceVoteStorageForm::resolve_with_contract"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn resolve_with_contract_v0(self, data_contract: &DataContract) -> Result<ResourceVote, Error> {
        let ContestedDocumentResourceVoteStorageForm {
            contract_id,
            document_type_name,
            index_values,
            resource_vote_choice,
            ..
        } = self;

        let document_type = data_contract.document_type_for_name(document_type_name.as_str())?;

        let index = document_type
            .find_contested_index()
            .ok_or(Error::DataContract(ProvidedContractMismatch(
                "no contested index on provided contract".to_string(),
            )))?;

        let resolved_index_values = index_values
            .into_iter()
            .zip(index.properties.iter())
            .map(|(serialized_index_value, property)| {
                let document_property = document_type
                    .flattened_properties()
                    .get(property.name.as_str())
                    .ok_or(Error::DataContract(CorruptedDataContract(
                        "document type does not have a property of its index".to_string(),
                    )))?;
                let value = document_property
                    .property_type
                    .decode_value_for_tree_keys(serialized_index_value.as_slice())?;
                Ok(value)
            })
            .collect::<Result<Vec<Value>, Error>>()?;

        let vote_poll =
            VotePoll::ContestedDocumentResourceVotePoll(ContestedDocumentResourceVotePoll {
                contract_id,
                document_type_name,
                index_name: index.name.clone(),
                index_values: resolved_index_values,
            });

        Ok(ResourceVote::V0(ResourceVoteV0 {
            vote_poll,
            resource_vote_choice,
        }))
    }
}

impl TreePathStorageForm for ContestedDocumentResourceVoteStorageForm {
    fn try_from_tree_path(mut path: Vec<Vec<u8>>) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        if path.len() < 10 {
            return Err(ProtocolError::VoteError(format!(
                "path {} is not long enough to construct vote information",
                path.into_iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            )));
        }

        let key_2 = path.get(2).unwrap(); // active_vote_polls
        let key_contract_id = path.get(3).unwrap(); // contract_id
        let key_document_type_name = path.get(4).unwrap(); // document_type_name
        let key_vote_choice = path.get(path.len() - 3).unwrap(); // this is the vote choice

        let Some(key_2_byte) = key_2.get(0) else {
            return Err(ProtocolError::VoteError(format!(
                "path {} third element must be a byte",
                path.into_iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            )));
        };

        if *key_2_byte != ACTIVE_POLLS_TREE_KEY as u8 {
            return Err(ProtocolError::VoteError(format!(
                "path {} third element must be a byte for ACTIVE_POLLS_TREE_KEY {}, got {}",
                path.iter().map(hex::encode).collect::<Vec<_>>().join("/"),
                ACTIVE_POLLS_TREE_KEY as u8,
                *key_2_byte
            )));
        };

        if key_contract_id.len() != DEFAULT_HASH_SIZE_USIZE {
            return Err(ProtocolError::VoteError(format!(
                "path {} fourth element must be a contract id but isn't 32 bytes long",
                path.into_iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            )));
        }

        let contract_id = Identifier::from_vec(key_contract_id.clone())?;

        let document_type_name = String::from_utf8(key_document_type_name.clone()).map_err(|_| ProtocolError::VoteError(format!("path {} fifth element must be a document type name but couldn't be converted to a string", path.iter().map(hex::encode).collect::<Vec<_>>().join("/"))))?;

        let resource_vote_choice = if key_vote_choice.len() == 32 {
            if key_vote_choice.as_slice() == RESOURCE_LOCK_VOTE_TREE_KEY_U8_32.as_slice() {
                ResourceVoteChoice::Lock
            } else if key_vote_choice.as_slice() == RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.as_slice()
            {
                ResourceVoteChoice::Abstain
            } else {
                ResourceVoteChoice::TowardsIdentity(Identifier::from_vec(key_vote_choice.clone())?)
            }
        } else {
            return Err(ProtocolError::VoteError(format!("path {} 2 before last element must be an identifier or RESOURCE_ABSTAIN_VOTE_TREE_KEY/RESOURCE_LOCK_VOTE_TREE_KEY", path.into_iter().map(hex::encode).collect::<Vec<_>>().join("/"))));
        };

        // 6 is the first index value, then we have 2 at the end that are not index values
        let index_values = path.drain(6..path.len() - 3).collect::<Vec<_>>();

        Ok(ContestedDocumentResourceVoteStorageForm {
            contract_id,
            document_type_name,
            index_values,
            resource_vote_choice,
        })
    }
}
