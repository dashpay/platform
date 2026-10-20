use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::platform::types::epoch::EpochQuery;
use dash_sdk::platform::LimitQuery;
use dash_sdk::{
    mock::Mockable,
    platform::{Query, QuerySettings},
    Sdk,
};
use dpp::block::extended_epoch_info::v0::{ExtendedEpochInfoV0, ExtendedEpochInfoV0Getters};
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::data_contract::config::DataContractConfig;
use dpp::{data_contract::DataContractFactory, prelude::Identifier};
use drive_proof_verifier::types::ExtendedEpochInfos;
use hex::ToHex;
use rs_dapi_client::transport::TransportRequest;
use std::collections::BTreeMap;
use tracing_subscriber::fmt::writer::{BoxMakeWriter, TestWriter};

use super::config::Config;

/// Test DPNS name for testing of the Sdk; at least 3 identities should request this name to be reserved
pub(crate) const TEST_DPNS_NAME: &str = "testname";

fn should_emit_test_logs_to_stdout() -> bool {
    let step_debug = std::env::var("ACTIONS_STEP_DEBUG")
        .map(|value| value == "true")
        .unwrap_or(false);
    let runner_debug = std::env::var("ACTIONS_RUNNER_DEBUG")
        .map(|value| value == "true")
        .unwrap_or(false);

    step_debug || runner_debug
}

/// Create a mock document type for testing of mock API
pub fn mock_document_type() -> dpp::data_contract::document_type::DocumentType {
    use dpp::{
        data_contract::document_type::DocumentType, platform_value::platform_value,
        version::PlatformVersion,
    };

    // `set_current()` is no longer called by the SDK builder; use `latest()` directly.
    let platform_version = PlatformVersion::latest();

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

    let config =
        DataContractConfig::default_for_version(platform_version).expect("create a default config");

    DocumentType::try_from_schema(
        Identifier::random(),
        1,
        config.version(),
        "document_type_name",
        schema,
        None,
        &BTreeMap::new(),
        &config,
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

/// Ratchet a fresh auto-detect mock SDK from the protocol-version floor up to the
/// network's latest version, exactly as production does on its first proven response.
///
/// An unpinned SDK boots at its per-network `min_protocol_version` (the upgrade-safe
/// floor) and only learns the real network version after a *proven* fetch, when response
/// metadata drives `maybe_update_protocol_version`. Mock tests that need the latest
/// wire (e.g. Count / `group_by`, or V2 document types) must therefore perform one
/// proven fetch before encoding their real request. This registers a cheap proven
/// `ExtendedEpochInfo::fetch_current` expectation and consumes it, leaving the SDK
/// ratcheted to `LATEST_VERSION`.
pub(crate) async fn bootstrap_mock_sdk_to_latest(sdk: &mut Sdk) {
    // `fetch_current` issues two queries: a genesis probe, then a two-epoch
    // ascending confirmation from the hinted current epoch (mock expectation
    // metadata reports epoch 0, so the hint is 0). The confirmation answers with
    // epoch 0 alone, which is how a real proof says "no epoch above 0 has
    // started".
    let probe_query = LimitQuery {
        query: EpochQuery::genesis(),
        limit: Some(1),
        start_info: None,
    };
    let confirmation_query = LimitQuery {
        query: EpochQuery::ascending_from(0),
        limit: Some(2),
        start_info: None,
    };

    let epoch = ExtendedEpochInfo::from(ExtendedEpochInfoV0 {
        index: 0,
        first_block_time: 0,
        first_block_height: 0,
        first_core_block_height: 0,
        fee_multiplier_permille: 0,
        protocol_version: dpp::version::LATEST_VERSION,
    });

    sdk.mock()
        .expect_fetch::<ExtendedEpochInfo, _>(probe_query, Some(epoch.clone()))
        .await
        .expect("register epoch probe expectation");
    sdk.mock()
        .expect_fetch_many::<_, ExtendedEpochInfo, _, ExtendedEpochInfos>(
            confirmation_query,
            Some(ExtendedEpochInfos::from_iter([(0, Some(epoch.clone()))])),
        )
        .await
        .expect("register epoch bootstrap expectation");

    let fetched = ExtendedEpochInfo::fetch_current(sdk)
        .await
        .expect("bootstrap fetch_current should ratchet the SDK to latest");

    assert_eq!(fetched.index(), epoch.index());
    assert_eq!(
        sdk.version().protocol_version,
        dpp::version::LATEST_VERSION,
        "bootstrap must ratchet the auto-detect SDK to the network's latest protocol version"
    );
}

/// Enable logging for tests
pub fn setup_logs() {
    let make_writer = if should_emit_test_logs_to_stdout() {
        BoxMakeWriter::new(std::io::stdout)
    } else {
        BoxMakeWriter::new(TestWriter::new)
    };

    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            "info,dash_sdk=trace,dash_sdk::platform::fetch=debug,drive_proof_verifier=debug,main=debug,h2=info",
        ))
        .pretty()
        .with_ansi(true)
        .with_writer(make_writer)
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
    let request_settings = rs_dapi_client::RequestSettings::default();
    let settings = QuerySettings {
        request_settings: &request_settings,
        protocol_version: dpp::version::PlatformVersion::latest(),
        prove: true,
    };
    let key = rs_dapi_client::mock::Key::new(&query.query(&settings).expect("valid query"));
    let test_case_id = format!("{}_{}", name_prefix, key.encode_hex::<String>());

    // create new sdk to ensure that test cases don't interfere with each other
    (test_case_id.clone(), cfg.setup_api(&test_case_id).await)
}
