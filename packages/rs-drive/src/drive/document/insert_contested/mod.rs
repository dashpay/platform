//! Insert Documents.
//!
//! This module implements functions in Drive relevant to inserting documents.
//!

// Module: add_contested_document
// This module contains functionality for adding a document
mod add_contested_document;

// Module: add_contested_document_for_contract
// This module contains functionality for adding a document for a given contract
mod add_contested_document_for_contract;

// Module: add_contested_document_for_contract_apply_and_add_to_operations
// This module contains functionality for applying and adding operations for a contract document
mod add_contested_document_for_contract_apply_and_add_to_operations;

// Module: add_contested_document_for_contract_operations
// This module contains functionality for adding a document for contract operations
mod add_contested_document_for_contract_operations;

// Module: add_contested_document_to_primary_storage
// This module contains functionality for adding a document to primary storage
mod add_contested_document_to_primary_storage;

// Module: add_contested_indices_for_index_level_for_contract_operations
// This module contains functionality for adding indices for an index level for contract operations
// mod add_contested_indices_for_index_level_for_contract_operations;

// Module: add_contested_indices_for_top_index_level_for_contract_operations
// This module contains functionality for adding indices for the top index level for contract operations
mod add_contested_indices_for_contract_operations;

// Module: add_contested_reference_and_vote_subtree_to_document_operations
// This module contains functionality for adding a reference for an index level for contract operations
mod add_contested_reference_and_vote_subtree_to_document_operations;
mod add_contested_vote_subtrees_for_non_identities_operations;

// TODO: Disabled module add_contested_indices_for_index_level_for_contract_operations

#[cfg(test)]
mod tests {
    use std::option::Option::None;

    use dpp::block::block_info::BlockInfo;
    use rand::random;

    use crate::drive::document::tests::setup_dashpay;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;

    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use dpp::tests::json_document::json_document_to_document;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_add_dashpay_conflicting_unique_index_documents() {
        let (drive, dashpay) = setup_dashpay("add_conflict", true);

        let document_type = dashpay
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let random_owner_id = random::<[u8; 32]>();

        let platform_version = PlatformVersion::first();

        let dashpay_cr_document_0 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        let dashpay_cr_document_0_dup = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0-dup-unique-index.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &dashpay_cr_document_0,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: None,
                    },
                    contract: &dashpay,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert a document successfully");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &dashpay_cr_document_0_dup,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: None,
                    },
                    contract: &dashpay,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect_err(
                "expected not to be able to insert document with already existing unique index",
            );
    }

    /// A resource contested once must be contestable again after its poll
    /// ends, even though the poll-end cleanup leaves the abstain and lock
    /// vote trees behind when nobody voted that way.
    mod restart_after_cleanup {
        use super::*;
        use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
        use crate::error::drive::DriveError;
        use crate::error::Error;
        use crate::util::object_size_info::DataContractOwnedResolvedInfo;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
        use dpp::data_contract::document_type::random_document::{
            CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
        };
        use dpp::data_contract::DataContract;
        use dpp::document::{Document, DocumentV0Setters};
        use dpp::fee::fee_result::FeeResult;
        use dpp::identifier::Identifier;
        use dpp::platform_value::{Bytes32, Value};
        use dpp::prelude::TimestampMillis;
        use dpp::tests::fixtures::get_dpns_data_contract_fixture;
        use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        use std::collections::BTreeMap;

        fn quantum_domain_document(
            dpns_contract: &DataContract,
            owner_id: Identifier,
            entropy: Bytes32,
            rng: &mut StdRng,
            platform_version: &PlatformVersion,
        ) -> Document {
            let document_type = dpns_contract
                .document_type_for_name("domain")
                .expect("domain should exist on DPNS");

            let mut document = document_type
                .random_document_with_params(
                    owner_id,
                    entropy,
                    Some(1),
                    Some(1),
                    Some(1),
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::MinDocumentFillSize,
                    rng,
                    platform_version,
                )
                .expect("random document");

            document.set("parentDomainName", "dash".into());
            document.set("normalizedParentDomainName", "dash".into());
            document.set("label", "quantum".into());
            document.set("normalizedLabel", "quantum".into());
            document.set("records.identity", owner_id.into());
            document.set("subdomainRules.allowSubdomains", false.into());

            document
        }

        /// Opens a contest for `dash.quantum`, runs the poll-end cleanup as
        /// `clean_up_after_contested_resources_vote_polls_end` does for a
        /// contest whose only contender received no votes and nobody voted
        /// abstain or lock, then opens a new contest for the same name.
        fn restart_contest_after_cleanup_without_abstain_or_lock_votes(
            platform_version: &PlatformVersion,
        ) -> Result<FeeResult, Error> {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let dpns_contract =
                get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            drive
                .apply_contract(
                    &dpns_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("applied dpns contract");

            let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
                contract: DataContractOwnedResolvedInfo::OwnedDataContract(dpns_contract.clone()),
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![
                    Value::Text("dash".to_string()),
                    Value::Text("quantum".to_string()),
                ],
            };

            let mut rng = StdRng::seed_from_u64(433);
            let first_owner_id = Identifier::from([0x11; 32]);
            let first_document = quantum_domain_document(
                &dpns_contract,
                first_owner_id,
                Bytes32::random_with_rng(&mut rng),
                &mut rng,
                platform_version,
            );

            drive
                .add_contested_document(
                    OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &first_document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(first_owner_id.to_buffer()),
                    },
                    vote_poll.clone(),
                    false,
                    None,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to open the first contest");

            // The voter map the poll-end hook builds: contenders always
            // appear, abstain and lock only when they received votes.
            let end_time: TimestampMillis = 1;
            let votes: BTreeMap<ResourceVoteChoice, Vec<Identifier>> =
                BTreeMap::from([(ResourceVoteChoice::TowardsIdentity(first_owner_id), vec![])]);
            let finished_polls = [(&vote_poll, &end_time, &votes)];

            let mut cleanup_operations = vec![];
            drive.remove_contested_resource_vote_poll_votes_operations(
                &finished_polls,
                true,
                &mut cleanup_operations,
                None,
                platform_version,
            )?;
            drive.remove_contested_resource_vote_poll_documents_operations(
                &finished_polls,
                false,
                &mut cleanup_operations,
                None,
                platform_version,
            )?;
            drive.remove_contested_resource_vote_poll_contenders_operations(
                &finished_polls,
                &mut cleanup_operations,
                None,
                platform_version,
            )?;
            drive.apply_batch_low_level_drive_operations(
                None,
                None,
                cleanup_operations,
                &mut vec![],
                &platform_version.drive,
            )?;

            let second_owner_id = Identifier::from([0x22; 32]);
            let second_document = quantum_domain_document(
                &dpns_contract,
                second_owner_id,
                Bytes32::random_with_rng(&mut rng),
                &mut rng,
                platform_version,
            );

            drive.add_contested_document(
                OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &second_document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(second_owner_id.to_buffer()),
                },
                vote_poll,
                false,
                None,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
        }

        #[test]
        fn a_resource_can_be_contested_again_over_the_orphaned_vote_trees() {
            restart_contest_after_cleanup_without_abstain_or_lock_votes(PlatformVersion::latest())
                .expect("expected the new contest to reuse the orphaned vote trees");
        }

        /// PROTOCOL_VERSION_13 keeps v0, which trips over the orphaned tree.
        /// Pinned so the replay boundary stays explicit.
        #[test]
        fn a_resource_can_not_be_contested_again_at_protocol_version_13() {
            let platform_version = PlatformVersion::get(13).expect("expected platform version 13");

            let err = restart_contest_after_cleanup_without_abstain_or_lock_votes(platform_version)
                .expect_err("expected the new contest to fail on the orphaned vote tree");

            assert!(
                matches!(err, Error::Drive(DriveError::CorruptedContractIndexes(_))),
                "unexpected error: {err:?}"
            );
        }
    }

    /// Tests covering the error branches of the contested-document insertion
    /// path that aren't already reached by the happy-path integration tests.
    mod error_paths {
        use super::*;
        use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
        use crate::error::document::DocumentError;
        use crate::error::drive::DriveError;
        use crate::error::Error;
        use crate::util::object_size_info::{
            DataContractOwnedResolvedInfo, DocumentAndContractInfo, OwnedDocumentInfo,
        };
        use crate::util::storage_flags::StorageFlags;
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
        use dpp::data_contract::document_type::random_document::{
            CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
        };
        use dpp::identifier::Identifier;
        use dpp::platform_value::{Bytes32, Value};
        use dpp::tests::fixtures::get_dpns_data_contract_fixture;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        /// Hitting `add_contested_document` with a contract that was never
        /// applied to the drive must surface as
        /// `Error::Document(DocumentError::DataContractNotFound)`.
        #[test]
        fn add_contested_document_returns_data_contract_not_found_for_missing_contract() {
            let platform_version = PlatformVersion::latest();
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            // Build a DPNS fixture contract but do NOT apply it to the drive.
            let dpns_contract =
                get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            let document_type = dpns_contract
                .document_type_for_name("domain")
                .expect("domain should exist on DPNS");

            // Build a throwaway document instance so `DocumentRefInfo` has
            // something to borrow. We need all timestamp-required fields to
            // be populated for DPNS domain, so use `random_document_with_params`.
            let mut rng = StdRng::seed_from_u64(1);
            let doc = document_type
                .random_document_with_params(
                    Identifier::from([0x01; 32]),
                    Bytes32::default(),
                    Some(0),
                    Some(0),
                    Some(0),
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::MinDocumentFillSize,
                    &mut rng,
                    platform_version,
                )
                .expect("random document");

            let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
                contract: DataContractOwnedResolvedInfo::OwnedDataContract(dpns_contract.clone()),
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![
                    Value::Text("dash".to_string()),
                    Value::Text("alice".to_string()),
                ],
            };

            let err = drive
                .add_contested_document(
                    OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &doc,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: None,
                    },
                    vote_poll,
                    false,
                    None,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect_err("unknown contract should fail early");
            match err {
                Error::Document(DocumentError::DataContractNotFound) => {}
                other => panic!("unexpected error: {other:?}"),
            }
        }

        /// `add_contested_document_for_contract` must bail with
        /// `ContestedDocumentMissingOwnerId` when the caller forgets to set
        /// the owner_id on `OwnedDocumentInfo` (needed for the contested
        /// index, not for the primary storage).
        #[test]
        fn add_contested_document_for_contract_requires_owner_id() {
            let platform_version = PlatformVersion::latest();
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let dpns_contract =
                get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            drive
                .apply_contract(
                    &dpns_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("applied dpns contract");

            let document_type = dpns_contract
                .document_type_for_name("domain")
                .expect("domain should exist on DPNS");

            // The fields of this doc aren't serialized until primary storage
            // insertion, and we fail earlier on owner_id.
            let mut rng = StdRng::seed_from_u64(7);
            let doc = document_type
                .random_document_with_params(
                    Identifier::from([0x10; 32]),
                    Bytes32::default(),
                    Some(0),
                    Some(0),
                    Some(0),
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::MinDocumentFillSize,
                    &mut rng,
                    platform_version,
                )
                .expect("random document");

            let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
                contract: DataContractOwnedResolvedInfo::OwnedDataContract(dpns_contract.clone()),
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![
                    Value::Text("dash".to_string()),
                    Value::Text("alice".to_string()),
                ],
            };

            // Because DPNS domain has `transferred_at` as a required
            // document-level field but the `random_document_with_params`
            // helper currently leaves it `None`, the failure observed here
            // is a Protocol MissingRequiredKey. That still validates that
            // the pipeline short-circuits cleanly *before* touching grovedb.
            // Either way the key invariant we care about -- nothing was
            // inserted into primary storage -- holds.
            let result = drive.add_contested_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &doc,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: None,
                    },
                    contract: &dpns_contract,
                    document_type,
                },
                vote_poll,
                false,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            );
            assert!(
                result.is_err(),
                "contested insert must fail without owner_id/required fields"
            );
        }

        /// `add_contested_document_for_contract` targeting a document type
        /// that has no contested index (`preorder` in DPNS) must surface
        /// `DriveError::ContestedIndexNotFound` from
        /// `add_contested_indices_for_contract_operations_v0`.
        #[test]
        fn add_contested_document_for_contract_errors_on_missing_contested_index() {
            let platform_version = PlatformVersion::latest();
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let dpns_contract =
                get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            drive
                .apply_contract(
                    &dpns_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("applied dpns contract");

            // preorder has a single unique index (`saltedHash`) but no
            // contested index.
            let document_type = dpns_contract
                .document_type_for_name("preorder")
                .expect("preorder should exist on DPNS");

            let mut rng = StdRng::seed_from_u64(5);
            let doc = document_type
                .random_document_with_rng(&mut rng, platform_version)
                .expect("random preorder document");

            let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
                contract: DataContractOwnedResolvedInfo::OwnedDataContract(dpns_contract.clone()),
                document_type_name: "preorder".to_string(),
                index_name: "saltedHash".to_string(),
                index_values: vec![Value::Bytes(vec![0u8; 32])],
            };

            let result = drive.add_contested_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &doc,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some([0x42; 32]),
                    },
                    contract: &dpns_contract,
                    document_type,
                },
                vote_poll,
                false,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            );
            match result {
                Err(Error::Drive(DriveError::ContestedIndexNotFound(_))) => {}
                other => panic!("expected ContestedIndexNotFound, got: {other:?}"),
            }
        }
    }
}
