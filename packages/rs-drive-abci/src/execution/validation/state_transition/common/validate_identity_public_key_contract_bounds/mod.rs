use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_identity_public_key_contract_bounds::v0::validate_identity_public_keys_contract_bounds_v0;
use crate::execution::validation::state_transition::common::validate_identity_public_key_contract_bounds::v1::validate_identity_public_keys_contract_bounds_v1;
use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub mod v0;
pub mod v1;

/// Validates the contract bounds attached to each public key in `identity_public_keys_with_witness`.
///
/// `epoch` is used by v1+ to bill the underlying grovedb reads to `execution_context`; v0
/// ignores it (v0 didn't bill these reads — pre-PROTOCOL_VERSION_12 behavior is preserved
/// verbatim for chain replay).
pub(crate) fn validate_identity_public_keys_contract_bounds(
    identity_id: Identifier,
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
    drive: &Drive,
    epoch: &Epoch,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .common_validation_methods
        .validate_identity_public_key_contract_bounds
    {
        0 => {
            let _ = epoch; // v0 doesn't bill these reads — by design, for chain replay.
            validate_identity_public_keys_contract_bounds_v0(
                identity_id,
                identity_public_keys_with_witness,
                drive,
                transaction,
                execution_context,
                platform_version,
            )
        }
        1 => validate_identity_public_keys_contract_bounds_v1(
            identity_id,
            identity_public_keys_with_witness,
            drive,
            epoch,
            transaction,
            execution_context,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "validate_identity_public_keys_contract_bounds".to_string(),
            known_versions: vec![0, 1],
            received: version,
        })),
    }
}

#[cfg(test)]
mod tests {
    //! Differential test pinning the v0 vs v1 behavior on the previously-buggy branch:
    //! `Purpose::DECRYPTION` against `ContractBounds::SingleContractDocumentType`, where
    //! the document type sets `requiresIdentityDecryptionBoundedKey` but not
    //! `requiresIdentityEncryptionBoundedKey`. v0 mistakenly checked the encryption
    //! requirement; v1 correctly checks the decryption requirement.
    //!
    //! v0's behavior is frozen for chain replay — the v0 assertion below pins it.
    use super::v0::validate_identity_public_keys_contract_bounds_v0;
    use super::v1::validate_identity_public_keys_contract_bounds_v1;
    use crate::execution::types::execution_operation::ValidationOperation;
    use crate::execution::types::state_transition_execution_context::{
        StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
    };
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContractFactory;
    use dpp::identifier::Identifier;
    use dpp::identity::contract_bounds::ContractBounds;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::{platform_value, BinaryData};
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
    use dpp::version::{DefaultForPlatformVersion, PlatformVersion};

    /// Document type with decryption bounds set and encryption bounds unset — exactly the
    /// asymmetric case the v0 bug mishandled.
    fn build_contract_with_decryption_only_bounds(
        platform_version: &PlatformVersion,
    ) -> dpp::data_contract::DataContract {
        let factory = DataContractFactory::new(platform_version.protocol_version).expect("factory");
        let schemas = platform_value!({
            "note": {
                "type": "object",
                "requiresIdentityDecryptionBoundedKey": 0_u64,
                "properties": {
                    "message": {"type": "string", "position": 0, "maxLength": 100_u32},
                },
                "additionalProperties": false,
            }
        });
        factory
            .create_with_value_config(Identifier::random(), 0, schemas, None, None)
            .expect("contract built")
            .data_contract_owned()
    }

    fn make_decryption_key_bound_to_doc_type(
        contract_id: Identifier,
        document_type_name: String,
    ) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreationV0 {
            id: 0,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::DECRYPTION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: Some(ContractBounds::SingleContractDocumentType {
                id: contract_id,
                document_type_name,
            }),
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            signature: BinaryData::default(),
        }
        .into()
    }

    #[test]
    fn v0_wrongly_rejects_decryption_key_when_only_decryption_bounds_set() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let contract = build_contract_with_decryption_only_bounds(platform_version);
        let contract_id = contract.id();
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("contract applied");

        let key = make_decryption_key_bound_to_doc_type(contract_id, "note".to_string());
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("execution context");

        let result = validate_identity_public_keys_contract_bounds_v0(
            Identifier::random(),
            &[key],
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("v0 returns Ok");

        assert!(
            !result.is_valid(),
            "v0 has the bug — it consults encryption bounds for a DECRYPTION key"
        );
        match &result.errors[0] {
            ConsensusError::BasicError(BasicError::DataContractBoundsNotPresentError(_)) => {}
            other => panic!(
                "expected v0 to wrongly emit DataContractBoundsNotPresentError, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn v1_accepts_decryption_key_when_decryption_bounds_present() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let contract = build_contract_with_decryption_only_bounds(platform_version);
        let contract_id = contract.id();
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("contract applied");

        let key = make_decryption_key_bound_to_doc_type(contract_id, "note".to_string());
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("execution context");

        let epoch = Epoch::new(0).expect("epoch 0");
        let result = validate_identity_public_keys_contract_bounds_v1(
            Identifier::random(),
            &[key],
            &platform.drive,
            &epoch,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("v1 returns Ok");

        assert!(
            result.is_valid(),
            "v1 fix: a DECRYPTION key targeting a doc type with decryption bounds is valid; \
             got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn v1_bills_contract_fetch_and_unique_key_lookup() {
        // v0 dropped these grovedb-read costs on the floor (audit N6/N7). v1 must push them
        // into the execution context so paid-error / successful-action billing sees them.
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let contract = build_contract_with_decryption_only_bounds(platform_version);
        let contract_id = contract.id();
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("contract applied");

        let key = make_decryption_key_bound_to_doc_type(contract_id, "note".to_string());
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("execution context");
        let epoch = Epoch::new(0).expect("epoch 0");

        let result = validate_identity_public_keys_contract_bounds_v1(
            Identifier::random(),
            &[key],
            &platform.drive,
            &epoch,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("v1 returns Ok");
        assert!(
            result.is_valid(),
            "expected valid; got: {:?}",
            result.errors
        );

        // Should have at least two billing entries: one for the contract fetch and one for the
        // uniqueness key lookup. Both must be non-zero processing fees.
        let precalculated: Vec<_> = execution_context
            .operations_slice()
            .iter()
            .filter_map(|op| match op {
                ValidationOperation::PrecalculatedOperation(fee) => Some(fee),
                _ => None,
            })
            .collect();
        assert!(
            precalculated.len() >= 2,
            "expected at least 2 billed grovedb reads, got {}: {:?}",
            precalculated.len(),
            precalculated
        );
        let total_processing: u64 = precalculated.iter().map(|f| f.processing_fee).sum();
        assert!(
            total_processing > 0,
            "expected non-zero billed processing fee from contract fetch + key lookup, got {:?}",
            precalculated
        );
    }
}
