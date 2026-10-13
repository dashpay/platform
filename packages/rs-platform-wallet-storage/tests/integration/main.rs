//! The crate's integration tests, built as one test binary.
//!
//! Each file in this directory is a module of a single test crate rather than
//! a test crate of its own. Every separate integration-test binary links the
//! crate's whole dependency graph, and with one binary per file the links
//! dominated CI build time. A new test file must be declared here, or it is
//! not compiled.

mod common;

mod fixture_gen;
mod persisted_core_transactions;
mod persistence_error_kind_mapping;
mod review_4651_concurrent_payment;
mod secrets_api;
mod secrets_default_on_compiles;
mod secrets_guard;
mod secrets_mock_store_test_util;
mod secrets_scan;
mod sqlite_account_zero_attribution;
mod sqlite_accounts_reader;
mod sqlite_asset_locks_filter;
mod sqlite_auto_backup;
mod sqlite_backup_restore;
mod sqlite_blob_roundtrip_coverage;
mod sqlite_blob_size_gate_on_load;
mod sqlite_buffer_semantics;
mod sqlite_check_constraints;
mod sqlite_cli_smoke;
mod sqlite_commit_writes_lock_poison_shortcircuit;
mod sqlite_compile_time;
mod sqlite_contacts_keys_rehydration;
mod sqlite_core_pool_writer;
mod sqlite_core_state_reader;
mod sqlite_dashpay_overlay_contract;
mod sqlite_delete_buffer_reconcile;
mod sqlite_delete_cross_process_exclusion;
mod sqlite_delete_partial_commit_window;
mod sqlite_delete_real_apply_failure;
mod sqlite_delete_wallet;
mod sqlite_delete_wallet_constraint_carveout;
mod sqlite_deleted_wallet_residue;
mod sqlite_error_classification;
mod sqlite_fk_changeset_ordering;
mod sqlite_foreign_db_rejection;
mod sqlite_foreign_keys;
mod sqlite_identity_hard_delete;
mod sqlite_identity_index_concurrency;
mod sqlite_identity_index_uniqueness;
mod sqlite_identity_keys_reader;
mod sqlite_identity_scan_state;
mod sqlite_legacy_state_migration;
mod sqlite_load_reconstruction;
mod sqlite_load_wiring;
mod sqlite_migration_execution;
mod sqlite_migrations;
mod sqlite_money_column_overflow_on_read;
mod sqlite_object_metadata;
mod sqlite_open_integrity_check;
mod sqlite_open_version_gate;
mod sqlite_permissions;
mod sqlite_persist_roundtrip;
mod sqlite_pool_reader;
mod sqlite_provider_key_accounts;
mod sqlite_qa_identity_removal;
mod sqlite_recovery_mode;
mod sqlite_recovery_mode_write_block;
mod sqlite_restore_cross_process_exclusion;
mod sqlite_restore_open_path_guard;
mod sqlite_restore_staged_validation;
mod sqlite_schema_pinning;
mod sqlite_second_open_guard;
mod sqlite_sent_payment_verdict_durability;
mod sqlite_shielded_viewing_keys;
mod sqlite_store_flush_seam;
mod sqlite_store_generation;
mod sqlite_structural_hardening;
mod sqlite_trait_dispatch;
mod sqlite_transaction_sweeps;
mod sqlite_unowned_identities;
mod sqlite_used_core_addresses;
mod sqlite_v008_isolation;
mod sqlite_v008_migration;
mod sqlite_version_bump;
mod sqlite_wallet_db_identity;
mod tracked_masternodes_roundtrip;
