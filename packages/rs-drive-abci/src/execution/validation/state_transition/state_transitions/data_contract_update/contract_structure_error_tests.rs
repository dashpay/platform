//! Updating a contract with a new document type that breaks a structural rule of the parser,
//! through `check_tx` and block processing.

use crate::execution::validation::state_transition::state_transitions::data_contract_common::contract_structure_test_harness::{
    assert_paid_contract_structure_error, assert_unpaid_internal_error, check_and_process,
    resign_with_schemas, summed_u64_schema, terminal_without_index_only_schema, Outcome,
    SUMMED_U64_MESSAGE, TERMINAL_WITHOUT_INDEX_ONLY_MESSAGE,
};
use crate::execution::validation::state_transition::state_transitions::tests::setup_identity;
use crate::test::helpers::setup::TestPlatformBuilder;
use dpp::block::block_info::BlockInfo;
use dpp::dash_to_credits;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::platform_value::Value;
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use drive::util::storage_flags::StorageFlags;
use platform_version::version::{PlatformVersion, ProtocolVersion};

/// The contract nonce the update is signed with.
const UPDATE_NONCE: u64 = 2;

/// Updates a stored contract, adding a document type `item` with `schema`.
async fn update_contract_adding_schema(
    schema: Value,
    protocol_version: ProtocolVersion,
) -> Outcome {
    let platform_version =
        PlatformVersion::get(protocol_version).expect("expected the protocol version");
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();

    let (identity, signer, key) = setup_identity(&mut platform, 5078, dash_to_credits!(1.0));

    let mut data_contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    data_contract.set_owner_id(identity.id());
    platform
        .drive
        .apply_contract(
            &data_contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("expected to apply the contract");

    let mut updated_data_contract = data_contract.clone();
    updated_data_contract.set_version(2);

    let mut state_transition = DataContractUpdateTransition::new_from_data_contract(
        updated_data_contract,
        &identity.clone().into_partial_identity_info(),
        key.id(),
        UPDATE_NONCE,
        0,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected to create the contract update transition");
    let transition_bytes = resign_with_schemas(
        &mut state_transition,
        |schemas| {
            schemas.insert("item".to_string(), schema);
        },
        &key,
        &signer,
    )
    .await;

    let owner_id = identity.id();
    let contract_id = data_contract.id();
    check_and_process(
        &platform,
        owner_id,
        transition_bytes,
        |platform| {
            platform
                .drive
                .fetch_identity_contract_nonce(
                    owner_id.to_buffer(),
                    contract_id.to_buffer(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to fetch the contract nonce")
        },
        platform_version,
    )
}

#[tokio::test]
async fn should_refuse_an_update_adding_a_summed_u64_property_with_a_paid_consensus_error() {
    let outcome = update_contract_adding_schema(
        summed_u64_schema(),
        PlatformVersion::latest().protocol_version,
    )
    .await;

    assert_eq!(outcome.nonce_before, None);
    assert_paid_contract_structure_error(&outcome, SUMMED_U64_MESSAGE, UPDATE_NONCE);
}

#[tokio::test]
async fn should_refuse_an_update_adding_a_terminal_outside_an_index_only_type_with_a_paid_consensus_error(
) {
    let outcome = update_contract_adding_schema(
        terminal_without_index_only_schema(),
        PlatformVersion::latest().protocol_version,
    )
    .await;

    assert_eq!(outcome.nonce_before, None);
    assert_paid_contract_structure_error(
        &outcome,
        TERMINAL_WITHOUT_INDEX_ONLY_MESSAGE,
        UPDATE_NONCE,
    );
}

#[tokio::test]
async fn should_keep_refusing_an_update_adding_a_summed_u64_property_unpaid_at_protocol_version_13()
{
    let outcome = update_contract_adding_schema(summed_u64_schema(), 13).await;

    assert_unpaid_internal_error(&outcome, SUMMED_U64_MESSAGE);
}
