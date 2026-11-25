use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A document create or replace, or a moderator's restore of a removed document, would take
/// the total its document type keeps of its summed property (`documentsSummable` or
/// `documentsAverageable`) past what the type's sums can hold. Every other sum of the type is
/// a sum over some of the same documents, so holding the total in range holds them all.
/// Refused by state validation from protocol version 15.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error("the write would take the total of \"{summed_property}\" over the documents of type \"{document_type_name}\" of contract {contract_id} past {limit}, the most its sums hold")]
#[platform_serialize(unversioned)]
pub struct DocumentSumOverflowError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
    summed_property: String,
    limit: i64,
}

impl DocumentSumOverflowError {
    pub fn new(
        contract_id: Identifier,
        document_type_name: String,
        summed_property: String,
        limit: i64,
    ) -> Self {
        Self {
            contract_id,
            document_type_name,
            summed_property,
            limit,
        }
    }

    pub fn contract_id(&self) -> &Identifier {
        &self.contract_id
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn summed_property(&self) -> &str {
        &self.summed_property
    }

    /// The most the type's total may reach.
    pub fn limit(&self) -> i64 {
        self.limit
    }
}

impl From<DocumentSumOverflowError> for ConsensusError {
    fn from(err: DocumentSumOverflowError) -> Self {
        Self::StateError(StateError::DocumentSumOverflowError(err))
    }
}
