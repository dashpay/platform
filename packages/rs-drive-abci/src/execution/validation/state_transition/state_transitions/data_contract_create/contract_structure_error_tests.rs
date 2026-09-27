//! Registering a contract whose document type breaks a structural rule of the parser, through
//! `check_tx` and block processing.

use crate::execution::validation::state_transition::state_transitions::data_contract_common::contract_structure_test_harness::{
    assert_paid_contract_structure_error, assert_unpaid_internal_error, check_and_process,
    resign_with_schemas, summed_u64_schema, terminal_without_index_only_schema, Outcome,
    SUMMED_U64_MESSAGE, TERMINAL_WITHOUT_INDEX_ONLY_MESSAGE,
};
use crate::execution::validation::state_transition::state_transitions::tests::setup_identity;
use crate::test::helpers::setup::TestPlatformBuilder;
use dpp::dash_to_credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::platform_value::Value;
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use platform_version::version::{PlatformVersion, ProtocolVersion};

/// Registers a contract whose only document type is `item`, with `schema`.
async fn register_contract_with_schema(
    schema: Value,
    protocol_version: ProtocolVersion,
) -> Outcome {
    let platform_version =
        PlatformVersion::get(protocol_version).expect("expected the protocol version");
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();

    let (identity, signer, key) = setup_identity(&mut platform, 5077, dash_to_credits!(1.0));

    let data_contract =
        get_data_contract_fixture(Some(identity.id()), 1, platform_version.protocol_version)
            .data_contract_owned();

    let mut state_transition = DataContractCreateTransition::new_from_data_contract(
        data_contract,
        1,
        &identity.clone().into_partial_identity_info(),
        key.id(),
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected to create the contract create transition");
    let transition_bytes = resign_with_schemas(
        &mut state_transition,
        |schemas| {
            schemas.clear();
            schemas.insert("item".to_string(), schema);
        },
        &key,
        &signer,
    )
    .await;

    let owner_id = identity.id();
    check_and_process(
        &platform,
        owner_id,
        transition_bytes,
        |platform| {
            platform
                .drive
                .fetch_identity_nonce(owner_id.to_buffer(), true, None, platform_version)
                .expect("expected to fetch the identity nonce")
        },
        platform_version,
    )
}

#[tokio::test]
async fn should_refuse_a_summed_u64_property_with_a_paid_consensus_error() {
    let outcome = register_contract_with_schema(
        summed_u64_schema(),
        PlatformVersion::latest().protocol_version,
    )
    .await;

    assert_eq!(outcome.nonce_before, Some(0));
    assert_paid_contract_structure_error(&outcome, SUMMED_U64_MESSAGE, 1);
}

#[tokio::test]
async fn should_refuse_a_terminal_outside_an_index_only_type_with_a_paid_consensus_error() {
    let outcome = register_contract_with_schema(
        terminal_without_index_only_schema(),
        PlatformVersion::latest().protocol_version,
    )
    .await;

    assert_eq!(outcome.nonce_before, Some(0));
    assert_paid_contract_structure_error(&outcome, TERMINAL_WITHOUT_INDEX_ONLY_MESSAGE, 1);
}

#[tokio::test]
async fn should_keep_refusing_a_summed_u64_property_unpaid_at_protocol_version_13() {
    let outcome = register_contract_with_schema(summed_u64_schema(), 13).await;

    assert_unpaid_internal_error(&outcome, SUMMED_U64_MESSAGE);
}
