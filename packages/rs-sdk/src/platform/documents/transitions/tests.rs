use super::delete::DocumentDeleteTransitionBuilder;
use crate::{Error, Sdk, SdkBuilder};
use dpp::address_funds::AddressWitness;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DataContractFactory;
use dpp::document::{Document, DocumentV0};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{platform_value, BinaryData, Value};
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::ProtocolError;
use drive_proof_verifier::types::IdentityContractNonceFetcher;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug)]
struct TestSigner;

impl Signer<IdentityPublicKey> for TestSigner {
    fn sign(&self, _key: &IdentityPublicKey, _data: &[u8]) -> Result<BinaryData, ProtocolError> {
        Ok(BinaryData::from(vec![1; 65]))
    }

    fn sign_create_witness(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Err(ProtocolError::CorruptedCodeExecution(
            "sign_create_witness is not used in these tests".to_string(),
        ))
    }

    fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
        true
    }
}

fn test_identity_public_key() -> IdentityPublicKey {
    IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: 1,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::CRITICAL,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::from(vec![2; 33]),
        disabled_at: None,
    })
}

fn test_data_contract(document_type_name: &str) -> Arc<dpp::data_contract::DataContract> {
    let platform_version = dpp::version::PlatformVersion::latest();
    let config =
        DataContractConfig::default_for_version(platform_version).expect("create contract config");

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

    let document_type = DocumentType::try_from_schema(
        Identifier::random(),
        1,
        config.version(),
        document_type_name,
        schema,
        None,
        &BTreeMap::new(),
        &config,
        true,
        &mut vec![],
        platform_version,
    )
    .expect("create test document type");

    let mut document_types: BTreeMap<String, Value> = BTreeMap::new();
    document_types.insert(
        document_type.name().to_string(),
        document_type.schema().clone(),
    );

    let contract = DataContractFactory::new(platform_version.protocol_version)
        .expect("create data contract factory")
        .create(
            Identifier::random(),
            0,
            platform_value!(document_types),
            None,
            None,
        )
        .expect("create test data contract")
        .data_contract_owned();

    Arc::new(contract)
}

const TEST_DOCUMENT_TYPE_NAME: &str = "testDoc";
const INVALID_NONCE: u64 = 1_u64 << 50;

fn test_document(owner_id: Identifier) -> Document {
    Document::V0(DocumentV0 {
        id: Identifier::random(),
        owner_id,
        properties: Default::default(),
        revision: Some(1),
        created_at: None,
        updated_at: None,
        transferred_at: None,
        created_at_block_height: None,
        updated_at_block_height: None,
        transferred_at_block_height: None,
        created_at_core_block_height: None,
        updated_at_core_block_height: None,
        transferred_at_core_block_height: None,
        creator_id: None,
    })
}

fn validate_transition_like_builder(state_transition: &StateTransition) -> Result<(), Error> {
    let platform_version = dpp::version::PlatformVersion::latest();
    let validation_result = match state_transition {
        StateTransition::Batch(batch_transition) => {
            batch_transition.validate_base_structure(platform_version)?
        }
        _ => {
            return Err(Error::Protocol(
                dpp::ProtocolError::InvalidStateTransitionType(
                    "expected Batch transition".to_string(),
                ),
            ))
        }
    };
    if let Some(first_error) = validation_result.errors.into_iter().next() {
        return Err(Error::Protocol(dpp::ProtocolError::ConsensusError(
            Box::new(first_error),
        )));
    }
    Ok(())
}

pub(super) fn assert_document_create_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let document_type = data_contract
        .document_type_for_name(TEST_DOCUMENT_TYPE_NAME)
        .expect("expected test document type");
    let transition = BatchTransition::new_document_creation_transition_from_document(
        document,
        document_type,
        [7; 32],
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        None,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_document_delete_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let document_type = data_contract
        .document_type_for_name(TEST_DOCUMENT_TYPE_NAME)
        .expect("expected test document type");
    let transition = BatchTransition::new_document_deletion_transition_from_document(
        document,
        document_type,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        None,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_document_purchase_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let purchaser_id = Identifier::random();
    let document_type = data_contract
        .document_type_for_name(TEST_DOCUMENT_TYPE_NAME)
        .expect("expected test document type");
    let transition = BatchTransition::new_document_purchase_transition_from_document(
        document,
        document_type,
        purchaser_id,
        100,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        None,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_document_replace_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let document_type = data_contract
        .document_type_for_name(TEST_DOCUMENT_TYPE_NAME)
        .expect("expected test document type");
    let transition = BatchTransition::new_document_replacement_transition_from_document(
        document,
        document_type,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        None,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_document_set_price_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let document_type = data_contract
        .document_type_for_name(TEST_DOCUMENT_TYPE_NAME)
        .expect("expected test document type");
    let transition = BatchTransition::new_document_update_price_transition_from_document(
        document,
        document_type,
        200,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        None,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_document_transfer_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let recipient_id = Identifier::random();
    let document_type = data_contract
        .document_type_for_name(TEST_DOCUMENT_TYPE_NAME)
        .expect("expected test document type");
    let transition = BatchTransition::new_document_transfer_transition_from_document(
        document,
        document_type,
        recipient_id,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        None,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

async fn new_mock_sdk_with_contract_nonce(
    identity_id: Identifier,
    contract_id: Identifier,
    fetched_nonce: u64,
) -> Sdk {
    let mut sdk = SdkBuilder::new_mock().build().expect("build mock sdk");

    sdk.mock()
        .expect_fetch::<IdentityContractNonceFetcher, _>(
            (identity_id, contract_id),
            Some(IdentityContractNonceFetcher(fetched_nonce)),
        )
        .await
        .expect("set nonce fetch expectation");

    sdk
}

#[tokio::test]
async fn document_delete_sign_masks_nonce_and_does_not_hit_nonce_out_of_bounds_validation() {
    // Document builders always create exactly one transition and obtain nonce through
    // `Sdk::get_identity_contract_nonce`, which masks out-of-bounds bits.
    // This makes `validate_base_structure` nonce-out-of-bounds errors unreachable here.
    let document_type_name = "testDoc";
    let data_contract = test_data_contract(document_type_name);
    let owner_id = Identifier::random();

    let sdk = new_mock_sdk_with_contract_nonce(owner_id, data_contract.id(), 1_u64 << 50).await;

    let builder = DocumentDeleteTransitionBuilder::new(
        Arc::clone(&data_contract),
        document_type_name.to_string(),
        Identifier::random(),
        owner_id,
    );

    let result = builder
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        result.is_ok(),
        "unexpected error while signing document delete transition: {:?}",
        result.err()
    );

    assert!(
        !matches!(
            result,
            Err(Error::Protocol(ProtocolError::ConsensusError(consensus_error)))
                if matches!(*consensus_error, dpp::consensus::ConsensusError::BasicError(
                    dpp::consensus::basic::BasicError::NonceOutOfBoundsError(_)
                ))
        ),
        "nonce out-of-bounds should be unreachable via document builders"
    );
}
