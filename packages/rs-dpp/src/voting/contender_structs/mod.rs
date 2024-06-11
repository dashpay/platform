use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::document::Document;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::ProtocolError;
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::fmt;

/// Represents a contender in the contested document vote poll.
/// This is for internal use where the document is in serialized form
///
/// This struct holds the identity ID of the contender, the serialized document,
/// and the vote tally.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct ContenderWithSerializedDocument {
    /// The identity ID of the contender.
    pub identity_id: Identifier,
    /// The serialized document associated with the contender.
    pub serialized_document: Option<Vec<u8>>,
    /// The vote tally for the contender.
    pub vote_tally: Option<u32>,
}

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
    /// The pro_tx_hashes of the voters for this contender.
    pub voters: Vec<Identifier>,
}
impl fmt::Display for FinalizedResourceVoteChoicesWithVoterInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let voters_str: Vec<String> = self
            .voters
            .iter()
            .map(|v| v.to_string(Encoding::Base58))
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
        let ContenderWithSerializedDocument {
            identity_id,
            serialized_document,
            vote_tally,
        } = value;

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

/// Represents a contender in the contested document vote poll.
///
/// This struct holds the identity ID of the contender, the serialized document,
/// and the vote tally.
#[derive(Debug, PartialEq, Clone, Default)]
#[cfg_attr(
    feature = "document-serde-conversion",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "$version")
)]
pub struct Contender {
    /// The identity ID of the contender.
    pub identity_id: Identifier,
    /// The document associated with the contender.
    pub document: Option<Document>,
    /// The vote tally for the contender.
    pub vote_tally: Option<u32>,
}

impl From<FinalizedContender> for Contender {
    fn from(value: FinalizedContender) -> Self {
        let FinalizedContender {
            identity_id,
            document,
            final_vote_tally,
            ..
        } = value;

        Contender {
            identity_id,
            document: Some(document),
            vote_tally: Some(final_vote_tally),
        }
    }
}

impl Contender {
    /// Try to get the finalized contender from a finalized contender with a serialized document
    pub fn try_from_contender_with_serialized_document(
        value: ContenderWithSerializedDocument,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let ContenderWithSerializedDocument {
            identity_id,
            serialized_document,
            vote_tally,
        } = value;

        Ok(Contender {
            identity_id,
            document: serialized_document
                .map(|v| Document::from_bytes(&v, document_type, platform_version))
                .transpose()?,
            vote_tally,
        })
    }
}
