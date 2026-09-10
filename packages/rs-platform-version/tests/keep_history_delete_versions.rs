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
        assert_eq!(
            version
                .drive
                .methods
                .document
                .query
                .fetch_document_lifecycle,
            0,
            "lifecycle read at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive
                .methods
                .document
                .delete
                .erase_document_for_contract_operations,
            0,
            "erase operation at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_erase_transition_structure_validation,
            0,
            "erase structure validation at protocol {protocol}"
        );
        assert_eq!(
            version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_erase_transition_state_validation,
            0,
            "erase state validation at protocol {protocol}"
        );
    }
}
