use platform_version::version::PlatformVersion;

#[test]
fn should_keep_released_document_history_storage_and_proofs_unchanged() {
    for protocol in [12, 13] {
        let version = PlatformVersion::get(protocol).unwrap();
        let document = &version.drive.methods.document;
        assert_eq!(document.insert.add_document_to_primary_storage, 0);
        assert_eq!(
            document
                .estimation_costs
                .add_estimation_costs_for_add_document_to_primary_storage,
            0
        );
        assert_eq!(document.query.fetch_document_history_query, 0);
        assert_eq!(document.query.fetch_document_history, 0);
        assert_eq!(document.query.prove_document_history, 0);
        assert_eq!(
            version
                .drive
                .methods
                .verify
                .document
                .verify_document_history,
            0
        );
        assert_eq!(version.drive_abci.query.document_history.max_version, 0);
        assert_ne!(
            version
                .drive_abci
                .methods
                .protocol_upgrade
                .perform_events_on_first_block_of_protocol_change,
            Some(2)
        );
    }
}

#[test]
fn should_activate_storage_migration_and_history_proofs_together() {
    let version = PlatformVersion::get(14).unwrap();
    let document = &version.drive.methods.document;
    assert_eq!(document.insert.add_document_to_primary_storage, 1);
    assert_eq!(
        document
            .estimation_costs
            .add_estimation_costs_for_add_document_to_primary_storage,
        1
    );
    assert_eq!(document.query.fetch_document_history_query, 1);
    assert_eq!(document.query.fetch_document_history, 1);
    assert_eq!(document.query.prove_document_history, 1);
    assert_eq!(
        version
            .drive
            .methods
            .verify
            .document
            .verify_document_history,
        1
    );
    assert_eq!(version.drive_abci.query.document_history.max_version, 1);
    assert_eq!(
        version
            .drive_abci
            .methods
            .protocol_upgrade
            .perform_events_on_first_block_of_protocol_change,
        Some(2)
    );
}
