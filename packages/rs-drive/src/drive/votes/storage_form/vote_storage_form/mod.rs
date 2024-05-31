use crate::drive::votes::paths::{CONTESTED_RESOURCE_TREE_KEY, VOTE_DECISIONS_TREE_KEY};
use crate::drive::votes::storage_form::contested_document_resource_storage_form::ContestedDocumentResourceVoteStorageForm;
use crate::drive::votes::tree_path_storage_form::TreePathStorageForm;
use crate::drive::RootTree::Votes;
use dpp::ProtocolError;

/// Represents the various storage forms of votes.
pub enum VoteStorageForm {
    /// Storage form for contested document resource votes.
    ContestedDocumentResource(ContestedDocumentResourceVoteStorageForm),
}

impl TreePathStorageForm for VoteStorageForm {
    fn try_from_tree_path(path: Vec<Vec<u8>>) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        if path.len() < 3 {
            return Err(ProtocolError::VoteError(format!(
                "path {} is not long enough to construct vote information",
                path.into_iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            )));
        }
        let key_0 = path.get(0).unwrap();
        let key_1 = path.get(1).unwrap();

        let Some(key_0_byte) = key_0.get(0) else {
            return Err(ProtocolError::VoteError(format!(
                "path {} first element must be a byte",
                path.into_iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            )));
        };

        if *key_0_byte != Votes as u8 {
            return Err(ProtocolError::VoteError(format!(
                "path {} first element must be a byte for voting {}, got {}",
                path.iter().map(hex::encode).collect::<Vec<_>>().join("/"),
                Votes as u8,
                *key_0_byte
            )));
        };

        let Some(key_1_byte) = key_1.get(0) else {
            return Err(ProtocolError::VoteError(format!(
                "path {} second element must be a byte",
                path.into_iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            )));
        };

        match *key_1_byte as char {
            CONTESTED_RESOURCE_TREE_KEY => Ok(VoteStorageForm::ContestedDocumentResource(
                ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path)?,
            )),
            VOTE_DECISIONS_TREE_KEY => Err(ProtocolError::NotSupported(
                "decision votes not supported yet".to_string(),
            )),
            _ => Err(ProtocolError::VoteError(format!(
                "path {} second element must be a byte for CONTESTED_RESOURCE_TREE_KEY {}, got {}",
                path.iter().map(hex::encode).collect::<Vec<_>>().join("/"),
                CONTESTED_RESOURCE_TREE_KEY as u8,
                *key_1_byte
            ))),
        }
    }
}
