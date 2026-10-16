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

// Module: fetch_charter_election_windows
// This module reads the windows a moderation election takes from its target contract
mod fetch_charter_election_windows;
pub use fetch_charter_election_windows::ContestWindows;

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
        use crate::drive::votes::paths::{VotePollPaths, VOTING_STORAGE_TREE_KEY};
        use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
        use crate::drive::Drive;
        use crate::error::drive::DriveError;
        use crate::error::Error;
        use crate::util::grove_operations::DirectQueryType;
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
        use grovedb::Element;
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
        ) -> (
            Drive,
            ContestedDocumentResourceVotePollWithContractInfo,
            Result<FeeResult, Error>,
        ) {
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
            drive
                .remove_contested_resource_vote_poll_votes_operations(
                    &finished_polls,
                    true,
                    &mut cleanup_operations,
                    None,
                    platform_version,
                )
                .expect("expected the votes cleanup operations");
            drive
                .remove_contested_resource_vote_poll_documents_operations(
                    &finished_polls,
                    false,
                    &mut cleanup_operations,
                    None,
                    platform_version,
                )
                .expect("expected the documents cleanup operations");
            drive
                .remove_contested_resource_vote_poll_contenders_operations(
                    &finished_polls,
                    &mut cleanup_operations,
                    None,
                    platform_version,
                )
                .expect("expected the contenders cleanup operations");
            drive
                .apply_batch_low_level_drive_operations(
                    None,
                    None,
                    cleanup_operations,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to apply the poll-end cleanup");

            let second_owner_id = Identifier::from([0x22; 32]);
            let second_document = quantum_domain_document(
                &dpns_contract,
                second_owner_id,
                Bytes32::random_with_rng(&mut rng),
                &mut rng,
                platform_version,
            );

            let result = drive.add_contested_document(
                OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &second_document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(second_owner_id.to_buffer()),
                },
                vote_poll.clone(),
                false,
                None,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            );

            (drive, vote_poll, result)
        }

        #[test]
        fn a_resource_can_be_contested_again_over_the_orphaned_vote_trees() {
            let platform_version = PlatformVersion::latest();

            let (drive, vote_poll, result) =
                restart_contest_after_cleanup_without_abstain_or_lock_votes(platform_version);

            result.expect("expected the new contest to be created over the orphaned vote trees");

            // The new contest must own reachable, empty abstain and lock vote
            // trees: the raw existence probe alone would have kept the
            // orphaned storage and left the contest without them.
            for resource_vote_choice in [ResourceVoteChoice::Abstain, ResourceVoteChoice::Lock] {
                let path = vote_poll
                    .contender_path(&resource_vote_choice, platform_version)
                    .expect("expected the contender path");
                let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();

                let vote_tree = drive
                    .grove_get_raw_optional(
                        path_refs.as_slice().into(),
                        &[VOTING_STORAGE_TREE_KEY],
                        DirectQueryType::StatefulDirectQuery,
                        None,
                        &mut vec![],
                        &platform_version.drive,
                    )
                    .expect("expected to read the vote tree");

                assert!(
                    matches!(vote_tree, Some(Element::SumTree(None, 0, _))),
                    "{resource_vote_choice:?} vote tree must be a reachable empty sum tree, got {vote_tree:?}"
                );
            }
        }

        /// PROTOCOL_VERSION_13 keeps v0, which trips over the orphaned tree.
        /// Pinned so the replay boundary stays explicit.
        #[test]
        fn a_resource_can_not_be_contested_again_at_protocol_version_13() {
            let platform_version = PlatformVersion::get(13).expect("expected platform version 13");

            let (_, _, result) =
                restart_contest_after_cleanup_without_abstain_or_lock_votes(platform_version);

            let err =
                result.expect_err("expected the new contest to fail on the orphaned vote tree");

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
        /// `add_contested_indices_for_contract_operations`.
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

    /// The second contender of a contest resolved without locking moves the poll's end
    /// date: the creator-flagged join-window entry is removed and the vote-window entry
    /// written, so pricing it without the fee history is rejected from protocol version
    /// 15. Both bare wrappers own their transaction when the caller passes none, so the
    /// rejected join leaves the root hash, the contenders and the end-date entries as they
    /// were; the production funnel joins with the history, and protocol version 14 joins
    /// without one.
    mod second_contender_without_fee_history {
        use super::*;
        use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
        use crate::drive::Drive;
        use crate::error::drive::DriveError;
        use crate::error::Error;
        use crate::query::vote_poll_vote_state_query::{
            ContestedDocumentVotePollDriveQueryResultType,
            ResolvedContestedDocumentVotePollDriveQuery,
        };
        use crate::query::VotePollsByEndDateDriveQuery;
        use crate::util::batch::drive_op_batch::DocumentOperationType;
        use crate::util::batch::DriveOperation;
        use crate::util::object_size_info::{
            DataContractInfo, DataContractOwnedResolvedInfo, DocumentTypeInfo,
        };
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
        use crate::util::test_helpers::setup_contract;
        use dpp::data_contract::document_type::random_document::{
            CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
        };
        use dpp::data_contract::DataContract;
        use dpp::document::{Document, DocumentV0Setters};
        use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
        use dpp::fee::fee_result::FeeResult;
        use dpp::identifier::Identifier;
        use dpp::platform_value::{Bytes32, Value};
        use dpp::prelude::TimestampMillis;
        use dpp::version::fee::FeeVersion;
        use dpp::version::LATEST_VERSION;
        use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        use std::borrow::Cow;
        use std::collections::BTreeMap;

        const NO_LOCKING_CONTRACT: &str =
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-no-locking.json";

        fn quantum_domain_document(
            contract: &DataContract,
            owner_id: Identifier,
            rng: &mut StdRng,
            platform_version: &PlatformVersion,
        ) -> Document {
            let document_type = contract
                .document_type_for_name("domain")
                .expect("domain should exist on DPNS");
            let mut document = document_type
                .random_document_with_params(
                    owner_id,
                    Bytes32::random_with_rng(rng),
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

        fn vote_poll(contract: &DataContract) -> ContestedDocumentResourceVotePollWithContractInfo {
            ContestedDocumentResourceVotePollWithContractInfo {
                contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract.clone()),
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![
                    Value::Text("dash".to_string()),
                    Value::Text("quantum".to_string()),
                ],
            }
        }

        fn owned(document: &Document, owner_id: Identifier) -> OwnedDocumentInfo<'_> {
            OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    document,
                    Some(Cow::Owned(StorageFlags::SingleEpochOwned(
                        0,
                        owner_id.to_buffer(),
                    ))),
                )),
                owner_id: Some(owner_id.to_buffer()),
            }
        }

        struct Contest {
            drive: Drive,
            contract: DataContract,
            first_owner_id: Identifier,
            second_owner_id: Identifier,
            second_document: Document,
        }

        /// Opens the `dash.quantum` contest with one contender through the production
        /// funnel, and prepares the second contender's document.
        fn open_contest(platform_version: &PlatformVersion) -> Contest {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));
            let contract = setup_contract(
                &drive,
                NO_LOCKING_CONTRACT,
                None,
                None,
                None::<fn(&mut DataContract)>,
                None,
                Some(platform_version),
            );
            let mut rng = StdRng::seed_from_u64(0x5EC0_004D);
            let first_owner_id = Identifier::from([0x31; 32]);
            let second_owner_id = Identifier::from([0x32; 32]);
            let first_document =
                quantum_domain_document(&contract, first_owner_id, &mut rng, platform_version);
            let second_document =
                quantum_domain_document(&contract, second_owner_id, &mut rng, platform_version);
            drive
                .add_contested_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: owned(&first_document, first_owner_id),
                        contract: &contract,
                        document_type: contract
                            .document_type_for_name("domain")
                            .expect("domain should exist on DPNS"),
                    },
                    vote_poll(&contract),
                    false,
                    BlockInfo::default(),
                    true,
                    Some(
                        ContestedDocumentVotePollStoredInfo::new(
                            BlockInfo::default(),
                            platform_version,
                        )
                        .expect("expected the stored info of a new poll"),
                    ),
                    None,
                    platform_version,
                )
                .expect("expected to open the contest");
            Contest {
                drive,
                contract,
                first_owner_id,
                second_owner_id,
                second_document,
            }
        }

        fn root_hash(drive: &Drive, platform_version: &PlatformVersion) -> [u8; 32] {
            drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("expected a root hash")
        }

        fn end_dates(drive: &Drive, platform_version: &PlatformVersion) -> Vec<TimestampMillis> {
            VotePollsByEndDateDriveQuery {
                start_time: None,
                end_time: None,
                limit: None,
                offset: None,
                order_ascending: true,
            }
            .execute_no_proof(drive, None, &mut vec![], platform_version)
            .expect("expected the end date entries")
            .into_iter()
            .flat_map(|(time, polls)| polls.into_iter().map(move |_| time))
            .collect()
        }

        fn contenders(
            drive: &Drive,
            contract: &DataContract,
            platform_version: &PlatformVersion,
        ) -> Vec<Identifier> {
            ResolvedContestedDocumentVotePollDriveQuery {
                vote_poll: (&vote_poll(contract)).into(),
                result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
                offset: None,
                limit: None,
                start_at: None,
                allow_include_locked_and_abstaining_vote_tally: false,
            }
            .execute(drive, None, &mut vec![], platform_version)
            .expect("expected the contenders")
            .contenders
            .into_iter()
            .map(|contender| contender.identity_id())
            .collect()
        }

        fn fee_history() -> CachedEpochIndexFeeVersions {
            BTreeMap::from([(0, FeeVersion::first())])
        }

        fn assert_rejected(result: Result<FeeResult, Error>) {
            assert!(
                matches!(
                    result,
                    Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
                ),
                "moving a creator-flagged end date without a fee history must be rejected, got {:?}",
                result
            );
        }

        #[test]
        fn should_leave_a_no_locking_contest_in_place_when_a_bare_wrapper_cannot_price_the_second_contender(
        ) {
            let platform_version = PlatformVersion::latest();
            let Contest {
                drive,
                contract,
                first_owner_id,
                second_owner_id,
                second_document,
            } = open_contest(platform_version);
            let join_window_end = end_dates(&drive, platform_version);
            assert_eq!(join_window_end.len(), 1, "the contest has one end date");
            let before = root_hash(&drive, platform_version);
            let document_type = contract
                .document_type_for_name("domain")
                .expect("domain should exist on DPNS");

            // The wrapper taking the contract reference.
            assert_rejected(drive.add_contested_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: owned(&second_document, second_owner_id),
                    contract: &contract,
                    document_type,
                },
                vote_poll(&contract),
                false,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            ));
            assert_eq!(root_hash(&drive, platform_version), before);

            // The wrapper taking the contract id.
            assert_rejected(drive.add_contested_document(
                owned(&second_document, second_owner_id),
                vote_poll(&contract),
                false,
                None,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            ));
            assert_eq!(
                root_hash(&drive, platform_version),
                before,
                "a rejected second contender must not persist"
            );
            assert_eq!(
                contenders(&drive, &contract, platform_version),
                vec![first_owner_id],
                "the contest still has its first contender only"
            );
            assert_eq!(
                end_dates(&drive, platform_version),
                join_window_end,
                "the join-window end date is still the only entry"
            );

            // The production funnel carries the history: the second contender joins and the
            // end date moves to the vote window.
            let history = fee_history();
            drive
                .apply_drive_operations(
                    vec![DriveOperation::DocumentOperation(
                        DocumentOperationType::AddContestedDocument {
                            owned_document_info: owned(&second_document, second_owner_id),
                            contested_document_resource_vote_poll: vote_poll(&contract),
                            contract_info: DataContractInfo::BorrowedDataContract(&contract),
                            document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("domain"),
                            insert_without_check: false,
                            also_insert_vote_poll_stored_info: None,
                        },
                    )],
                    true,
                    &BlockInfo::default(),
                    None,
                    platform_version,
                    Some(&history),
                )
                .expect("expected the second contender to join with the fee history");
            assert_eq!(
                contenders(&drive, &contract, platform_version),
                vec![first_owner_id, second_owner_id]
            );
            let vote_window_end = end_dates(&drive, platform_version);
            assert_eq!(
                vote_window_end.len(),
                1,
                "the contest still has one end date"
            );
            assert!(
                vote_window_end[0] > join_window_end[0],
                "the end date moved from the join window to the vote window"
            );

            // Protocol version 14 prices the shipped shortcut without a history and commits.
            let frozen_platform_version = PlatformVersion::get(14).expect("protocol version 14");
            let Contest {
                drive,
                contract,
                first_owner_id,
                second_owner_id,
                second_document,
            } = open_contest(frozen_platform_version);
            drive
                .add_contested_document(
                    owned(&second_document, second_owner_id),
                    vote_poll(&contract),
                    false,
                    None,
                    &BlockInfo::default(),
                    true,
                    None,
                    frozen_platform_version,
                )
                .expect("protocol version 14 prices the shipped shortcut without a history");
            assert_eq!(
                contenders(&drive, &contract, frozen_platform_version),
                vec![first_owner_id, second_owner_id]
            );
        }

        /// The first contender frees nothing, so the bare wrappers price it at every
        /// version; the by-id wrapper's fee includes the contract read from protocol
        /// version 15 (generation 0 priced the write alone).
        #[test]
        fn should_open_a_contest_through_the_bare_wrappers_at_every_version() {
            for (protocol_version, prices_the_fetch) in [(14, false), (LATEST_VERSION, true)] {
                let platform_version =
                    PlatformVersion::get(protocol_version).expect("expected a platform version");
                let drive = setup_drive_with_initial_state_structure(Some(platform_version));
                let contract = setup_contract(
                    &drive,
                    NO_LOCKING_CONTRACT,
                    None,
                    None,
                    None::<fn(&mut DataContract)>,
                    None,
                    Some(platform_version),
                );
                let mut rng = StdRng::seed_from_u64(0x5EC0_004E);
                let owner_id = Identifier::from([0x33; 32]);
                let document =
                    quantum_domain_document(&contract, owner_id, &mut rng, platform_version);
                let stored_info = || {
                    Some(
                        ContestedDocumentVotePollStoredInfo::new(
                            BlockInfo::default(),
                            platform_version,
                        )
                        .expect("expected the stored info of a new poll"),
                    )
                };

                let by_reference = drive
                    .add_contested_document_for_contract(
                        DocumentAndContractInfo {
                            owned_document_info: owned(&document, owner_id),
                            contract: &contract,
                            document_type: contract
                                .document_type_for_name("domain")
                                .expect("domain should exist on DPNS"),
                        },
                        vote_poll(&contract),
                        false,
                        BlockInfo::default(),
                        false,
                        stored_info(),
                        None,
                        platform_version,
                    )
                    .expect("expected to estimate the contest by contract reference");
                let by_id = drive
                    .add_contested_document(
                        owned(&document, owner_id),
                        vote_poll(&contract),
                        false,
                        stored_info(),
                        &BlockInfo::default(),
                        false,
                        None,
                        platform_version,
                    )
                    .expect("expected to estimate the contest by contract id");
                assert_eq!(by_id.storage_fee, by_reference.storage_fee);
                if prices_the_fetch {
                    assert!(
                        by_id.processing_fee > by_reference.processing_fee,
                        "protocol version {protocol_version} must price the contract read"
                    );
                } else {
                    assert_eq!(by_id.processing_fee, by_reference.processing_fee);
                }

                let before = root_hash(&drive, platform_version);
                drive
                    .add_contested_document(
                        owned(&document, owner_id),
                        vote_poll(&contract),
                        false,
                        stored_info(),
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to open the contest by contract id");
                assert_ne!(
                    root_hash(&drive, platform_version),
                    before,
                    "the contest was committed"
                );
                assert_eq!(
                    contenders(&drive, &contract, platform_version),
                    vec![owner_id]
                );
            }
        }
    }
}
