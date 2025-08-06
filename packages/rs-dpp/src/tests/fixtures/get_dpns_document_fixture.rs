use std::collections::BTreeMap;

use platform_value::{Identifier, Value};

use crate::document::document_factory::DocumentFactory;
use crate::document::Document;
use crate::tests::utils::generate_random_identifier_struct;

use super::get_dpns_data_contract_fixture;

#[cfg(feature = "extended-document")]
use crate::document::ExtendedDocument;
use crate::prelude::IdentityNonce;
use crate::util::strings::convert_to_homograph_safe_chars;

pub struct ParentDocumentOptions {
    pub label: String,
    pub owner_id: Identifier,
    pub identity_nonce: IdentityNonce,
}

impl Default for ParentDocumentOptions {
    fn default() -> Self {
        Self {
            label: String::from("Parent"),
            owner_id: generate_random_identifier_struct(),
            identity_nonce: 0,
        }
    }
}

pub fn get_dpns_parent_document_fixture(
    options: ParentDocumentOptions,
    protocol_version: u32,
) -> Document {
    let data_contract = get_dpns_data_contract_fixture(
        Some(options.owner_id),
        options.identity_nonce,
        protocol_version,
    );
    let document_factory =
        DocumentFactory::new(protocol_version).expect("expected to get document factory");
    let mut pre_order_salt = [0u8; 32];
    let _ = getrandom::fill(&mut pre_order_salt);

    let normalized_label = convert_to_homograph_safe_chars(options.label.as_str());

    let mut map = BTreeMap::new();
    map.insert("label".to_string(), Value::Text(options.label));
    map.insert("normalizedLabel".to_string(), Value::Text(normalized_label));

    map.insert("parentDomainName".to_string(), Value::Text(String::new()));
    map.insert(
        "normalizedParentDomainName".to_string(),
        Value::Text(String::new()),
    );
    map.insert("preorderSalt".to_string(), Value::Bytes32(pre_order_salt));
    map.insert(
        "records".to_string(),
        Value::Map(vec![(
            Value::Text("dashUniqueIdentityId".to_string()),
            Value::Identifier(options.owner_id.to_buffer()),
        )]),
    );
    map.insert(
        "subdomainRules".to_string(),
        Value::Map(vec![(
            Value::Text("allowSubdomains".to_string()),
            Value::Bool(true),
        )]),
    );

    document_factory
        .create_document(
            data_contract.data_contract(),
            options.owner_id,
            String::from("domain"),
            map.into(),
        )
        .expect("DPNS document should be created")
}

#[cfg(feature = "extended-document")]
pub fn get_dpns_parent_extended_document_fixture(
    options: ParentDocumentOptions,
    protocol_version: u32,
) -> ExtendedDocument {
    let data_contract = get_dpns_data_contract_fixture(
        Some(options.owner_id),
        options.identity_nonce,
        protocol_version,
    );
    let document_factory =
        DocumentFactory::new(protocol_version).expect("expected to get document factory");
    let mut pre_order_salt = [0u8; 32];
    let _ = getrandom::fill(&mut pre_order_salt);

    let normalized_label = convert_to_homograph_safe_chars(options.label.as_str());

    let mut map = BTreeMap::new();
    map.insert("label".to_string(), Value::Text(options.label));
    map.insert("normalizedLabel".to_string(), Value::Text(normalized_label));

    map.insert("parentDomainName".to_string(), Value::Text(String::new()));
    map.insert(
        "normalizedParentDomainName".to_string(),
        Value::Text(String::new()),
    );

    map.insert("preorderSalt".to_string(), Value::Bytes32(pre_order_salt));
    map.insert(
        "records".to_string(),
        Value::Map(vec![(
            Value::Text("dashUniqueIdentityId".to_string()),
            Value::Identifier(options.owner_id.to_buffer()),
        )]),
    );
    map.insert(
        "subdomainRules".to_string(),
        Value::Map(vec![(
            Value::Text("allowSubdomains".to_string()),
            Value::Bool(true),
        )]),
    );

    document_factory
        .create_extended_document(
            data_contract.data_contract(),
            options.owner_id,
            String::from("domain"),
            map.into(),
        )
        .expect("DPNS document should be created")
}
