//! Semantic pins for the version slots the keep-history document lifecycle
//! rides on.
//!
//! A numeric pin alone would not say what the number means, so each assertion
//! below names the behaviour the slot selects and the released protocol
//! versions it must leave alone.

use platform_version::version::PlatformVersion;

/// The released protocol versions parse contracts with the generation that
/// predates the lifecycle grammar and refuse a keep-history delete before it
/// reaches storage. Both must replay identically forever.
#[test]
fn should_preserve_released_keep_history_validation_versions() {
    for protocol in [12, 13] {
        let version = PlatformVersion::get(protocol).unwrap();
        assert_eq!(
            version
                .dpp
                .contract_versions
                .document_type_versions
                .class_method_versions
                .try_from_schema,
            2,
            "parser at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_delete_transition_structure_validation,
            0,
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
        assert_eq!(
            version
                .dpp
                .state_transition_serialization_versions
                .batch_state_transition
                .max_version,
            1,
            "batch wire versions at protocol {protocol}"
        );
        assert_eq!(
            version
                .dpp
                .state_transitions
                .documents
                .documents_batch_transition
                .validation
                .validate_base_structure,
            0,
            "batch structure validation at protocol {protocol}"
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
        assert_eq!(
            version
                .drive
                .methods
                .verify
                .state_transition
                .verify_state_transition_was_executed_with_proof,
            0,
            "state transition proof verification at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .transform_into_action,
            1,
            "batch action transform at protocol {protocol}"
        );
    }
}

/// Protocol 14 selects the lifecycle: the parser generation that admits the new
/// keywords, the delete that consults the lifecycle, and the erase kind with
/// the bound on how much one transition may remove.
#[test]
fn should_activate_keep_history_validation_at_protocol_14() {
    let version = PlatformVersion::get(14).unwrap();
    assert_eq!(
        version
            .dpp
            .contract_versions
            .document_type_versions
            .class_method_versions
            .try_from_schema,
        3
    );
    assert_eq!(
        version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .document_delete_transition_structure_validation,
        1
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
            .drive
            .methods
            .document
            .delete
            .delete_document_for_contract_operations,
        1,
        "protocol 14 selects the keep-history delete branch"
    );
    assert_eq!(
        version
            .dpp
            .state_transition_serialization_versions
            .batch_state_transition
            .max_version,
        2,
        "protocol 14 admits BatchTransitionV2"
    );
    assert_eq!(
        version
            .dpp
            .state_transition_serialization_versions
            .batch_state_transition
            .default_current_version,
        2,
        "protocol 14 constructs BatchTransitionV2"
    );
    assert_eq!(
        version
            .dpp
            .state_transitions
            .documents
            .documents_batch_transition
            .validation
            .validate_base_structure,
        1,
        "protocol 14 selects structure validation that admits erase only in V2"
    );
    assert_eq!(
        version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .document_delete_transition,
        1,
        "protocol 14 selects the lifecycle-specific delete operation"
    );
    assert_eq!(
        version
            .drive
            .methods
            .verify
            .state_transition
            .verify_state_transition_was_executed_with_proof,
        1,
        "protocol 14 selects erase-aware proof verification"
    );
    assert_eq!(
        version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .transform_into_action,
        2,
        "protocol 14 selects the erase-aware transformer"
    );
    assert_eq!(
        version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .advanced_structure,
        1,
        "protocol 14 selects erase-aware advanced validation"
    );
    assert_eq!(
        version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .state,
        1,
        "protocol 14 selects erase-aware state validation"
    );

    let bounds = version
        .dpp
        .state_transition_serialization_versions
        .document_erase_state_transition
        .as_ref()
        .expect("the erase kind joins the wire at protocol 14");
    assert_eq!(bounds.bounds.min_version, 0);
    assert_eq!(bounds.bounds.max_version, 0);
    assert_eq!(bounds.bounds.default_current_version, 0);

    let chunk = version
        .system_limits
        .max_document_revisions_erased_per_transition
        .expect("protocol 14 bounds the erase chunk");
    assert_eq!(
        chunk, 100,
        "the chunk bounds the work one erase can demand and the balance every \
         erase needs up front; changing it changes both"
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
        let expected_erase_version = (protocol >= 14).then_some(0);
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
    }
}
