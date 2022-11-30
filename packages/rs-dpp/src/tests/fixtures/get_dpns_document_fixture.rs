use getrandom::getrandom;
use serde_json::json;

use crate::{
    document::{document_factory::DocumentFactory, Document},
    mocks,
    prelude::Identifier,
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

pub fn get_dpns_parent_document_fixture(options: ParentDocumentOptions) -> Document {
    let document_factory = DocumentFactory::new(
        LATEST_VERSION,
        get_document_validator_fixture(),
        mocks::FetchAndValidateDataContract {},
    );
    let data_contract = get_dpns_data_contract_fixture(Some(options.owner_id.clone()));
    let mut pre_order_salt = [0u8; 32];
    let _ = getrandom(&mut pre_order_salt);

    let data = json!({
        "label" : options.label,
        "normalizedLabel" : options.normalized_label,
        "normalizedParentDomainName" : "",
        "preorderSalt" : pre_order_salt,
        "records"  : {
            "dashUniqueIdentityId" : options.owner_id.as_bytes(),
        },
        "subdomainRules" : {
            "allowSubdomains" : true
        }
    });

    document_factory
        .create(
            data_contract,
            options.owner_id,
            String::from("domain"),
            data,
        )
        .expect("DPNS document should be created")
}
