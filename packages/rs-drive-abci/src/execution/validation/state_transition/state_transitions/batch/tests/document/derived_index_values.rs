//! Derived index properties (protocol version 14): the reference validation of a create
//! records the value of each derived index property from the document it fetched, which the
//! create action carries to Drive, so the new document is keyed without a second read.

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::DocumentReferenceValidation;
use crate::platform_types::platform::PlatformStateRef;
use crate::test::helpers::setup::TestPlatformBuilder;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContractFactory;
use dpp::document::{Document, DocumentV0};
use dpp::identifier::Identifier;
use dpp::platform_value::{platform_value, Value};
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::util::storage_flags::StorageFlags;
use std::collections::BTreeMap;

const POSTER: [u8; 32] = [7; 32];
const POST: [u8; 32] = [1; 32];

#[test]
fn should_record_the_derived_values_the_reference_validation_read() {
    let platform_version = PlatformVersion::latest();
    let platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_genesis_state();
    let schemas = platform_value!({
        "post": {
            "type": "object",
            "documentsMutable": false,
            "canBeDeleted": false,
            "properties": {
                "text": { "type": "string", "maxLength": 50, "position": 0 }
            },
            "additionalProperties": false
        },
        "reply": {
            "type": "object",
            "documentsMutable": false,
            "canBeDeleted": true,
            "indices": [{ "name": "toPostOwner", "properties": [{ "postId.$ownerId": "asc" }] }],
            "properties": {
                "postId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "refersTo": { "type": "permanentDocument", "documentType": "post" },
                    "position": 0
                }
            },
            "required": ["postId"],
            "additionalProperties": false
        }
    });
    let contract = DataContractFactory::new(platform_version.protocol_version)
        .expect("factory")
        .create_with_value_config(Identifier::from([202u8; 32]), 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned();
    platform
        .drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("expected to apply the contract");
    let post = Document::V0(DocumentV0 {
        id: Identifier::from(POST),
        owner_id: Identifier::from(POSTER),
        ..Default::default()
    });
    platform
        .drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &post,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(POSTER),
                },
                contract: &contract,
                document_type: contract.document_type_for_name("post").expect("post"),
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("expected to insert the post");

    let (_, contract_fetch_info) = platform
        .drive
        .get_contract_with_fetch_info_and_fee(
            contract.id().to_buffer(),
            None,
            false,
            None,
            platform_version,
        )
        .expect("expected to fetch the contract");
    let base = DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
        id: Identifier::from([0xAB; 32]),
        identity_contract_nonce: 1,
        document_type_name: "reply".to_string(),
        data_contract: contract_fetch_info.expect("the contract is in state"),
        token_cost: None,
        shielded_token_payment: None,
        gas_fees_paid_by: GasFeesPaidBy::default(),
        contract_gas_fees_paid_by: GasFeesPaidBy::default(),
        declared_action_fee: None,
    });
    let platform_state = platform.state.load();
    let platform_ref = PlatformStateRef {
        drive: &platform.drive,
        state: &platform_state,
        config: &platform.config,
    };
    let mut execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)
            .expect("expected an execution context");
    let replier = Identifier::from([9; 32]);
    let data = BTreeMap::from([("postId".to_string(), Value::Identifier(POST))]);
    let mut derived_index_values = BTreeMap::new();
    let result = base
        .validate_document_references(
            &data,
            replier,
            Some(replier),
            None,
            None,
            &platform_ref,
            &BlockInfo::default(),
            &mut Vec::new(),
            Some(&mut derived_index_values),
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("expected the references to be validated");
    assert!(result.is_valid(), "{:?}", result.errors);
    assert_eq!(
        derived_index_values,
        BTreeMap::from([("postId.$ownerId".to_string(), Value::Identifier(POSTER))])
    );
}
