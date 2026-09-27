//! A `contract` reference's `owner` requirement on replace (protocol version
//! 14). The requirement relates the referenced contract to the WRITER of the
//! referring document (`"self"`: the writer owns it, `"other"`: someone else
//! does), and the writer is transition metadata: a transfer or a purchase
//! hands a document to a new owner without any write. So on a document type
//! whose documents can be transferred or traded, a reference carrying the
//! requirement is re-validated on every replace, touched or not, as a
//! `$ownerId` writer gate is, and the new owner has to repoint it. Where the
//! owner cannot change nothing is re-checked (the writer never moves and a
//! contract's owner never changes), and the other requirements, facts about
//! the referenced contract, never bring a reference back.
//!
//! The fixture's `message` can be transferred. `selfContractId` requires a
//! contract the writer owns, `otherContractId` one it does not,
//! `readonlyContractId` a read-only contract, `selfContracts` holds contracts
//! the writer owns on the elements of a typed array, and `note` is a plain
//! string a replace can change alone. The non-transferable twin is the
//! `owner: "self"` fixture of the create tests.

use super::*;

mod contract_owner_requirement_tests {
    use super::super::reference_test_setup::{
        assert_successful, process_and_commit, register_contract_at,
    };
    use super::*;
    use crate::execution::types::execution_operation::ValidationOperation;
    use crate::execution::types::state_transition_execution_context::{
        StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
    };
    use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::DocumentReferenceValidation;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::platform_value;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::validation::SimpleConsensusValidationResult;
    use dpp::version::DefaultForPlatformVersion;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use drive::util::test_helpers::setup_contract;
    use std::collections::{BTreeMap, BTreeSet};

    /// The transferable fixture.
    const CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-owner-contract-ref-transferable.json";

    /// A `message` type that can be neither transferred nor traded, whose
    /// `refContractId` requires a contract the writer owns. It declares an
    /// empty `indices` list, which only a non-validating load admits.
    const NON_TRANSFERABLE_CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-owner-self-contract-ref.json";

    /// The contract the writer and the receiver each own a copy of, for a
    /// reference to point at.
    const REFERENCED_CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-contract-ref.json";

    /// The ids of the writer's and the receiver's referenced contracts.
    const WRITER_CONTRACT_ID: [u8; 32] = [0xC5; 32];
    const RECEIVER_CONTRACT_ID: [u8; 32] = [0xC6; 32];

    /// The owner the direct validations register the fixtures under, and a
    /// writer that owns nothing.
    const FIXTURE_OWNER: Identifier = Identifier::new([0x0A; 32]);
    const STRANGER: Identifier = Identifier::new([0xEE; 32]);

    /// Who writes the replace: the writer, who created the document and still
    /// holds it, or the receiver it was transferred to in between.
    #[derive(Clone, Copy)]
    enum ReplacedBy {
        Writer,
        Receiver,
    }

    /// A referenced contract owned by each identity of a create, transfer and
    /// replace: the writer and the receiver.
    struct Targets {
        writer_contract_id: Identifier,
        receiver_contract_id: Identifier,
    }

    /// A contract with the id `contract_id` owned by `owner`. Written to state
    /// directly, as the fixtures are.
    fn contract_owned_by(
        platform: &TempPlatform<MockCoreRPCLike>,
        contract_id: [u8; 32],
        owner: Identifier,
    ) -> Identifier {
        setup_contract(
            &platform.drive,
            REFERENCED_CONTRACT_PATH,
            Some(contract_id),
            Some(owner.to_buffer()),
            None::<fn(&mut DataContract)>,
            None,
            None,
        )
        .id()
    }

    /// A list of identifiers as a document stores it.
    fn identifier_list(ids: &[Identifier]) -> Value {
        Value::Array(
            ids.iter()
                .map(|id| Value::Identifier(id.to_buffer()))
                .collect(),
        )
    }

    /// Registers the transferable fixture, has the writer create a `message`
    /// shaped by `create_mutator` (asserting success), transfers it to the
    /// receiver when `replaced_by` says so, and replaces it, shaped by
    /// `replace_mutator`, as its holder. Returns the replace execution result
    /// and the targets.
    async fn run_create_then_replace<C, R>(
        replaced_by: ReplacedBy,
        create_mutator: C,
        replace_mutator: R,
    ) -> (StateTransitionExecutionResult, Targets)
    where
        C: FnOnce(&mut Document, &Targets),
        R: FnOnce(&mut Document, &Targets),
    {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(437);

        let platform_state = platform.state.load();

        let (writer, writer_signer, writer_key) =
            setup_identity(&mut platform, 961, dash_to_credits!(0.1));
        let (receiver, receiver_signer, receiver_key) =
            setup_identity(&mut platform, 454, dash_to_credits!(0.1));

        let targets = Targets {
            writer_contract_id: contract_owned_by(&platform, WRITER_CONTRACT_ID, writer.id()),
            receiver_contract_id: contract_owned_by(&platform, RECEIVER_CONTRACT_ID, receiver.id()),
        };

        let contract = register_contract_at(
            &platform,
            CONTRACT_PATH,
            FIXTURE_OWNER,
            true,
            platform_version,
        );
        let message = contract
            .document_type_for_name("message")
            .expect("expected a message document type");

        let entropy = Bytes32::random_with_rng(&mut rng);
        let mut document = message
            .random_document_with_identifier_and_entropy(
                &mut rng,
                writer.id(),
                entropy,
                // Every property is optional; each test sets what it
                // exercises
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random message document");
        document
            .set_id_for_creation(message, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");
        create_mutator(&mut document, &targets);

        let create_transition = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            message,
            entropy.0,
            &writer_key,
            2,
            0,
            None,
            &writer_signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create documents batch transition");
        let result = process_and_commit(
            &platform,
            &platform_state,
            &create_transition,
            platform_version,
        );
        assert_successful(
            &result,
            "the writer meets every requirement when it creates the document",
        );

        let (replace_key, replace_signer, replace_nonce, replace_revision) = match replaced_by {
            ReplacedBy::Writer => (&writer_key, &writer_signer, 3, 2),
            ReplacedBy::Receiver => {
                // The transfer is not a reference check: the document changes
                // hands with its references as written
                document.set_revision(Some(2));
                let transfer_transition =
                    BatchTransition::new_document_transfer_transition_from_document(
                        document.clone(),
                        message,
                        receiver.id(),
                        &writer_key,
                        3,
                        0,
                        None,
                        &writer_signer,
                        platform_version,
                        None,
                    )
                    .await
                    .expect("expect to create documents batch transition for transfer");
                let result = process_and_commit(
                    &platform,
                    &platform_state,
                    &transfer_transition,
                    platform_version,
                );
                assert_successful(&result, "the transfer checks no reference");
                document.set_owner_id(receiver.id());
                (&receiver_key, &receiver_signer, 1, 3)
            }
        };

        document.set_revision(Some(replace_revision));
        replace_mutator(&mut document, &targets);

        let replace_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                message,
                replace_key,
                replace_nonce,
                0,
                None,
                replace_signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");
        let result = process_and_commit(
            &platform,
            &platform_state,
            &replace_transition,
            platform_version,
        );

        (
            result
                .execution_results()
                .first()
                .expect("expected one execution result")
                .clone(),
            targets,
        )
    }

    /// The write was refused, paid, because the contract `contract_id`
    /// referenced at `path` does not meet its `owner` requirement `required`
    /// (40135).
    fn assert_owner_requirement_unmet(
        result: &StateTransitionExecutionResult,
        contract_id: Identifier,
        required: &str,
        path: &str,
    ) {
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedContractRequirementNotMetError(e)),
                ..
            } if *e.contract_id() == contract_id
                && e.field() == "owner"
                && e.required() == required
                && e.path() == path,
            "{result:?}"
        );
    }

    /// The case this suite exists for: the receiver of a transferred document
    /// does not own the contract the writer's `owner: "self"` reference
    /// points at, and its replace of another property is refused. Before the
    /// requirement was re-checked on every replace, the untouched reference
    /// was skipped and the replace passed.
    #[tokio::test]
    async fn should_document_replace_fail_after_transfer_when_untouched_self_owned_contract_reference_is_not_the_new_owners(
    ) {
        let (result, targets) = run_create_then_replace(
            ReplacedBy::Receiver,
            |document, targets| {
                document.set("selfContractId", targets.writer_contract_id.into());
            },
            |document, _| {
                document.set("note", "changed by the receiver".into());
            },
        )
        .await;

        assert_owner_requirement_unmet(
            &result,
            targets.writer_contract_id,
            "self",
            "selfContractId",
        );
    }

    /// The other direction: the writer pointed `otherContractId` at the
    /// receiver's contract, which it does not own, and the transfer makes the
    /// receiver both the writer and the contract's owner.
    #[tokio::test]
    async fn should_document_replace_fail_after_transfer_when_untouched_other_owned_contract_reference_is_the_new_owners(
    ) {
        let (result, targets) = run_create_then_replace(
            ReplacedBy::Receiver,
            |document, targets| {
                document.set("otherContractId", targets.receiver_contract_id.into());
            },
            |document, _| {
                document.set("note", "changed by the receiver".into());
            },
        )
        .await;

        assert_owner_requirement_unmet(
            &result,
            targets.receiver_contract_id,
            "other",
            "otherContractId",
        );
    }

    /// The way out after a transfer: the receiver repoints the reference at a
    /// contract it owns.
    #[tokio::test]
    async fn should_document_replace_succeed_after_transfer_when_self_owned_contract_reference_is_repointed_at_the_new_owners_contract(
    ) {
        let (result, _) = run_create_then_replace(
            ReplacedBy::Receiver,
            |document, targets| {
                document.set("selfContractId", targets.writer_contract_id.into());
            },
            |document, targets| {
                document.set("selfContractId", targets.receiver_contract_id.into());
            },
        )
        .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// The re-check is not a refusal: the writer still holding its
    /// transferable document replaces another property, the requirement is
    /// checked again and still holds.
    #[tokio::test]
    async fn should_document_replace_succeed_when_the_writer_still_holds_its_transferable_document()
    {
        let (result, _) = run_create_then_replace(
            ReplacedBy::Writer,
            |document, targets| {
                document.set("selfContractId", targets.writer_contract_id.into());
            },
            |document, _| {
                document.set("note", "changed by the writer".into());
            },
        )
        .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// The elements of a typed array follow the single reference: an
    /// untouched list is re-validated, element by element.
    #[tokio::test]
    async fn should_document_replace_fail_after_transfer_when_untouched_self_owned_contract_elements_are_not_the_new_owners(
    ) {
        let (result, targets) = run_create_then_replace(
            ReplacedBy::Receiver,
            |document, targets| {
                document.set(
                    "selfContracts",
                    identifier_list(&[targets.writer_contract_id]),
                );
            },
            |document, _| {
                document.set("note", "changed by the receiver".into());
            },
        )
        .await;

        assert_owner_requirement_unmet(
            &result,
            targets.writer_contract_id,
            "self",
            "selfContracts[0]",
        );
    }

    /// A changed list normally re-validates only the elements the stored list
    /// did not hold. An owner requirement re-validates them all: the receiver
    /// appending its own contract does not keep the writer's.
    #[tokio::test]
    async fn should_document_replace_fail_after_transfer_when_a_changed_list_keeps_a_contract_element_the_new_owner_does_not_own(
    ) {
        let (result, targets) = run_create_then_replace(
            ReplacedBy::Receiver,
            |document, targets| {
                document.set(
                    "selfContracts",
                    identifier_list(&[targets.writer_contract_id]),
                );
            },
            |document, targets| {
                document.set(
                    "selfContracts",
                    identifier_list(&[targets.writer_contract_id, targets.receiver_contract_id]),
                );
            },
        )
        .await;

        assert_owner_requirement_unmet(
            &result,
            targets.writer_contract_id,
            "self",
            "selfContracts[0]",
        );
    }

    /// A platform with the fixture at `contract_path` registered under
    /// `FIXTURE_OWNER`, fully validated when `validate` is set.
    fn platform_with(
        contract_path: &str,
        validate: bool,
    ) -> (TempPlatform<MockCoreRPCLike>, DataContract) {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let contract = register_contract_at(
            &platform,
            contract_path,
            FIXTURE_OWNER,
            validate,
            PlatformVersion::latest(),
        );
        (platform, contract)
    }

    /// Runs the document reference validation directly on `data`, a
    /// `message` of `contract` written by `writer`, as a replace changing
    /// `changed_fields`, and returns the result with the execution context,
    /// whose operations are the reads it billed. `writer` is tied to no stored
    /// document, so on a type whose documents cannot change owner it can
    /// stand for a writer other than the one who wrote the reference, a
    /// state consensus never reaches, which is what makes a skipped check
    /// observable.
    fn validate_replace_directly(
        platform: &TempPlatform<MockCoreRPCLike>,
        contract: &DataContract,
        writer: Identifier,
        data: &BTreeMap<String, Value>,
        changed_fields: &[&str],
        platform_version: &PlatformVersion,
    ) -> (
        SimpleConsensusValidationResult,
        StateTransitionExecutionContext,
    ) {
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
            document_type_name: "message".to_string(),
            data_contract: contract_fetch_info.expect("the contract is in state"),
            token_cost: None,
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
        let changed_fields: BTreeSet<String> = changed_fields
            .iter()
            .map(|field| field.to_string())
            .collect();
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("expected an execution context");
        let result = base
            .validate_document_references(
                data,
                writer,
                // Neither fixture declares a creator reference
                None,
                Some(&changed_fields),
                // Nor a changed typed array, whose stored list this would
                // hold
                None,
                &platform_ref,
                &BlockInfo::default(),
                None,
                &mut execution_context,
                platform_version,
            )
            .expect("expected the references to be validated");
        (result, execution_context)
    }

    /// The direct validation was refused because the contract `contract_id`
    /// referenced at `path` does not meet its `field` requirement `required`
    /// (40135).
    fn assert_requirement_unmet(
        result: &SimpleConsensusValidationResult,
        contract_id: Identifier,
        field: &str,
        required: &str,
        path: &str,
    ) {
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::ReferencedContractRequirementNotMetError(e))]
                if *e.contract_id() == contract_id
                    && e.field() == field
                    && e.required() == required
                    && e.path() == path,
            "{:?}",
            result.errors
        );
    }

    /// The control: on a type whose documents stay with their owner, every
    /// replace is written by the owner the requirement was checked against
    /// and a contract's owner never changes, so a replace leaving the
    /// reference alone checks nothing and bills no contract fetch, even for
    /// a writer the requirement would refuse. The same value is refused when
    /// the replace writes it.
    #[test]
    fn should_read_nothing_for_an_untouched_contract_owner_requirement_when_documents_cannot_change_owner(
    ) {
        let (platform, contract) = platform_with(NON_TRANSFERABLE_CONTRACT_PATH, false);
        let data = BTreeMap::from([
            (
                "refContractId".to_string(),
                Value::Identifier(contract.id().to_buffer()),
            ),
            ("note".to_string(), Value::Text("changed".to_string())),
        ]);

        let (result, execution_context) = validate_replace_directly(
            &platform,
            &contract,
            STRANGER,
            &data,
            &["note"],
            PlatformVersion::latest(),
        );
        assert!(result.is_valid(), "{:?}", result.errors);
        assert!(execution_context.operations_slice().is_empty());

        let (result, _) = validate_replace_directly(
            &platform,
            &contract,
            STRANGER,
            &data,
            &["refContractId"],
            PlatformVersion::latest(),
        );
        assert_requirement_unmet(&result, contract.id(), "owner", "self", "refContractId");
    }

    /// On a type whose documents can change owner, a replace leaving the
    /// reference alone re-checks the requirement against its writer and
    /// bills the contract fetch: a writer that does not own the referenced
    /// contract is refused, its owner passes.
    #[test]
    fn should_re_check_an_untouched_contract_owner_requirement_when_documents_can_change_owner() {
        let (platform, contract) = platform_with(CONTRACT_PATH, true);
        let data = BTreeMap::from([
            (
                "selfContractId".to_string(),
                Value::Identifier(contract.id().to_buffer()),
            ),
            ("note".to_string(), Value::Text("changed".to_string())),
        ]);

        let (result, execution_context) = validate_replace_directly(
            &platform,
            &contract,
            STRANGER,
            &data,
            &["note"],
            PlatformVersion::latest(),
        );
        assert_requirement_unmet(&result, contract.id(), "owner", "self", "selfContractId");
        assert_matches!(
            execution_context.operations_slice(),
            [ValidationOperation::PrecalculatedOperation(_)],
            "the contract fetch is the one billed operation"
        );

        let (result, execution_context) = validate_replace_directly(
            &platform,
            &contract,
            FIXTURE_OWNER,
            &data,
            &["note"],
            PlatformVersion::latest(),
        );
        assert!(result.is_valid(), "{:?}", result.errors);
        assert_matches!(
            execution_context.operations_slice(),
            [ValidationOperation::PrecalculatedOperation(_)],
            "the re-check is billed whoever writes"
        );
    }

    /// Only the owner requirement brings a reference back: the fixture is not
    /// read-only, so writing a `readonlyContractId` naming it is refused, but
    /// a replace leaving it alone checks nothing, even on a type whose
    /// documents can change owner.
    #[test]
    fn should_not_re_check_untouched_contract_requirements_other_than_owner_when_documents_can_change_owner(
    ) {
        let (platform, contract) = platform_with(CONTRACT_PATH, true);
        let data = BTreeMap::from([
            (
                "readonlyContractId".to_string(),
                Value::Identifier(contract.id().to_buffer()),
            ),
            ("note".to_string(), Value::Text("changed".to_string())),
        ]);

        let (result, execution_context) = validate_replace_directly(
            &platform,
            &contract,
            STRANGER,
            &data,
            &["note"],
            PlatformVersion::latest(),
        );
        assert!(result.is_valid(), "{:?}", result.errors);
        assert!(execution_context.operations_slice().is_empty());

        let (result, _) = validate_replace_directly(
            &platform,
            &contract,
            STRANGER,
            &data,
            &["readonlyContractId"],
            PlatformVersion::latest(),
        );
        assert_requirement_unmet(
            &result,
            contract.id(),
            "readonly",
            "true",
            "readonlyContractId",
        );
    }

    /// Protocol version 13, the last shipped version, selects the same
    /// reference validation generation, whose `binds_a_changed_property` this
    /// change edits in place. Its parser leaves `refersTo` unread, so the
    /// transferable fixture carries no contract reference there, and a
    /// replace by a writer owning nothing reads and bills nothing, whichever
    /// property it changes. The typed array is left out, since typed arrays
    /// do not parse before version 14.
    #[test]
    fn should_check_no_contract_owner_requirement_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 exists");
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut schema: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(CONTRACT_PATH).expect("the fixture reads"),
        )
        .expect("the fixture is JSON");
        schema["documentSchemas"]["message"]["properties"]
            .as_object_mut()
            .expect("an object")
            .remove("selfContracts");
        let contract = DataContract::from_value(
            platform_value::to_value(schema).expect("converts"),
            false,
            platform_version,
        )
        .expect("protocol version 13 parses the contract, ignoring refersTo");
        assert!(contract
            .document_type_for_name("message")
            .expect("the message type")
            .flattened_properties()
            .values()
            .all(|property| property.property_type.reference().is_none()));
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

        let data = BTreeMap::from([
            (
                "selfContractId".to_string(),
                Value::Identifier(contract.id().to_buffer()),
            ),
            ("note".to_string(), Value::Text("changed".to_string())),
        ]);
        for changed_field in ["note", "selfContractId"] {
            let (result, execution_context) = validate_replace_directly(
                &platform,
                &contract,
                STRANGER,
                &data,
                &[changed_field],
                platform_version,
            );
            assert!(result.is_valid(), "{:?}", result.errors);
            assert!(execution_context.operations_slice().is_empty());
        }
    }
}
