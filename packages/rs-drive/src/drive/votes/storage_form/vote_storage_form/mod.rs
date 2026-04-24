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
        let key_0 = path.first().unwrap();
        let key_1 = path.get(1).unwrap();

        let Some(key_0_byte) = key_0.first() else {
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

        let Some(key_1_byte) = key_1.first() else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::paths::{
        ACTIVE_POLLS_TREE_KEY, CONTESTED_RESOURCE_TREE_KEY, VOTE_DECISIONS_TREE_KEY,
    };

    /// Returns a minimally valid path that reaches the ContestedDocumentResource
    /// dispatch branch, but still fails inside the inner try_from_tree_path (length
    /// check) -- sufficient to exercise this router's happy dispatch.
    fn path_with_prefix(second_byte: u8) -> Vec<Vec<u8>> {
        vec![
            vec![Votes as u8],
            vec![second_byte],
            vec![ACTIVE_POLLS_TREE_KEY as u8],
        ]
    }

    #[test]
    fn try_from_tree_path_errors_when_path_too_short() {
        // path.len() < 3 -> error
        let path = vec![vec![Votes as u8], vec![CONTESTED_RESOURCE_TREE_KEY as u8]];
        let err = match VoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::VoteError(msg) => {
                assert!(msg.contains("is not long enough"), "msg: {msg}");
            }
            other => panic!("expected VoteError, got {other:?}"),
        }
    }

    #[test]
    fn try_from_tree_path_errors_when_first_element_empty() {
        // first_element empty -> "first element must be a byte"
        let path = vec![
            vec![],
            vec![CONTESTED_RESOURCE_TREE_KEY as u8],
            vec![ACTIVE_POLLS_TREE_KEY as u8],
        ];
        let err = match VoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::VoteError(msg) => {
                assert!(msg.contains("first element must be a byte"), "msg: {msg}");
            }
            other => panic!("expected VoteError, got {other:?}"),
        }
    }

    #[test]
    fn try_from_tree_path_errors_when_root_key_not_votes() {
        // Wrong root byte (not Votes root tree) -> error
        let wrong_root: u8 = (Votes as u8).wrapping_add(1);
        let path = vec![
            vec![wrong_root],
            vec![CONTESTED_RESOURCE_TREE_KEY as u8],
            vec![ACTIVE_POLLS_TREE_KEY as u8],
        ];
        let err = match VoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::VoteError(msg) => {
                assert!(
                    msg.contains("first element must be a byte for voting"),
                    "msg: {msg}"
                );
            }
            other => panic!("expected VoteError, got {other:?}"),
        }
    }

    #[test]
    fn try_from_tree_path_errors_when_second_element_empty() {
        let path = vec![vec![Votes as u8], vec![], vec![ACTIVE_POLLS_TREE_KEY as u8]];
        let err = match VoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::VoteError(msg) => {
                assert!(msg.contains("second element must be a byte"), "msg: {msg}");
            }
            other => panic!("expected VoteError, got {other:?}"),
        }
    }

    #[test]
    fn try_from_tree_path_returns_not_supported_for_decision_votes() {
        let path = path_with_prefix(VOTE_DECISIONS_TREE_KEY as u8);
        let err = match VoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::NotSupported(msg) => {
                assert!(msg.contains("decision votes"), "msg: {msg}");
            }
            other => panic!("expected NotSupported, got {other:?}"),
        }
    }

    #[test]
    fn try_from_tree_path_errors_for_unknown_second_byte() {
        // Pick a byte that is neither CONTESTED_RESOURCE_TREE_KEY nor VOTE_DECISIONS_TREE_KEY
        let unknown_byte = b'Z';
        assert_ne!(unknown_byte, CONTESTED_RESOURCE_TREE_KEY as u8);
        assert_ne!(unknown_byte, VOTE_DECISIONS_TREE_KEY as u8);
        let path = path_with_prefix(unknown_byte);
        let err = match VoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::VoteError(msg) => {
                assert!(
                    msg.contains("second element must be a byte for CONTESTED_RESOURCE_TREE_KEY"),
                    "msg: {msg}"
                );
            }
            other => panic!("expected VoteError, got {other:?}"),
        }
    }

    #[test]
    fn try_from_tree_path_dispatches_to_contested_document_inner() {
        // For the CONTESTED_RESOURCE_TREE_KEY branch, dispatch reaches
        // ContestedDocumentResourceVoteStorageForm::try_from_tree_path, which requires
        // path.len() >= 10. With len == 3 the inner code errors "not long enough".
        let path = path_with_prefix(CONTESTED_RESOURCE_TREE_KEY as u8);
        let err = match VoteStorageForm::try_from_tree_path(path) {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        };
        match err {
            ProtocolError::VoteError(msg) => {
                assert!(msg.contains("is not long enough"), "msg: {msg}");
            }
            other => panic!("expected VoteError from inner, got {other:?}"),
        }
    }
}
