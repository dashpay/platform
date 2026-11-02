//! What the FFI reads out of a contender's serialized document.
//!
//! Every contender of a contested name carries the document it submitted
//! (for DPNS, its `domain` document). The list queries and the vote-state
//! query both decode it to give callers the requester's own spelling and the
//! time the request was made; this module is the one place that decode lives.

use crate::types::DashSDKContender;
use dash_sdk::dpp::data_contract::document_type::DocumentTypeRef;
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::dpp::voting::contender_structs::ContenderWithSerializedDocument;
use std::ffi::CString;
use std::os::raw::c_char;

/// Fields decoded from a contender's document. Both are `None` when the
/// document could not be decoded.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ContenderDocumentFields {
    /// The label exactly as the contender typed it ("pizza"), where the
    /// contest is keyed by its normalized form ("p1zza"). `None` when empty.
    pub label: Option<String>,
    /// The document's `$createdAt`, in milliseconds since the Unix epoch:
    /// when this request was submitted. `None` for 0, which is not a time a
    /// document can have been created at, so every binding reads it the same
    /// way.
    pub created_at_ms: Option<u64>,
}

impl ContenderDocumentFields {
    /// The C contender the list queries hand out. Takes ownership of
    /// `identity_id`; the label is allocated here. Both are released by
    /// `free_contender_fields`, which every contender destructor goes through.
    pub(crate) fn into_ffi_contender(
        self,
        identity_id: *mut c_char,
        vote_count: u32,
    ) -> DashSDKContender {
        DashSDKContender {
            identity_id,
            vote_count,
            label: self
                .label
                .and_then(|label| CString::new(label).ok())
                .map(CString::into_raw)
                .unwrap_or(std::ptr::null_mut()),
            created_at: self.created_at_ms.unwrap_or(0),
            has_created_at: self.created_at_ms.is_some(),
        }
    }

    /// The optional members of a vote-state contender object, each with its
    /// leading comma: `,"label":"pizza","created_at":1760140800000`. A field
    /// that did not decode is left out, never written as null.
    pub(crate) fn json_members(&self) -> String {
        let label = self
            .label
            .as_ref()
            .and_then(|label| serde_json::to_string(label).ok())
            .map(|label| format!(r#","label":{}"#, label))
            .unwrap_or_default();
        let created_at = self
            .created_at_ms
            .map(|created_at| format!(r#","created_at":{}"#, created_at))
            .unwrap_or_default();
        label + &created_at
    }
}

/// Decode `contender`'s document against `document_type` and read its fields.
///
/// Best-effort: a missing document type, a contender without a serialized
/// document, or a document that does not decode gives empty fields rather than
/// an error, so one bad contender never fails the whole query.
pub(crate) fn contender_document_fields(
    contender: &ContenderWithSerializedDocument,
    document_type: Option<DocumentTypeRef>,
    platform_version: &PlatformVersion,
) -> ContenderDocumentFields {
    let Some(document_type) = document_type else {
        return ContenderDocumentFields::default();
    };
    let Ok(decoded) = contender.try_to_contender(document_type, platform_version) else {
        return ContenderDocumentFields::default();
    };
    document_fields(decoded.document().as_ref())
}

fn document_fields(document: Option<&Document>) -> ContenderDocumentFields {
    let Some(document) = document else {
        return ContenderDocumentFields::default();
    };
    ContenderDocumentFields {
        label: document
            .get("label")
            .and_then(|value| value.as_str())
            .filter(|label| !label.is_empty())
            .map(str::to_string),
        created_at_ms: document.created_at().filter(|created_at| *created_at > 0),
    }
}

/// Real DPNS `domain` documents for tests, serialized the way Platform hands
/// contenders back.
#[cfg(test)]
pub(crate) mod test_support {
    use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dash_sdk::dpp::document::{Document, DocumentV0, DocumentV0Getters};
    use dash_sdk::dpp::identifier::Identifier;
    use dash_sdk::dpp::platform_value::Value;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::dpp::version::PlatformVersion;
    use dash_sdk::dpp::voting::contender_structs::{
        Contender, ContenderV0, ContenderWithSerializedDocument,
    };
    use std::collections::BTreeMap;

    pub(crate) const CREATED_AT_MS: u64 = 1_760_140_800_000;

    pub(crate) fn domain_document(created_at: Option<u64>) -> Document {
        requester_document(7, "pizza", created_at)
    }

    /// A `domain` document for `label`, owned by the identity `[owner; 32]`.
    pub(crate) fn requester_document(owner: u8, label: &str, created_at: Option<u64>) -> Document {
        let owner_id = Identifier::from([owner; 32]);
        let properties = BTreeMap::from([
            (
                "parentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            (
                "normalizedParentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            ("label".to_string(), Value::Text(label.to_string())),
            (
                "normalizedLabel".to_string(),
                Value::Text("p1zza".to_string()),
            ),
            ("preorderSalt".to_string(), Value::Bytes32([3u8; 32])),
            (
                "records".to_string(),
                Value::Map(vec![(
                    Value::Text("identity".to_string()),
                    Value::Identifier(owner_id.to_buffer()),
                )]),
            ),
            (
                "subdomainRules".to_string(),
                Value::Map(vec![(
                    Value::Text("allowSubdomains".to_string()),
                    Value::Bool(false),
                )]),
            ),
        ]);
        Document::V0(DocumentV0 {
            id: Identifier::from([owner.wrapping_add(100); 32]),
            owner_id,
            properties,
            revision: Some(1),
            created_at,
            updated_at: created_at,
            transferred_at: created_at,
            ..Default::default()
        })
    }

    /// Serialize `document` the way Platform hands contenders back.
    pub(crate) fn serialized_contender(document: Document) -> ContenderWithSerializedDocument {
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("DPNS system contract");
        let document_type = contract.document_type_for_name("domain").expect("domain");
        Contender::V0(ContenderV0 {
            identity_id: document.owner_id(),
            document: Some(document),
            vote_tally: Some(3),
        })
        .try_into_contender_with_serialized_document(document_type, &contract, platform_version)
        .expect("serialize contender")
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{domain_document, serialized_contender, CREATED_AT_MS};
    use super::*;
    use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dash_sdk::dpp::identifier::Identifier;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::dpp::voting::contender_structs::ContenderWithSerializedDocumentV0;
    use std::ffi::CStr;

    fn fields(contender: &ContenderWithSerializedDocument) -> ContenderDocumentFields {
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("DPNS system contract");
        let document_type = contract.document_type_for_name("domain").ok();
        contender_document_fields(contender, document_type, platform_version)
    }

    #[test]
    fn should_read_label_and_created_at() {
        let contender = serialized_contender(domain_document(Some(CREATED_AT_MS)));
        assert_eq!(
            fields(&contender),
            ContenderDocumentFields {
                label: Some("pizza".to_string()),
                created_at_ms: Some(CREATED_AT_MS),
            }
        );
    }

    #[test]
    fn should_keep_the_label_without_created_at() {
        assert_eq!(
            document_fields(Some(&domain_document(None))),
            ContenderDocumentFields {
                label: Some("pizza".to_string()),
                created_at_ms: None,
            }
        );
    }

    #[test]
    fn should_treat_a_zero_created_at_as_absent() {
        assert_eq!(
            document_fields(Some(&domain_document(Some(0)))).created_at_ms,
            None
        );
    }

    #[test]
    fn should_give_no_fields_for_an_undecodable_document() {
        let contender = ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
            identity_id: Identifier::from([7u8; 32]),
            serialized_document: Some(vec![0xde, 0xad, 0xbe, 0xef]),
            vote_tally: Some(1),
        });
        assert_eq!(fields(&contender), ContenderDocumentFields::default());
    }

    #[test]
    fn should_give_no_fields_without_a_document() {
        let contender = ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
            identity_id: Identifier::from([7u8; 32]),
            serialized_document: None,
            vote_tally: Some(1),
        });
        assert_eq!(fields(&contender), ContenderDocumentFields::default());
    }

    #[test]
    fn should_fill_the_ffi_contender_from_decoded_fields() {
        let fields = ContenderDocumentFields {
            label: Some("pizza".to_string()),
            created_at_ms: Some(CREATED_AT_MS),
        };
        let id = CString::new("Ab1").unwrap().into_raw();
        let contender = fields.into_ffi_contender(id, 4);
        unsafe {
            assert_eq!(contender.vote_count, 4);
            assert!(contender.has_created_at);
            assert_eq!(contender.created_at, CREATED_AT_MS);
            assert_eq!(CStr::from_ptr(contender.label).to_str().unwrap(), "pizza");
            drop(CString::from_raw(contender.identity_id));
            drop(CString::from_raw(contender.label));
        }
    }

    #[test]
    fn should_flag_a_missing_date_in_the_ffi_contender() {
        let id = CString::new("Ab1").unwrap().into_raw();
        let contender = ContenderDocumentFields::default().into_ffi_contender(id, 0);
        assert!(!contender.has_created_at);
        assert_eq!(contender.created_at, 0);
        assert!(contender.label.is_null());
        unsafe { drop(CString::from_raw(contender.identity_id)) };
    }

    #[test]
    fn should_write_label_and_created_at_json_members() {
        let fields = ContenderDocumentFields {
            label: Some("pi\"zza".to_string()),
            created_at_ms: Some(CREATED_AT_MS),
        };
        assert_eq!(
            fields.json_members(),
            format!(r#","label":"pi\"zza","created_at":{}"#, CREATED_AT_MS)
        );
    }

    #[test]
    fn should_leave_out_json_members_that_did_not_decode() {
        assert_eq!(ContenderDocumentFields::default().json_members(), "");
    }

    #[test]
    fn should_give_no_fields_without_a_document_type() {
        let contender = serialized_contender(domain_document(Some(CREATED_AT_MS)));
        assert_eq!(
            contender_document_fields(&contender, None, PlatformVersion::latest()),
            ContenderDocumentFields::default()
        );
    }
}
