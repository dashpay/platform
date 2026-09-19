//! Semantic pins for the version slots the keep-history document lifecycle
//! rides on.
//!
//! A numeric pin alone would not say what the number means, so each assertion
//! below names the behaviour the slot selects and the released protocol
//! versions it must leave alone.

use platform_version::version::PlatformVersion;

/// The released protocol versions parse contracts with a generation that
/// predates the lifecycle grammar and refuse a keep-history delete before it
/// reaches storage. Both must replay identically forever.
#[test]
fn should_preserve_released_keep_history_validation_versions() {
    for protocol in [12, 13, 14] {
        let version = PlatformVersion::get(protocol).unwrap();
        let released_parser_generation = if protocol == 14 { 3 } else { 2 };
        assert_eq!(
            version
                .dpp
                .contract_versions
                .document_type_versions
                .class_method_versions
                .try_from_schema,
            released_parser_generation,
            "parser at protocol {protocol}"
        );
        // Protocol 14 refuses a delete of a keep-history document as a paid
        // consensus error instead of the earlier internal error; neither
        // generation carries one out.
        let released_delete_structure_generation = if protocol == 14 { 1 } else { 0 };
        assert_eq!(
            version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_delete_transition_structure_validation,
            released_delete_structure_generation,
            "delete structure validation at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_delete_transition_state_validation,
            0,
            "delete state validation at protocol {protocol}"
        );
        assert!(
            version
                .dpp
                .state_transition_serialization_versions
                .document_erase_state_transition
                .is_none(),
            "the erase kind must not exist on the wire at protocol {protocol}"
        );
        assert!(
            version
                .system_limits
                .max_document_revisions_erased_per_transition
                .is_none(),
            "no erase can exist at protocol {protocol}, so no chunk bounds one"
        );
        assert_eq!(
            version
                .drive
                .methods
                .document
                .delete
                .delete_document_for_contract_operations,
            0,
            "the keep-history delete branch must not be selected at protocol {protocol}"
        );
        let released_insert_generation = if protocol == 14 { 1 } else { 0 };
        assert_eq!(
            version
                .drive
                .methods
                .document
                .insert
                .add_document_for_contract_operations,
            released_insert_generation,
            "document insert at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive
                .methods
                .state_transitions
                .convert_to_high_level_operations
                .document_delete_transition,
            0,
            "delete action conversion at protocol {protocol}"
        );
    }
}

/// Protocol 15 selects the lifecycle: the parser generation that admits the new
/// keywords, the delete that consults the lifecycle, and the erase kind with
/// the bound on how much one transition may remove.
#[test]
fn should_activate_keep_history_validation_at_protocol_15() {
    let version = PlatformVersion::get(15).unwrap();
    assert_eq!(
        version
            .dpp
            .contract_versions
            .document_type_versions
            .class_method_versions
            .try_from_schema,
        4
    );
    assert_eq!(
        version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .document_delete_transition_structure_validation,
        2
    );
    assert_eq!(
        version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .document_delete_transition_state_validation,
        1
    );
    assert_eq!(
        version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .document_create_transition_state_validation,
        3,
        "protocol 15 refuses to create over a deleted or erasing keep-history document"
    );
    assert_eq!(
        version
            .drive
            .methods
            .document
            .delete
            .delete_document_for_contract_operations,
        1,
        "protocol 15 selects the keep-history delete branch"
    );
    assert_eq!(
        version
            .drive
            .methods
            .document
            .insert
            .add_document_for_contract_operations,
        2,
        "protocol 15 selects the insert that never writes over retained revisions"
    );
    assert_eq!(
        version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .document_delete_transition,
        1,
        "protocol 15 selects the lifecycle-specific delete operation"
    );

    let bounds = version
        .dpp
        .state_transition_serialization_versions
        .document_erase_state_transition
        .as_ref()
        .expect("the erase kind joins the wire at protocol 15");
    assert_eq!(bounds.bounds.min_version, 0);
    assert_eq!(bounds.bounds.max_version, 0);
    assert_eq!(bounds.bounds.default_current_version, 0);

    let chunk = version
        .system_limits
        .max_document_revisions_erased_per_transition
        .expect("protocol 15 bounds the erase chunk");
    assert_eq!(
        chunk, 100,
        "the chunk bounds the work one erase can demand and the balance every \
         erase needs up front; changing it changes both"
    );
}

/// The erase kind is appended to the batch's document transition enum and
/// carried by the shipped batch wire formats, so no batch-level generation
/// changes with it: the batch wire bounds and every generation that reads a
/// batch as a whole are the same at protocol 15 as at protocol 14. Only the
/// per-kind erase slots turn on.
#[test]
fn should_carry_the_erase_kind_without_a_new_batch_generation() {
    let released = PlatformVersion::get(14).unwrap();
    let current = PlatformVersion::get(15).unwrap();

    let batch_bounds = |version: &PlatformVersion| {
        let bounds = &version
            .dpp
            .state_transition_serialization_versions
            .batch_state_transition;
        (
            bounds.min_version,
            bounds.max_version,
            bounds.default_current_version,
        )
    };
    assert_eq!(
        batch_bounds(current),
        batch_bounds(released),
        "the batch wire formats are unchanged; the erase kind rides inside them"
    );
    assert_eq!(
        batch_bounds(current),
        (0, 1, 1),
        "the batch default wire format stays 1"
    );

    let batch_validation = |version: &PlatformVersion| {
        version
            .dpp
            .state_transitions
            .documents
            .documents_batch_transition
            .validation
            .validate_base_structure
    };
    assert_eq!(
        batch_validation(current),
        batch_validation(released),
        "the batch basic-structure generation gates the erase kind by its bounds slot"
    );

    let batch_generations = |version: &PlatformVersion| {
        let batch = &version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition;
        (
            batch.transform_into_action,
            batch.advanced_structure,
            batch.state,
            batch.revision,
            batch.is_allowed,
            batch.fetch_documents_for_transitions_knowing_contract_and_document_type,
        )
    };
    assert_eq!(
        batch_generations(current),
        batch_generations(released),
        "the batch transformer, structure, state, nonce and admission generations are unchanged"
    );

    let batch_drive = |version: &PlatformVersion| {
        (
            version
                .drive
                .methods
                .state_transitions
                .convert_to_high_level_operations
                .documents_batch_transition,
            version.drive.methods.prove.prove_state_transition,
            version
                .drive
                .methods
                .verify
                .state_transition
                .verify_state_transition_was_executed_with_proof,
        )
    };
    assert_eq!(
        batch_drive(current),
        batch_drive(released),
        "the batch conversion, prover and execution-proof verifier are unchanged"
    );
}

/// The lifecycle read and the erase storage operation exist at exactly one
/// version each, so no table can select an implementation that is not there.
#[test]
fn should_expose_one_implementation_of_each_new_lifecycle_slot() {
    for protocol in 1..=PlatformVersion::latest().protocol_version {
        let Ok(version) = PlatformVersion::get(protocol) else {
            continue;
        };
        let expected_erase_version = (protocol >= 15).then_some(0);
        assert_eq!(
            version
                .drive
                .methods
                .document
                .query
                .fetch_document_lifecycle,
            expected_erase_version,
            "lifecycle read at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive
                .methods
                .document
                .delete
                .erase_document_for_contract_operations,
            expected_erase_version,
            "erase operation at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive
                .methods
                .document
                .delete
                .add_estimation_costs_for_erase_document,
            expected_erase_version,
            "erase estimation at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive
                .methods
                .state_transitions
                .convert_to_high_level_operations
                .document_erase_transition,
            expected_erase_version,
            "erase action conversion at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_erase_transition_structure_validation,
            expected_erase_version,
            "erase structure validation at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_erase_transition_state_validation,
            expected_erase_version,
            "erase state validation at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .fetch_keep_history_document_lifecycle,
            expected_erase_version,
            "lifecycle state read at protocol {protocol}"
        );
        assert_eq!(
            version
                .dpp
                .state_transition_serialization_versions
                .document_erase_state_transition
                .as_ref()
                .map(|bounds| bounds.bounds.default_current_version),
            expected_erase_version,
            "erase wire bounds at protocol {protocol}"
        );
    }
}
