use dpp::{data_contract::DataContractFactory, prelude::Identifier};

/// Create a mock document type for testing of mock API
pub fn mock_document_type() -> dpp::data_contract::document_type::DocumentType {
    use dpp::{
        data_contract::document_type::DocumentType,
        platform_value::platform_value,
        version::{PlatformVersion, PlatformVersionCurrentVersion},
    };

    let platform_version = PlatformVersion::get_current().unwrap();

    let schema = platform_value!({
        "type": "object",
        "properties": {
            "a": {
                "type": "string",
                "maxLength": 10,
                "position": 0
            }
        },
        "additionalProperties": false,
    });

    DocumentType::try_from_schema(
        Identifier::random(),
        "document_type_name",
        schema,
        None,
        false,
        false,
        false,
        true,
        &mut vec![],
        platform_version,
    )
    .expect("expected to create a document type")
}

/// Create a mock data contract for testing of mock API
pub fn mock_data_contract(
    document_type: Option<&dpp::data_contract::document_type::DocumentType>,
) -> dpp::prelude::DataContract {
    use dpp::{
        data_contract::document_type::accessors::DocumentTypeV0Getters,
        platform_value::{platform_value, Value},
        version::PlatformVersion,
    };
    use std::collections::BTreeMap;

    let platform_version = PlatformVersion::latest();
    let protocol_version = platform_version.protocol_version;

    // let owner_id = Identifier::from_bytes(&IDENTITY_ID_BYTES).unwrap();
    let owner_id = Identifier::random();

    let mut document_types: BTreeMap<String, Value> = BTreeMap::new();

    if let Some(doc) = document_type {
        let schema = doc.schema();
        document_types.insert(doc.name().to_string(), schema.clone());
    }

    DataContractFactory::new(protocol_version)
        .unwrap()
        .create(owner_id, 0, platform_value!(document_types), None, None)
        .expect("create data contract")
        .data_contract_owned()
}

/// Enable logging for tests
pub fn setup_logs() {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            "info,dash_sdk=trace,h2=info",
        ))
        .pretty()
        .with_ansi(true)
        .with_writer(std::io::stdout)
        .try_init()
        .ok();
}
