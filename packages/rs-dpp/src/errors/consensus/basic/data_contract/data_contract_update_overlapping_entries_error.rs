use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use std::fmt;
use thiserror::Error;

/// The kind of contract entry a delta-based contract update names.
///
/// A V1 data contract update transition carries additive and in-place
/// changes per entry kind rather than a whole contract, so validation
/// errors report which section of the delta a name came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub enum DataContractUpdateEntryKind {
    /// A document type, keyed by its name.
    DocumentType,
    /// A shared `$defs` sub-schema, keyed by its definition name.
    SchemaDef,
    /// A group, keyed by its contract position.
    Group,
    /// A token, keyed by its contract position.
    Token,
    /// A search keyword.
    Keyword,
}

impl fmt::Display for DataContractUpdateEntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            DataContractUpdateEntryKind::DocumentType => "document type",
            DataContractUpdateEntryKind::SchemaDef => "schema definition",
            DataContractUpdateEntryKind::Group => "group",
            DataContractUpdateEntryKind::Token => "token",
            DataContractUpdateEntryKind::Keyword => "keyword",
        };
        f.write_str(name)
    }
}

/// The most bytes of an entry's name an error about a delta contract update carries.
///
/// A delta names the entries it changes, so a name in one is as long as a state transition
/// lets it be, while a consensus error is serialized in at most 2,000 bytes and cannot be
/// reported past that. A name over this bound is cut to it, so each of these errors is
/// reported whatever the delta names. A document type's name takes 64 characters at most
/// and a keyword 50, so a name the contract can hold is carried whole.
pub const MAX_DATA_CONTRACT_UPDATE_ENTRY_NAME_BYTES: usize = 256;

/// `name`, cut to [`MAX_DATA_CONTRACT_UPDATE_ENTRY_NAME_BYTES`] bytes where it is longer,
/// at the last character that fits whole.
pub(crate) fn bounded_entry_name(mut name: String) -> String {
    if name.len() > MAX_DATA_CONTRACT_UPDATE_ENTRY_NAME_BYTES {
        let mut end = MAX_DATA_CONTRACT_UPDATE_ENTRY_NAME_BYTES;
        while !name.is_char_boundary(end) {
            end -= 1;
        }
        name.truncate(end);
    }
    name
}

/// A delta-based contract update names the same entry in two sections
/// that cannot both apply: a document type or schema definition listed
/// as both new and updated, or a keyword listed as both added and removed.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeUntrusted,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
)]
#[error(
    "Data Contract {data_contract_id} update names {entry_kind} '{name}' in two conflicting sections"
)]
#[platform_serialize(unversioned)]
pub struct DataContractUpdateOverlappingEntriesError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    entry_kind: DataContractUpdateEntryKind,
    name: String,
}

impl DataContractUpdateOverlappingEntriesError {
    pub fn new(
        data_contract_id: Identifier,
        entry_kind: DataContractUpdateEntryKind,
        name: String,
    ) -> Self {
        Self {
            data_contract_id,
            entry_kind,
            name: bounded_entry_name(name),
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }

    pub fn entry_kind(&self) -> DataContractUpdateEntryKind {
        self.entry_kind
    }

    /// The entry's name, cut to [`MAX_DATA_CONTRACT_UPDATE_ENTRY_NAME_BYTES`] bytes.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl From<DataContractUpdateOverlappingEntriesError> for ConsensusError {
    fn from(err: DataContractUpdateOverlappingEntriesError) -> Self {
        Self::BasicError(BasicError::DataContractUpdateOverlappingEntriesError(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::state::data_contract::data_contract_update_entry_already_exists_error::DataContractUpdateEntryAlreadyExistsError;
    use crate::consensus::state::data_contract::data_contract_update_entry_not_found_error::DataContractUpdateEntryNotFoundError;
    use crate::serialization::{
        PlatformDeserializableUntrusted, PlatformSerializableWithPlatformVersion,
    };
    use platform_version::version::PlatformVersion;

    /// The most bytes a serialized consensus error takes: the limit `ConsensusError` is
    /// serialized and deserialized with.
    const CONSENSUS_ERROR_BYTES: usize = 2_000;

    #[test]
    fn should_carry_a_name_within_the_bound_whole() {
        let name = "n".repeat(MAX_DATA_CONTRACT_UPDATE_ENTRY_NAME_BYTES);

        assert_eq!(bounded_entry_name(name.clone()), name);
    }

    #[test]
    fn should_cut_a_longer_name_at_the_last_character_that_fits_whole() {
        // One byte, then characters of two bytes: the bound falls inside one of them
        let name = format!(
            "n{}",
            "\u{e9}".repeat(MAX_DATA_CONTRACT_UPDATE_ENTRY_NAME_BYTES)
        );

        let bounded = bounded_entry_name(name.clone());

        assert_eq!(bounded.len(), MAX_DATA_CONTRACT_UPDATE_ENTRY_NAME_BYTES - 1);
        assert!(name.starts_with(&bounded));
    }

    #[test]
    fn should_report_an_error_about_a_name_longer_than_a_consensus_error() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([7; 32]);
        // A state transition has room for this name, and a consensus error has not
        let name = "n".repeat(3_000);
        assert!(name.len() > CONSENSUS_ERROR_BYTES);

        let errors: [ConsensusError; 3] = [
            DataContractUpdateOverlappingEntriesError::new(
                contract_id,
                DataContractUpdateEntryKind::Keyword,
                name.clone(),
            )
            .into(),
            DataContractUpdateEntryAlreadyExistsError::new(
                contract_id,
                DataContractUpdateEntryKind::SchemaDef,
                name.clone(),
            )
            .into(),
            DataContractUpdateEntryNotFoundError::new(
                contract_id,
                DataContractUpdateEntryKind::DocumentType,
                name.clone(),
            )
            .into(),
        ];

        for error in errors {
            let bytes = error
                .serialize_to_bytes_with_platform_version(platform_version)
                .expect("expected the error to fit a consensus error");
            assert!(bytes.len() <= CONSENSUS_ERROR_BYTES);

            let decoded = ConsensusError::deserialize_from_bytes_untrusted(&bytes)
                .expect("expected the error to be read back");
            assert_eq!(decoded, error);
        }
    }
}
