use crate::consensus::basic::contract_moderation::{
    ContractModerationReasonTooLongError, InvalidContractModerationReasonDocumentsError,
};
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;
use crate::validation::SimpleConsensusValidationResult;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The longest document type name a contract admits, in bytes: the bound the document type
/// parser enforces on a schema, applied here to a name a reason cites.
const MAX_DOCUMENT_TYPE_NAME_LENGTH: usize = 64;

/// A document a moderation reason is about, named by its document type and its id.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Encode,
    Decode,
    DecodeUntrusted,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractModerationDocument {
    /// The document type of the document, on the moderated contract.
    pub document_type_name: String,
    /// The document's id.
    pub document_id: Identifier,
}

/// Why a moderator banned, suspended or warned an identity, or deleted a document. Every such
/// action carries one, and it is stored with the entry or the record, so whoever reads the
/// list reads the reason.
///
/// Nothing checks what a moderator writes: the text is free, so is the code, and the documents
/// a reason cites are not looked up. A cited document may have been deleted since, by its
/// author or by a moderator (whose deletion left a record), or may never have existed. The
/// one exception is the reason document a seated elected team names: it must be one its
/// proposal lists.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractModerationReason {
    /// Reserved for the ban codes a contract may declare in a later protocol version. No
    /// contract declares any today, so this is expected to be `None`. A value is accepted and
    /// stored as written, and is not checked against anything.
    #[serde(default)]
    pub code: Option<u16>,
    /// Free text, at most `SystemLimits::max_contract_moderation_reason_length` bytes of
    /// UTF-8. May be empty.
    pub text: String,
    /// The documents the reason is about: the posts a warning or a ban is for, at most
    /// `SystemLimits::max_contract_moderation_reason_documents`, none twice. Left out of the
    /// JSON when there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documents: Vec<ContractModerationDocument>,
    /// The `reason` document of the moderation charters system contract the action is taken
    /// on (protocol version 14). A seated elected team's ban, suspension, warning or document
    /// deletion must name one its proposal lists (`ModerationReasonNotListedError`); for every
    /// other moderator it is stored as written and checked against nothing. Last, so that a
    /// reason written before it existed decodes; left out of the JSON when there is none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_document_id: Option<Identifier>,
}

impl ContractModerationReason {
    /// A reason without a code, about no document, naming no reason document.
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            code: None,
            text: text.into(),
            documents: vec![],
            reason_document_id: None,
        }
    }

    /// The same reason, naming the reason document `reason_document_id`.
    pub fn with_reason_document(mut self, reason_document_id: Identifier) -> Self {
        self.reason_document_id = Some(reason_document_id);
        self
    }

    /// The same reason, about `documents`.
    pub fn with_documents(mut self, documents: Vec<ContractModerationDocument>) -> Self {
        self.documents = documents;
        self
    }

    /// Checks the length of the text and the documents cited: at most the limit, each with a
    /// document type name a contract could admit (one to 64 bytes), and none twice. The code
    /// is not checked, and neither is whether a cited document, or its type, exists.
    pub fn validate(&self, platform_version: &PlatformVersion) -> SimpleConsensusValidationResult {
        let max_length = platform_version
            .system_limits
            .max_contract_moderation_reason_length;
        if self.text.len() > max_length as usize {
            return SimpleConsensusValidationResult::new_with_error(
                ContractModerationReasonTooLongError::new(self.text.len() as u64, max_length)
                    .into(),
            );
        }
        let max_documents = platform_version
            .system_limits
            .max_contract_moderation_reason_documents;
        if self.documents.len() > usize::from(max_documents) {
            return SimpleConsensusValidationResult::new_with_error(
                InvalidContractModerationReasonDocumentsError::new(format!(
                    "{} documents cited, at most {} allowed",
                    self.documents.len(),
                    max_documents
                ))
                .into(),
            );
        }
        let mut seen = BTreeSet::new();
        for document in &self.documents {
            let name_length = document.document_type_name.len();
            if name_length == 0 || name_length > MAX_DOCUMENT_TYPE_NAME_LENGTH {
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidContractModerationReasonDocumentsError::new(format!(
                        "a cited document type name is {} bytes long, it must be 1 to {}",
                        name_length, MAX_DOCUMENT_TYPE_NAME_LENGTH
                    ))
                    .into(),
                );
            }
            if !seen.insert(document) {
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidContractModerationReasonDocumentsError::new(format!(
                        "document {} of type {} is cited twice",
                        document.document_id, document.document_type_name
                    ))
                    .into(),
                );
            }
        }
        SimpleConsensusValidationResult::new()
    }
}

#[cfg(feature = "json-conversion")]
impl JsonSafeFields for ContractModerationDocument {}

#[cfg(feature = "json-conversion")]
impl JsonSafeFields for ContractModerationReason {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use platform_value::string_encoding::Encoding;

    #[test]
    fn should_round_trip_through_json_with_and_without_a_code() {
        let reason = ContractModerationReason {
            code: Some(7),
            text: "spam".to_string(),
            documents: vec![],
            reason_document_id: None,
        };
        let json = serde_json::to_value(&reason).expect("to json");
        assert_eq!(json, serde_json::json!({"code": 7, "text": "spam"}));
        assert_eq!(
            serde_json::from_value::<ContractModerationReason>(json).expect("from json"),
            reason
        );

        // The code may be left out
        assert_eq!(
            serde_json::from_value::<ContractModerationReason>(serde_json::json!({"text": ""}))
                .expect("from json"),
            ContractModerationReason::from_text("")
        );
    }

    fn document(seed: u8) -> ContractModerationDocument {
        ContractModerationDocument {
            document_type_name: "post".to_string(),
            document_id: Identifier::from([seed; 32]),
        }
    }

    #[test]
    fn should_round_trip_the_documents_through_json_and_leave_them_out_when_none() {
        let reason = ContractModerationReason::from_text("spam").with_documents(vec![document(1)]);
        let json = serde_json::to_value(&reason).expect("to json");
        assert_eq!(json["documents"][0]["documentTypeName"], "post");
        assert_eq!(
            json["documents"][0]["documentId"],
            Identifier::from([1; 32]).to_string(Encoding::Base58)
        );
        assert_eq!(
            serde_json::from_value::<ContractModerationReason>(json).expect("from json"),
            reason
        );
        // A reason about no document has no `documents` key, as before documents existed.
        let json =
            serde_json::to_value(ContractModerationReason::from_text("spam")).expect("to json");
        assert!(json.get("documents").is_none());
    }

    #[test]
    fn should_bound_the_documents_cited_and_refuse_one_cited_twice() {
        let platform_version = PlatformVersion::latest();
        let max_documents = platform_version
            .system_limits
            .max_contract_moderation_reason_documents as u8;
        let cited = |count: u8| {
            ContractModerationReason::from_text("spam")
                .with_documents((1..=count).map(document).collect())
        };
        assert!(cited(max_documents).validate(platform_version).is_valid());
        let refused = |reason: ContractModerationReason| {
            matches!(
                reason.validate(platform_version).errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidContractModerationReasonDocumentsError(_)
                )]
            )
        };
        assert!(refused(cited(max_documents + 1)));
        assert!(refused(
            ContractModerationReason::from_text("spam")
                .with_documents(vec![document(1), document(1)])
        ));
        // The same id under another type is another document.
        let other_type = ContractModerationDocument {
            document_type_name: "reply".to_string(),
            ..document(1)
        };
        assert!(ContractModerationReason::from_text("spam")
            .with_documents(vec![document(1), other_type])
            .validate(platform_version)
            .is_valid());
        for name in ["", &"n".repeat(65)] {
            assert!(refused(
                ContractModerationReason::from_text("spam").with_documents(vec![
                    ContractModerationDocument {
                        document_type_name: name.to_string(),
                        document_id: Identifier::from([1; 32]),
                    }
                ])
            ));
        }
        // Nothing checks whether the document exists.
        assert!(cited(1).validate(platform_version).is_valid());
    }

    #[test]
    fn should_round_trip_the_reason_document_through_json_and_leave_it_out_when_none() {
        let reason = ContractModerationReason::from_text("spam")
            .with_reason_document(Identifier::from([4; 32]));
        let json = serde_json::to_value(&reason).expect("to json");
        assert_eq!(
            json["reasonDocumentId"],
            Identifier::from([4; 32]).to_string(Encoding::Base58)
        );
        assert_eq!(
            serde_json::from_value::<ContractModerationReason>(json).expect("from json"),
            reason
        );
        let json =
            serde_json::to_value(ContractModerationReason::from_text("spam")).expect("to json");
        assert!(json.get("reasonDocumentId").is_none());
    }

    #[test]
    fn should_refuse_an_unknown_field() {
        serde_json::from_value::<ContractModerationReason>(
            serde_json::json!({"text": "spam", "note": "x"}),
        )
        .expect_err("unknown field");
    }

    #[test]
    fn should_accept_any_code_and_a_text_up_to_the_limit() {
        let platform_version = PlatformVersion::latest();
        let max_length = platform_version
            .system_limits
            .max_contract_moderation_reason_length as usize;

        let reason = ContractModerationReason {
            code: Some(u16::MAX),
            text: "é".repeat(max_length / 2),
            documents: vec![],
            reason_document_id: None,
        };
        assert_eq!(reason.text.len(), max_length);
        assert!(reason.validate(platform_version).is_valid());
        assert!(ContractModerationReason::default()
            .validate(platform_version)
            .is_valid());
    }

    #[test]
    fn should_refuse_a_text_over_the_limit_counted_in_bytes() {
        let platform_version = PlatformVersion::latest();
        let max_length = platform_version
            .system_limits
            .max_contract_moderation_reason_length;

        // One character under the limit in characters, over it in bytes
        let reason = ContractModerationReason::from_text("é".repeat(max_length as usize / 2 + 1));
        let result = reason.validate(platform_version);
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::ContractModerationReasonTooLongError(error))]
                if error.length() == max_length as u64 + 2 && error.max_length() == max_length
        ));
    }
}
