use platform_version::version::PlatformVersion;

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
        4
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
