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

        let Some(key_2_byte) = key_2.first() else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::paths::ACTIVE_POLLS_TREE_KEY;

    /// Build a valid 10-element path whose third element is ACTIVE_POLLS_TREE_KEY,
    /// contract id is 32 bytes, document type name is valid UTF-8, and the
    /// 3-before-last element is a 32-byte identity-style choice.
    ///
    /// Layout indices (len == 10):
    ///   0: root (unused by this parser but required by preceding caller)
    ///   1: (unused here)
    ///   2: active_polls_key
    ///   3: contract_id (32 bytes)
    ///   4: document_type_name utf-8
    ///   5: first "real" index value that will be captured in index_values
    ///   6..len-3 : index values
    ///   len-3   : vote choice (key_vote_choice, 32 bytes)
    ///   len-2, len-1: tail (ignored by this parser)
    fn make_valid_path(choice: [u8; 32], first_index_byte: u8) -> Vec<Vec<u8>> {
        vec![
            vec![0u8],                         // 0: root placeholder
            vec![0u8],                         // 1: placeholder
            vec![ACTIVE_POLLS_TREE_KEY as u8], // 2: active polls
            vec![7u8; 32],                     // 3: contract_id (32 bytes)
            b"mytype".to_vec(),                // 4: document type name
            vec![first_index_byte],            // 5: leading byte
            vec![9u8],                         // 6: index value captured
            choice.to_vec(),                   // 7: vote choice (len-3)
            vec![0u8],                         // 8: tail
            vec![0u8],                         // 9: tail
        ]
    }

    #[test]
    fn try_from_tree_path_errors_when_too_short() {
        let path: Vec<Vec<u8>> = vec![vec![0u8]; 9];
        let err = match ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::VoteError(msg) => assert!(msg.contains("is not long enough"), "{msg}"),
            other => panic!("expected VoteError, got {other:?}"),
        }
    }

    #[test]
    fn try_from_tree_path_errors_when_third_element_empty() {
        let mut path = make_valid_path([0xAB; 32], 8);
        path[2] = vec![];
        let err = match ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::VoteError(msg) => {
                assert!(msg.contains("third element must be a byte"), "{msg}")
            }
            other => panic!("expected VoteError, got {other:?}"),
        }
    }

    #[test]
    fn try_from_tree_path_errors_when_third_element_is_not_active_polls() {
        let mut path = make_valid_path([0xAB; 32], 8);
        path[2] = vec![(ACTIVE_POLLS_TREE_KEY as u8).wrapping_add(1)];
        let err = match ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::VoteError(msg) => assert!(
                msg.contains("third element must be a byte for ACTIVE_POLLS_TREE_KEY"),
                "{msg}"
            ),
            other => panic!("expected VoteError, got {other:?}"),
        }
    }

    #[test]
    fn try_from_tree_path_errors_when_contract_id_wrong_length() {
        let mut path = make_valid_path([0xAB; 32], 8);
        path[3] = vec![1, 2, 3]; // not 32 bytes
        let err = match ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::VoteError(msg) => {
                assert!(msg.contains("32 bytes long"), "{msg}")
            }
            other => panic!("expected VoteError, got {other:?}"),
        }
    }

    #[test]
    fn try_from_tree_path_errors_when_document_type_name_not_utf8() {
        let mut path = make_valid_path([0xAB; 32], 8);
        // 0x80 on its own is not valid UTF-8
        path[4] = vec![0x80, 0xFF];
        let err = match ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::VoteError(msg) => {
                assert!(msg.contains("document type name"), "{msg}")
            }
            other => panic!("expected VoteError, got {other:?}"),
        }
    }

    #[test]
    fn try_from_tree_path_errors_when_vote_choice_not_32_bytes() {
        let mut path = make_valid_path([0xAB; 32], 8);
        let len = path.len();
        path[len - 3] = vec![1, 2, 3]; // non-32-byte choice
        let err = match ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::VoteError(msg) => {
                assert!(
                    msg.contains("identifier or RESOURCE_ABSTAIN_VOTE_TREE_KEY")
                        || msg.contains("2 before last element"),
                    "{msg}"
                )
            }
            other => panic!("expected VoteError, got {other:?}"),
        }
    }

    #[test]
    fn try_from_tree_path_parses_towards_identity_choice() {
        let identity_choice_bytes = [0x42u8; 32];
        let path = make_valid_path(identity_choice_bytes, 8);
        let form = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path)
            .expect("should parse");
        assert_eq!(form.contract_id, Identifier::from([7u8; 32]));
        assert_eq!(form.document_type_name, "mytype");
        assert!(matches!(
            form.resource_vote_choice,
            ResourceVoteChoice::TowardsIdentity(id) if id == Identifier::from(identity_choice_bytes)
        ));
        // index_values spans indices 6..len-3 which is just [vec![9u8]]
        assert_eq!(form.index_values, vec![vec![9u8]]);
    }

    #[test]
    fn try_from_tree_path_parses_lock_choice() {
        let path = make_valid_path(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, 8);
        let form = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path)
            .expect("should parse");
        assert_eq!(form.resource_vote_choice, ResourceVoteChoice::Lock);
    }

    #[test]
    fn try_from_tree_path_parses_abstain_choice() {
        let path = make_valid_path(RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, 8);
        let form = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path)
            .expect("should parse");
        assert_eq!(form.resource_vote_choice, ResourceVoteChoice::Abstain);
    }

    #[test]
    fn resolve_with_contract_unknown_version_returns_drive_error() {
        // Directly set an unsupported version into the platform version clone
        let mut pv = PlatformVersion::latest().clone();
        pv.drive.methods.vote.storage_form.resolve_with_contract = 99;

        let form = ContestedDocumentResourceVoteStorageForm {
            contract_id: Identifier::from([1u8; 32]),
            document_type_name: "ignored".to_string(),
            index_values: vec![],
            resource_vote_choice: ResourceVoteChoice::Abstain,
        };

        // We don't need a real contract; the version dispatch happens before any
        // contract-level work.
        let dummy_contract: DataContract =
            dpp::tests::fixtures::get_dpns_data_contract_fixture(None, 0, pv.protocol_version)
                .data_contract_owned();

        let err = form
            .resolve_with_contract(&dummy_contract, &pv)
            .expect_err("expected unknown version error");
        match err {
            Error::Drive(DriveError::UnknownVersionMismatch {
                method, received, ..
            }) => {
                assert!(method.contains("resolve_with_contract"), "method: {method}");
                assert_eq!(received, 99);
            }
            other => panic!("expected UnknownVersionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn resolve_with_contract_v0_errors_when_document_type_missing() {
        let pv = PlatformVersion::latest();
        let data_contract =
            dpp::tests::fixtures::get_dpns_data_contract_fixture(None, 0, pv.protocol_version)
                .data_contract_owned();

        let form = ContestedDocumentResourceVoteStorageForm {
            contract_id: data_contract.id(),
            document_type_name: "nonexistent_doc_type".to_string(),
            index_values: vec![],
            resource_vote_choice: ResourceVoteChoice::Abstain,
        };

        let err = form
            .resolve_with_contract(&data_contract, pv)
            .expect_err("expected error for missing document type");
        // We simply assert that it errors out; the exact error variant is the
        // contract's own "document type not found" error, surfaced as Error.
        let _ = err; // discarded on purpose -- any error is fine here
    }
}
