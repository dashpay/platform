use crate::document::ExtendedDocument;
use crate::tests::fixtures::ParentDocumentOptions;
use crate::util::hash::hash;
use crate::{document::document_factory::DocumentFactory, version::LATEST_VERSION};
use getrandom::getrandom;
use platform_value::Value;
use std::collections::BTreeMap;

use super::get_dpns_data_contract_fixture;

#[cfg(feature = "extended-document")]
pub fn get_dpns_preorder_document_fixture(
    options: ParentDocumentOptions,
    protocol_version: u32,
) -> (ExtendedDocument, [u8; 32]) {
    let data_contract = get_dpns_data_contract_fixture(Some(options.owner_id), protocol_version);
    let document_factory =
        DocumentFactory::new(LATEST_VERSION, data_contract.data_contract_owned()).expect("expected to get document factory");
    let mut pre_order_salt = [0u8; 32];
    let _ = getrandom(&mut pre_order_salt);

    let salted_domain_hash = hash(&[options.label.as_bytes(), &pre_order_salt].concat());

    let mut map = BTreeMap::new();
    map.insert(
        "saltedDomainHash".to_string(),
        Value::Bytes32(salted_domain_hash),
    );

    (
        document_factory
            .create_extended_document(
                options.owner_id,
                String::from("preorder"),
                map.into(),
            )
            .expect("DPNS preorder document should be created"),
        pre_order_salt,
    )
}
