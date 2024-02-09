use rand::rngs::StdRng;
use rand::SeedableRng;

use platform_value::{platform_value, Value};

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::document::document_factory::DocumentFactory;
use crate::document::Document;
use crate::version::PlatformVersion;
use crate::{prelude::*, tests::utils::generate_random_identifier_struct as gen_owner_id};

#[cfg(feature = "extended-document")]
pub fn get_extended_documents_fixture_with_owner_id_from_contract(
    data_contract: &DataContract,
    protocol_version: u32,
) -> Result<Vec<ExtendedDocument>, ProtocolError> {
    let owner_id = data_contract.owner_id();
    let factory = DocumentFactory::new(protocol_version)?;

    get_extended_documents(factory, data_contract, owner_id)
}

pub fn get_documents_fixture_with_owner_id_from_contract(
    data_contract: &DataContract,
    protocol_version: u32,
) -> Result<Vec<Document>, ProtocolError> {
    let owner_id = data_contract.owner_id();
    let factory = DocumentFactory::new(protocol_version)?;

    get_documents(factory, data_contract, owner_id)
}

pub fn get_documents_fixture(
    data_contract: &DataContract,
    protocol_version: u32,
) -> Result<Vec<Document>, ProtocolError> {
    let factory = DocumentFactory::new(protocol_version)?;
    let owner_id = gen_owner_id();

    get_documents(factory, data_contract, owner_id)
}

#[cfg(feature = "extended-document")]
pub fn get_extended_documents_fixture(
    data_contract: &DataContract,
    protocol_version: u32,
) -> Result<Vec<ExtendedDocument>, ProtocolError> {
    let factory = DocumentFactory::new(protocol_version)?;
    let owner_id = gen_owner_id();

    get_extended_documents(factory, data_contract, owner_id)
}

fn get_documents(
    factory: DocumentFactory,
    data_contract: &DataContract,
    owner_id: Identifier,
) -> Result<Vec<Document>, ProtocolError> {
    let documents = vec![
        factory.create_document(
            data_contract,
            owner_id,
            "niceDocument".to_string(),
            platform_value!({ "name": "Cutie" }),
        )?,
        factory.create_document(
            data_contract,
            owner_id,
            "prettyDocument".to_string(),
            platform_value!({ "lastName": "Shiny" }),
        )?,
        factory.create_document(
            data_contract,
            owner_id,
            "prettyDocument".to_string(),
            platform_value!({ "lastName": "Sweety" }),
        )?,
        factory.create_document(
            data_contract,
            owner_id,
            "indexedDocument".to_string(),
            platform_value!( { "firstName": "William", "lastName": "Birkin" }),
        )?,
        factory.create_document(
            data_contract,
            owner_id,
            "indexedDocument".to_string(),
            platform_value!( { "firstName": "Leon", "lastName": "Kennedy" }),
        )?,
        factory.create_document(
            data_contract,
            owner_id,
            "noTimeDocument".to_string(),
            platform_value!({ "name": "ImOutOfTime" }),
        )?,
        factory.create_document(
            data_contract,
            owner_id,
            "uniqueDates".to_string(),
            platform_value!({ "firstName": "John" }),
        )?,
        factory.create_document(
            data_contract,
            owner_id,
            "indexedDocument".to_string(),
            platform_value!( { "firstName": "Bill", "lastName": "Gates" }),
        )?,
        factory.create_document(data_contract, owner_id, "withByteArrays".to_string(), platform_value!( { "byteArrayField": get_random_10_bytes(), "identifierField": gen_owner_id().to_buffer() }))?,
        factory.create_document(
            data_contract,
            owner_id,
            "optionalUniqueIndexedDocument".to_string(),
            platform_value!({ "firstName": "Jacques-Yves", "lastName": "Cousteau" })
        )?,
    ];

    Ok(documents)
}

#[cfg(feature = "extended-document")]
fn get_extended_documents(
    factory: DocumentFactory,
    data_contract: &DataContract,
    owner_id: Identifier,
) -> Result<Vec<ExtendedDocument>, ProtocolError> {
    let documents = vec![
        factory.create_extended_document(
            data_contract,
            owner_id,
            "niceDocument".to_string(),
            platform_value!({ "name": "Cutie" }),
        )?,
        factory.create_extended_document(
            data_contract,
            owner_id,
            "prettyDocument".to_string(),
            platform_value!({ "lastName": "Shiny" }),
        )?,
        factory.create_extended_document(
            data_contract,
            owner_id,
            "prettyDocument".to_string(),
            platform_value!({ "lastName": "Sweety" }),
        )?,
        factory.create_extended_document(
            data_contract,
            owner_id,
            "indexedDocument".to_string(),
            platform_value!( { "firstName": "William", "lastName": "Birkin" }),
        )?,
        factory.create_extended_document(
            data_contract,
            owner_id,
            "indexedDocument".to_string(),
            platform_value!( { "firstName": "Leon", "lastName": "Kennedy" }),
        )?,
        factory.create_extended_document(
            data_contract,
            owner_id,
            "noTimeDocument".to_string(),
            platform_value!({ "name": "ImOutOfTime" }),
        )?,
        factory.create_extended_document(
            data_contract,
            owner_id,
            "uniqueDates".to_string(),
            platform_value!({ "firstName": "John" }),
        )?,
        factory.create_extended_document(
            data_contract,
            owner_id,
            "indexedDocument".to_string(),
            platform_value!( { "firstName": "Bill", "lastName": "Gates" }),
        )?,
        factory.create_extended_document(data_contract, owner_id, "withByteArrays".to_string(), platform_value!( { "byteArrayField": get_random_10_bytes(), "identifierField": gen_owner_id().to_buffer() }))?,
        factory.create_extended_document(
            data_contract,
            owner_id,
            "optionalUniqueIndexedDocument".to_string(),
            platform_value!({ "firstName": "Jacques-Yves", "lastName": "Cousteau" })
        )?,
    ];

    Ok(documents)
}

pub fn get_withdrawal_document_fixture(
    data_contract: &DataContract,
    owner_id: Identifier,
    data: Value,
    seed: Option<u64>,
    protocol_version: u32,
) -> Result<Document, ProtocolError> {
    let mut rng = match seed {
        None => StdRng::from_entropy(),
        Some(seed_value) => StdRng::seed_from_u64(seed_value),
    };

    let document_type = data_contract.document_type_for_name(
        data_contracts::withdrawals_contract::v1::document_types::withdrawal::NAME,
    )?;

    let properties = data
        .into_btree_string_map()
        .map_err(ProtocolError::ValueError)?;

    let id = Identifier::random_with_rng(&mut rng);

    let platform_version = PlatformVersion::get(protocol_version)?;
    document_type.create_document_with_prevalidated_properties(
        id,
        owner_id,
        properties,
        platform_version,
    )
}

fn get_random_10_bytes() -> Vec<u8> {
    let mut buffer = [0u8; 10];
    let _ = getrandom::getrandom(&mut buffer);
    buffer.to_vec()
}
