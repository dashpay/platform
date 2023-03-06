use std::collections::BTreeMap;
use std::sync::Arc;

use getrandom::getrandom;
use platform_value::Value;

use crate::document::ExtendedDocument;
use crate::{
    document::{
        document_factory::DocumentFactory,
        fetch_and_validate_data_contract::DataContractFetcherAndValidator,
    },
    prelude::Identifier,
    state_repository::MockStateRepositoryLike,
    tests::utils::generate_random_identifier_struct,
    version::LATEST_VERSION,
};

use super::{get_document_validator_fixture, get_dpns_data_contract_fixture};

pub struct ParentDocumentOptions {
    pub label: String,
    pub normalized_label: String,
    pub owner_id: Identifier,
}

impl Default for ParentDocumentOptions {
    fn default() -> Self {
        Self {
            label: String::from("Parent"),
            normalized_label: String::from("parent"),
            owner_id: generate_random_identifier_struct(),
        }
    }
}

pub fn get_dpns_parent_document_fixture(options: ParentDocumentOptions) -> ExtendedDocument {
    let document_factory = DocumentFactory::new(
        LATEST_VERSION,
        get_document_validator_fixture(),
        DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
        None,
    );
    let data_contract = get_dpns_data_contract_fixture(Some(options.owner_id));
    let mut pre_order_salt = [0u8; 32];
    let _ = getrandom(&mut pre_order_salt);

    let mut map = BTreeMap::new();
    map.insert("label".to_string(), Value::Text(options.label));
    map.insert(
        "normalizedLabel".to_string(),
        Value::Text(options.normalized_label),
    );
    map.insert(
        "normalizedParentDomainName".to_string(),
        Value::Text(String::new()),
    );
    map.insert("preorderSalt".to_string(), Value::Bytes32(pre_order_salt));
    map.insert(
        "records".to_string(),
        Value::Map(vec![(
            Value::Text("dashUniqueIdentityId".to_string()),
            Value::Identifier(options.owner_id.buffer),
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
        .create_document_for_state_transition(
            data_contract,
            options.owner_id,
            String::from("domain"),
            map.into(),
        )
        .expect("DPNS document should be created")
}
