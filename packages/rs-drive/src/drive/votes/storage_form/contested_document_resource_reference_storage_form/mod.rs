use bincode::{Decode, Encode};
use grovedb::reference_path::ReferencePathType;

/// Represents the storage form of a reference.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct ContestedDocumentResourceVoteReferenceStorageForm {
    /// The reference
    pub reference_path_type: ReferencePathType,

    /// The amount of times the identity has voted
    pub identity_vote_times: u16,
}
