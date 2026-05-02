//! Shared test infrastructure for document and token transition builder tests.

use crate::{Error, Sdk, SdkBuilder};
use async_trait::async_trait;
use dpp::address_funds::AddressWitness;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DataContractFactory;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{platform_value, BinaryData, Value};
use dpp::prelude::Identifier;
use dpp::state_transition::StateTransition;
use dpp::ProtocolError;
use drive_proof_verifier::types::IdentityContractNonceFetcher;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) const TEST_DOCUMENT_TYPE_NAME: &str = "testDoc";
/// Exceeds the 40-bit nonce mask (MISSING_IDENTITY_REVISIONS_FILTER), triggering
/// NonceOutOfBoundsError in validate_base_structure.
pub(crate) const INVALID_NONCE: u64 = 1_u64 << 50;

#[derive(Debug)]
pub(crate) struct TestSigner;

#[async_trait]
impl Signer<IdentityPublicKey> for TestSigner {
    async fn sign(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        Ok(BinaryData::from(vec![1; 65]))
    }

    async fn sign_create_witness(
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

pub(crate) fn test_identity_public_key() -> IdentityPublicKey {
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

pub(crate) fn test_data_contract(
    document_type_name: &str,
) -> Arc<dpp::data_contract::DataContract> {
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

/// Asserts that a freshly-constructed transition was rejected by DPP's
/// pre-sign base-structure validation with `NonceOutOfBoundsError`.
///
/// `BatchTransition::new_*` constructors run pre-sign base-structure validation before
/// signing (see `batch-base-structure-validation` feature), so an
/// out-of-bounds identity contract nonce surfaces as
/// `ProtocolError::ConsensusError(BasicError::NonceOutOfBoundsError)` or inside
/// `ProtocolError::ConsensusErrors(...)` from the constructor itself.
#[track_caller]
pub(crate) fn assert_nonce_out_of_bounds_construction_error(
    construction_result: Result<StateTransition, ProtocolError>,
) {
    let result = construction_result.map_err(Error::Protocol);
    let has_nonce_error = match &result {
        Err(Error::Protocol(ProtocolError::ConsensusError(consensus_error))) => matches!(
            **consensus_error,
            ConsensusError::BasicError(BasicError::NonceOutOfBoundsError(_)),
        ),
        Err(Error::Protocol(ProtocolError::ConsensusErrors(consensus_errors))) => {
            consensus_errors.iter().any(|consensus_error| {
                matches!(
                    consensus_error,
                    ConsensusError::BasicError(BasicError::NonceOutOfBoundsError(_)),
                )
            })
        }
        _ => false,
    };

    assert!(
        has_nonce_error,
        "expected NonceOutOfBoundsError, got {result:?}",
    );
}

pub(crate) async fn new_mock_sdk_with_contract_nonce(
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
