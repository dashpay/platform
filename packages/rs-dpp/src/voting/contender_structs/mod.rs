mod contender;

use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::document::Document;
use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::ProtocolError;
use bincode::{Decode, Encode};
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voting::contender_structs::contender::v0::ContenderWithSerializedDocumentV0;
    use platform_value::Identifier;

    mod finalized_resource_vote_choices_display {
        use super::*;

        #[test]
        fn display_with_no_voters() {
            let item = FinalizedResourceVoteChoicesWithVoterInfo {
                resource_vote_choice: ResourceVoteChoice::Abstain,
                voters: vec![],
            };
            let s = item.to_string();
            assert!(s.contains("resource_vote_choice"));
            assert!(s.contains("voters: []"));
        }

        #[test]
        fn display_with_multiple_voters() {
            let id1 = Identifier::new([1u8; 32]);
            let id2 = Identifier::new([2u8; 32]);
            let item = FinalizedResourceVoteChoicesWithVoterInfo {
                resource_vote_choice: ResourceVoteChoice::Lock,
                voters: vec![(id1, 1), (id2, 4)],
            };
            let s = item.to_string();
            // Each voter rendered as id:strength and comma separated
            assert!(s.contains(&format!("{}:1", id1)));
            assert!(s.contains(&format!("{}:4", id2)));
            assert!(s.contains(", "));
        }
    }

    mod finalized_resource_vote_choices_encoding {
        use super::*;
        use bincode::config;

        #[test]
        fn roundtrip_preserves_data() {
            let id = Identifier::new([7u8; 32]);
            let original = FinalizedResourceVoteChoicesWithVoterInfo {
                resource_vote_choice: ResourceVoteChoice::TowardsIdentity(id),
                voters: vec![(id, 3)],
            };
            let cfg = config::standard();
            let bytes = bincode::encode_to_vec(&original, cfg).expect("encode");
            let (decoded, _): (FinalizedResourceVoteChoicesWithVoterInfo, _) =
                bincode::decode_from_slice(&bytes, cfg).expect("decode");
            assert_eq!(decoded, original);
        }
    }

    mod finalized_contender_with_serialized_document_default {
        use super::*;

        #[test]
        fn default_has_zero_fields() {
            let fc = FinalizedContenderWithSerializedDocument::default();
            assert_eq!(fc.identity_id, Identifier::default());
            assert!(fc.serialized_document.is_empty());
            assert_eq!(fc.final_vote_tally, 0);
        }
    }

    mod try_from_contender_with_serialized_document {
        use super::*;

        #[test]
        fn err_when_serialized_document_missing() {
            let csd = ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
                identity_id: Identifier::new([1u8; 32]),
                serialized_document: None,
                vote_tally: Some(10),
            });
            let result = FinalizedContenderWithSerializedDocument::try_from(csd);
            match result {
                Err(ProtocolError::CorruptedCodeExecution(msg)) => {
                    assert!(msg.contains("expected serialized document"));
                }
                _ => panic!("expected CorruptedCodeExecution error for missing document"),
            }
        }

        #[test]
        fn err_when_vote_tally_missing() {
            let csd = ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
                identity_id: Identifier::new([1u8; 32]),
                serialized_document: Some(vec![1, 2, 3]),
                vote_tally: None,
            });
            let result = FinalizedContenderWithSerializedDocument::try_from(csd);
            match result {
                Err(ProtocolError::CorruptedCodeExecution(msg)) => {
                    assert!(msg.contains("expected vote tally"));
                }
                _ => panic!("expected CorruptedCodeExecution error for missing vote tally"),
            }
        }

        #[test]
        fn ok_when_all_fields_present() {
            let id = Identifier::new([9u8; 32]);
            let doc = vec![0xAA, 0xBB, 0xCC];
            let csd = ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
                identity_id: id,
                serialized_document: Some(doc.clone()),
                vote_tally: Some(42),
            });
            let finalized =
                FinalizedContenderWithSerializedDocument::try_from(csd).expect("should succeed");
            assert_eq!(finalized.identity_id, id);
            assert_eq!(finalized.serialized_document, doc);
            assert_eq!(finalized.final_vote_tally, 42);
        }
    }

    mod finalized_contender_with_serialized_document_equality {
        use super::*;

        #[test]
        fn equal_structs_compare_equal() {
            let id = Identifier::new([3u8; 32]);
            let a = FinalizedContenderWithSerializedDocument {
                identity_id: id,
                serialized_document: vec![1, 2, 3],
                final_vote_tally: 100,
            };
            let b = FinalizedContenderWithSerializedDocument {
                identity_id: id,
                serialized_document: vec![1, 2, 3],
                final_vote_tally: 100,
            };
            assert_eq!(a, b);
            // Clone preserves fields
            let c = a.clone();
            assert_eq!(a, c);
        }

        #[test]
        fn different_tally_not_equal() {
            let id = Identifier::new([3u8; 32]);
            let a = FinalizedContenderWithSerializedDocument {
                identity_id: id,
                serialized_document: vec![1, 2, 3],
                final_vote_tally: 100,
            };
            let b = FinalizedContenderWithSerializedDocument {
                identity_id: id,
                serialized_document: vec![1, 2, 3],
                final_vote_tally: 101,
            };
            assert_ne!(a, b);
        }
    }
}
