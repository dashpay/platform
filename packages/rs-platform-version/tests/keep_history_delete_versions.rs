//! The lifecycle's admission, storage, and bounds must activate together.

use platform_version::version::{PlatformVersion, PLATFORM_VERSIONS};

#[test]
fn should_activate_lifecycle_handlers_and_limits_only_with_erase_wire_bounds() {
    for version in PLATFORM_VERSIONS {
        let protocol = version.protocol_version;
        let expected = (protocol >= 15).then_some(0);
        let document = &version.drive.methods.document;
        let batch = &version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition;
        let conversion = &version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations;
        let bounds = &version
            .dpp
            .state_transition_serialization_versions
            .document_erase_state_transition;
        assert_eq!(
            bounds
                .as_ref()
                .map(|bounds| bounds.bounds.default_current_version),
            expected,
            "erase wire at {protocol}"
        );
        for slot in [
            document.query.fetch_document_lifecycle,
            document.delete.erase_document_for_contract_operations,
            document.delete.add_estimation_costs_for_erase_document,
            document.delete.add_estimation_costs_for_lifecycle_record,
            conversion.document_erase_transition,
            batch.document_erase_transition_structure_validation,
            batch.document_erase_transition_state_validation,
        ] {
            assert_eq!(slot, expected, "lifecycle handler at {protocol}");
        }
        assert_eq!(
            version
                .system_limits
                .max_document_revisions_erased_per_transition,
            (protocol >= 15).then_some(100),
            "erase bound at {protocol}"
        );
        if protocol <= 14 {
            assert_eq!(document.delete.delete_document_for_contract_operations, 0);
            assert_eq!(conversion.document_delete_transition, 0);
        }
    }
}

#[test]
fn should_extend_the_current_protocol_fifteen_tables_for_lifecycle() {
    let version = PlatformVersion::latest();
    assert_eq!(version.protocol_version, 15);
    let contract = &version.dpp.contract_versions;
    assert_eq!(
        contract
            .document_type_versions
            .class_method_versions
            .try_from_schema,
        4
    );
    assert_eq!(
        contract.document_type_versions.schema.document_type_schema,
        3
    );
    assert_eq!(contract.contract_storage_document_type_version, 0);
    let document = &version.drive.methods.document;
    assert_eq!(document.insert.add_document_for_contract_operations, 2);
    assert_eq!(document.update.update_document_for_contract_operations, 2);
    assert_eq!(document.delete.delete_document_for_contract_operations, 1);
    assert_eq!(document.delete.remove_document_from_primary_storage, 1);
    let batch = &version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition;
    assert_eq!(batch.document_create_transition_state_validation, 3);
    assert_eq!(batch.document_delete_transition_state_validation, 1);
    assert_eq!(batch.document_delete_transition_structure_validation, 2);
    let serialization = &version.dpp.state_transition_serialization_versions;
    assert_eq!(
        serialization
            .document_base_state_transition
            .default_current_version,
        2
    );
    assert_eq!(
        serialization.batch_state_transition.default_current_version,
        1
    );
}

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
            "delete validation at protocol {protocol}"
        );
    }
}

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
}
