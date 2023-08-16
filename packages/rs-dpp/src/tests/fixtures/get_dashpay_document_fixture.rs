use platform_value::patch::merge;
use platform_value::{platform_value, BinaryData, Value};

use crate::document::document_factory::DocumentFactory;
use crate::document::Document;
use crate::{prelude::Identifier, tests::utils::generate_random_identifier_struct};

use super::get_dashpay_contract_fixture;

#[cfg(feature = "extended-document")]
use crate::document::ExtendedDocument;

pub fn get_contact_request_document_fixture(
    owner_id: Option<Identifier>,
    additional_data: Option<Value>,
    protocol_version: u32,
) -> Document {
    let owner_id = owner_id.unwrap_or_else(generate_random_identifier_struct);
    let data_contract = get_dashpay_contract_fixture(None, protocol_version).data_contract_owned();

    let factory = DocumentFactory::new(protocol_version, data_contract)
        .expect("expected to create factory for get_contact_request_document_fixture");

    let mut data = platform_value! ({
            "toUserId": Identifier::new([0_u8;32]),
            "encryptedPublicKey": BinaryData::new([0_u8;96].to_vec()),
            "senderKeyIndex": 0u32,
            "recipientKeyIndex": 0u32,
            "accountReference": 0u32,
    });

    if let Some(additional_data) = additional_data {
        merge(&mut data, &additional_data)
    }

    factory
        .create_document(owner_id, "contactRequest".to_string(), data)
        .expect("the document dashpay contact request should be created")
}

#[cfg(feature = "extended-document")]
pub fn get_contact_request_extended_document_fixture(
    owner_id: Option<Identifier>,
    additional_data: Option<Value>,
    protocol_version: u32,
) -> ExtendedDocument {
    let owner_id = owner_id.unwrap_or_else(generate_random_identifier_struct);
    let data_contract = get_dashpay_contract_fixture(None, protocol_version).data_contract_owned();

    let factory =
        DocumentFactory::new(protocol_version, data_contract).expect("expected document factory");

    let mut data = platform_value! ({
            "toUserId": Identifier::new([0_u8;32]),
            "encryptedPublicKey": BinaryData::new([0_u8;96].to_vec()),
            "senderKeyIndex": 0u32,
            "recipientKeyIndex": 0u32,
            "accountReference": 0u32,
    });

    if let Some(additional_data) = additional_data {
        merge(&mut data, &additional_data)
    }

    factory
        .create_extended_document(owner_id, "contactRequest".to_string(), data)
        .expect("the document dashpay contact request should be created")
}
