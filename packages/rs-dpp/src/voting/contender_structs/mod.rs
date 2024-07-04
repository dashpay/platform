mod contender;

use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::document::Document;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::fmt;

pub use contender::v0::{ContenderV0, ContenderWithSerializedDocumentV0};
pub use contender::{Contender, ContenderWithSerializedDocument};

/// Represents a finalized contender in the contested document vote poll.
/// This is for internal use where the document is in serialized form
///
/// This struct holds the identity ID of the contender, the serialized document,
/// and the vote tally.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct FinalizedContenderWithSerializedDocument {
    /// The identity ID of the contender.
    pub identity_id: Identifier,
    /// The serialized document associated with the contender.
    pub serialized_document: Vec<u8>,
    /// The vote tally for the contender.
    pub final_vote_tally: u32,
}

/// Represents a finalized contender in the contested document vote poll.
/// This is for keeping information about previous vote polls
///
/// This struct holds the identity ID of the contender, the serialized document,
/// and the vote tally.
#[derive(Debug, PartialEq, Eq, Clone, Default, Encode, Decode)]
pub struct FinalizedResourceVoteChoicesWithVoterInfo {
    /// The resource vote choice.
    pub resource_vote_choice: ResourceVoteChoice,
    /// The pro_tx_hashes of the voters for this contender along with their strength
    pub voters: Vec<(Identifier, u8)>,
}
impl fmt::Display for FinalizedResourceVoteChoicesWithVoterInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let voters_str: Vec<String> = self
            .voters
            .iter()
            .map(|(id, strength)| format!("{}:{}", id, strength))
            .collect();
        write!(
            f,
            "FinalizedResourceVoteChoicesWithVoterInfo {{ resource_vote_choice: {}, voters: [{}] }}",
            self.resource_vote_choice,
            voters_str.join(", ")
        )
    }
}

/// Represents a finalized contender in the contested document vote poll.
/// This is for internal use where the document is in serialized form
///
/// This struct holds the identity ID of the contender, the document,
/// and the vote tally.
#[derive(Debug, PartialEq, Clone)]
pub struct FinalizedContender {
    /// The identity ID of the contender.
    pub identity_id: Identifier,
    /// The document associated with the contender.
    pub document: Document,
    /// The still serialized document
    pub serialized_document: Vec<u8>,
    /// The vote tally for the contender.
    pub final_vote_tally: u32,
}

impl FinalizedContender {
    /// Try to get the finalized contender from a finalized contender with a serialized document
    pub fn try_from_contender_with_serialized_document(
        value: FinalizedContenderWithSerializedDocument,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let FinalizedContenderWithSerializedDocument {
            identity_id,
            serialized_document,
            final_vote_tally,
        } = value;

        Ok(FinalizedContender {
            identity_id,
            document: Document::from_bytes(&serialized_document, document_type, platform_version)?,
            serialized_document,
            final_vote_tally,
        })
    }
}

impl TryFrom<ContenderWithSerializedDocument> for FinalizedContenderWithSerializedDocument {
    type Error = ProtocolError;

    fn try_from(value: ContenderWithSerializedDocument) -> Result<Self, Self::Error> {
        let (identity_id, serialized_document, vote_tally) = match value {
            ContenderWithSerializedDocument::V0(v0) => {
                let ContenderWithSerializedDocumentV0 {
                    identity_id,
                    serialized_document,
                    vote_tally,
                } = v0;
                (identity_id, serialized_document, vote_tally)
            }
        };

        Ok(FinalizedContenderWithSerializedDocument {
            identity_id,
            serialized_document: serialized_document.ok_or(
                ProtocolError::CorruptedCodeExecution("expected serialized document".to_string()),
            )?,
            final_vote_tally: vote_tally.ok_or(ProtocolError::CorruptedCodeExecution(
                "expected vote tally".to_string(),
            ))?,
        })
    }
}
