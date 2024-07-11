use dash_sdk::{mock::Mockable, platform::Query, Sdk};
use dpp::{data_contract::DataContractFactory, prelude::Identifier};
use hex::ToHex;
use rs_dapi_client::transport::TransportRequest;

use super::config::Config;

/// Test DPNS name for testing of the Sdk; at least 3 identities should request this name to be reserved
pub(crate) const TEST_DPNS_NAME: &str = "testname";

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
            "info,dash_sdk=trace,dash_sdk::platform::fetch=debug,drive_proof_verifier=debug,main=debug,h2=info",
        ))
        .pretty()
        .with_ansi(true)
        .with_writer(std::io::stdout)
        .try_init()
        .ok();
}

/// Configure test case generated with [::test_case] crate.
///
/// This function is intended to use with multiple test cases in a single function.
/// As a test case shares function body, we need to generate unique name for each of them to isolate generated
/// test vectors. It is done by hashing query and using it as a suffix for test case name.
///
/// ## Returns
///
/// Returns unique name of test case (generated from `name_prefix` and hash of query) and configured SDK.
pub(crate) async fn setup_sdk_for_test_case<T: TransportRequest + Mockable, Q: Query<T>>(
    cfg: Config,
    query: Q,
    name_prefix: &str,
) -> (String, Sdk) {
    let key = rs_dapi_client::mock::Key::new(&query.query(true).expect("valid query"));
    let test_case_id = format!("{}_{}", name_prefix, key.encode_hex::<String>());

    // create new sdk to ensure that test cases don't interfere with each other
    (test_case_id.clone(), cfg.setup_api(&test_case_id).await)
}
