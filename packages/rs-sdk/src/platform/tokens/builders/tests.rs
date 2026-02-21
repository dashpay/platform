use super::mint::TokenMintTransitionBuilder;
use crate::{Error, Sdk, SdkBuilder};
use dpp::address_funds::AddressWitness;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DataContractFactory;
use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{platform_value, BinaryData, Value};
use dpp::prelude::Identifier;
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
async fn token_mint_sign_returns_invalid_token_amount_error_when_amount_is_zero() {
    let issuer_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(issuer_id, data_contract.id(), 0).await;

    let result = TokenMintTransitionBuilder::new(Arc::clone(&data_contract), 0, issuer_id, 0)
        .issued_to_identity_id(Identifier::random())
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        matches!(
            result,
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_mint_sign_returns_invalid_action_id_error_for_mismatched_group_action_id() {
    let issuer_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(issuer_id, data_contract.id(), 0).await;

    let invalid_group_info = GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
        GroupStateTransitionInfo {
            group_contract_position: 0,
            action_id: Identifier::from_bytes(&[0; 32]).expect("create static action id"),
            action_is_proposer: true,
        },
    );

    let result = TokenMintTransitionBuilder::new(Arc::clone(&data_contract), 0, issuer_id, 1)
        .issued_to_identity_id(Identifier::random())
        .with_using_group_info(invalid_group_info)
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        matches!(
            result,
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidActionIdError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}
