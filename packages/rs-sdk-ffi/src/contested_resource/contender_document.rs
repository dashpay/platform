//! The C and JSON shapes of a contender's request.
//!
//! Every contender of a contested name carries the document it submitted.
//! The SDK decodes it into a [`ContenderRequest`] (the requester's own
//! spelling and the time of the request); this module only marshals that
//! result for the list queries and the vote-state query.

use crate::types::DashSDKContender;
use dash_sdk::platform::dpns_usernames::ContenderRequest;
use std::ffi::CString;
use std::os::raw::c_char;

/// The C contender the list queries hand out. Takes ownership of
/// `identity_id`; the label is allocated here. Both are released by
/// `free_contender_fields`, which every contender destructor goes through.
pub(crate) fn ffi_contender(
    request: ContenderRequest,
    identity_id: *mut c_char,
    vote_count: u32,
) -> DashSDKContender {
    DashSDKContender {
        identity_id,
        vote_count,
        label: request
            .label
            .and_then(|label| CString::new(label).ok())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
        created_at: request.created_at.unwrap_or(0),
        has_created_at: request.created_at.is_some(),
    }
}

/// The optional members of a vote-state contender object, each with its
/// leading comma: `,"label":"pizza","created_at":1760140800000`. A field
/// that did not decode is left out, never written as null.
pub(crate) fn json_members(request: &ContenderRequest) -> String {
    let label = request
        .label
        .as_ref()
        .and_then(|label| serde_json::to_string(label).ok())
        .map(|label| format!(r#","label":{}"#, label))
        .unwrap_or_default();
    let created_at = request
        .created_at
        .map(|created_at| format!(r#","created_at":{}"#, created_at))
        .unwrap_or_default();
    label + &created_at
}

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
    use super::test_support::CREATED_AT_MS;
    use super::*;
    use std::ffi::CStr;

    fn request(label: Option<&str>, created_at: Option<u64>) -> ContenderRequest {
        ContenderRequest {
            label: label.map(str::to_string),
            created_at,
        }
    }

    #[test]
    fn should_fill_the_ffi_contender_from_the_request() {
        let id = CString::new("Ab1").unwrap().into_raw();
        let contender = ffi_contender(request(Some("pizza"), Some(CREATED_AT_MS)), id, 4);
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
        let contender = ffi_contender(ContenderRequest::default(), id, 0);
        assert!(!contender.has_created_at);
        assert_eq!(contender.created_at, 0);
        assert!(contender.label.is_null());
        unsafe { drop(CString::from_raw(contender.identity_id)) };
    }

    #[test]
    fn should_keep_a_present_zero_date_in_the_ffi_contender() {
        let id = CString::new("Ab1").unwrap().into_raw();
        let contender = ffi_contender(request(None, Some(0)), id, 0);
        assert!(contender.has_created_at);
        assert_eq!(contender.created_at, 0);
        unsafe { drop(CString::from_raw(contender.identity_id)) };
    }

    #[test]
    fn should_write_label_and_created_at_json_members() {
        assert_eq!(
            json_members(&request(Some("pi\"zza"), Some(CREATED_AT_MS))),
            format!(r#","label":"pi\"zza","created_at":{}"#, CREATED_AT_MS)
        );
    }

    #[test]
    fn should_write_a_present_zero_created_at_json_member() {
        assert_eq!(json_members(&request(None, Some(0))), r#","created_at":0"#);
    }

    #[test]
    fn should_leave_out_json_members_that_did_not_decode() {
        assert_eq!(json_members(&ContenderRequest::default()), "");
    }
}
