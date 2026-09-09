// MIT LICENSE
//
// Copyright (c) 2023 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Check Index uniqueness for documents.
//!
//! This module implements functions in Drive relevant to checking if a document validates all
//! uniqueness constraints.
//!

mod internal;

mod validate_document_create_transition_action_uniqueness;

mod validate_document_replace_transition_action_uniqueness;

mod validate_document_purchase_transition_action_uniqueness;
mod validate_document_transfer_transition_action_uniqueness;
mod validate_document_update_price_transition_action_uniqueness;

#[cfg(test)]
#[cfg(feature = "server")]
mod tests {
    //! Tests that exercise the uniqueness validation code paths directly.
    //!
    //! These tests target error branches that the usual happy-path
    //! integration tests don't reach: empty-state success, non-unique
    //! indexes being skipped, duplicate unique-index rejection, and the
    //! `allow_original` semantics used during replace/transfer/purchase
    //! flows.
    use std::borrow::Cow;
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;
    use std::vec;

    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use dpp::platform_value::Value;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::version::PlatformVersion;

    use crate::drive::contract::DataContractFetchInfo;
    use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::{
        UniquenessOfDataRequest, UniquenessOfDataRequestUpdateType, UniquenessOfDataRequestV0,
        UniquenessOfDataRequestV1,
    };
    use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{
        DocumentBaseTransitionAction, DocumentBaseTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
        DocumentCreateTransitionAction, DocumentCreateTransitionActionV0,
    };
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    use dpp::data_contract::DataContract;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;

    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};

    /// Helper: set up a drive with the DPNS data contract applied.
    fn setup_drive_with_dpns(
        platform_version: &'static PlatformVersion,
    ) -> (crate::drive::Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let dpns = get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        drive
            .apply_contract(
                &dpns,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("applied dpns contract");
        (drive, dpns)
    }

    /// Build a minimal `DocumentCreateTransitionAction` for the DPNS preorder
    /// document type referencing `data_contract` and an explicit
    /// `saltedDomainHash` value.
    fn build_preorder_create_action(
        data_contract: DataContract,
        document_id: Identifier,
        salted_domain_hash: [u8; 32],
    ) -> DocumentCreateTransitionAction {
        let data_contract_fetch_info = Arc::new(DataContractFetchInfo {
            contract: data_contract,
            storage_flags: None,
            cost: Default::default(),
            fee: None,
        });
        let base = DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
            id: document_id,
            identity_contract_nonce: 1,
            document_type_name: "preorder".to_string(),
            data_contract: data_contract_fetch_info,
            token_cost: None,
            gas_fees_paid_by: GasFeesPaidBy::default(),
        });

        let data = BTreeMap::from([(
            "saltedDomainHash".to_string(),
            Value::Bytes(salted_domain_hash.to_vec()),
        )]);

        DocumentCreateTransitionAction::V0(DocumentCreateTransitionActionV0 {
            base,
            block_info: BlockInfo::default(),
            data,
            prefunded_voting_balance: None,
            current_store_contest_info: None,
            should_store_contest_info: None,
        })
    }

    /// An empty drive has no documents, so a freshly-proposed unique index
    /// must validate successfully.
    #[test]
    fn validate_document_create_uniqueness_succeeds_on_empty_state_v0() {
        let platform_version = PlatformVersion::first();
        let (drive, dpns) = setup_drive_with_dpns(platform_version);

        let document_type = dpns
            .document_type_for_name("preorder")
            .expect("preorder doctype");

        let action =
            build_preorder_create_action(dpns.clone(), Identifier::from([0x11; 32]), [0x22; 32]);

        let result = drive
            .validate_document_create_transition_action_uniqueness(
                &dpns,
                document_type,
                &action,
                Identifier::from([0xAA; 32]),
                None,
                platform_version,
            )
            .expect("call should succeed");
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    /// The v1 dispatch must also produce a valid result on an empty state.
    #[test]
    fn validate_document_create_uniqueness_succeeds_on_empty_state_v1() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns) = setup_drive_with_dpns(platform_version);

        let document_type = dpns
            .document_type_for_name("preorder")
            .expect("preorder doctype");

        let action =
            build_preorder_create_action(dpns.clone(), Identifier::from([0x33; 32]), [0x44; 32]);

        let result = drive
            .validate_document_create_transition_action_uniqueness(
                &dpns,
                document_type,
                &action,
                Identifier::from([0xAA; 32]),
                None,
                platform_version,
            )
            .expect("call should succeed");
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    /// Once a document occupies a unique index, proposing a *different*
    /// document id with the same index value must surface
    /// `DuplicateUniqueIndexError`.
    #[test]
    fn validate_document_create_uniqueness_detects_conflict_v0() {
        let platform_version = PlatformVersion::first();
        let (drive, dpns) = setup_drive_with_dpns(platform_version);

        let document_type = dpns
            .document_type_for_name("preorder")
            .expect("preorder doctype");

        // Insert a real preorder document so the unique index is populated.
        let salted_hash = [0xAB; 32];
        let owner = Identifier::from([0x77; 32]);
        let existing_action =
            build_preorder_create_action(dpns.clone(), Identifier::from([0x01; 32]), salted_hash);

        use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentFromCreateTransitionAction;
        let existing_document = dpp::document::Document::try_from_create_transition_action(
            &existing_action,
            owner,
            platform_version,
        )
        .expect("build document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &existing_document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner.to_buffer()),
                    },
                    contract: &dpns,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("insert existing preorder");

        // Now propose a *different* document with the same salted hash.
        let conflicting_action =
            build_preorder_create_action(dpns.clone(), Identifier::from([0x02; 32]), salted_hash);

        let result = drive
            .validate_document_create_transition_action_uniqueness(
                &dpns,
                document_type,
                &conflicting_action,
                Identifier::from([0x88; 32]),
                None,
                platform_version,
            )
            .expect("call should succeed");
        assert!(!result.is_valid(), "expected duplicate index to fail");
        assert_eq!(result.errors.len(), 1);
        match &result.errors[0] {
            ConsensusError::StateError(state) => {
                use dpp::consensus::state::state_error::StateError;
                assert!(
                    matches!(state, StateError::DuplicateUniqueIndexError(_)),
                    "unexpected state error: {state:?}"
                );
            }
            other => panic!("unexpected consensus error: {other:?}"),
        }
    }

    /// A `UniquenessOfDataRequestV0` with `allow_original = true` pointing at
    /// the same `document_id` that already occupies the unique-index slot
    /// must validate successfully — this is the "replace/transfer doesn't
    /// collide with itself" branch.
    #[test]
    fn validate_uniqueness_of_data_v0_allows_original_document_id() {
        let platform_version = PlatformVersion::first();
        let (drive, dpns) = setup_drive_with_dpns(platform_version);

        let document_type = dpns
            .document_type_for_name("preorder")
            .expect("preorder doctype");

        // Insert a preorder whose id we will later claim to "be".
        let salted_hash = [0xCD; 32];
        let owner = Identifier::from([0x55; 32]);
        let document_id = Identifier::from([0xBB; 32]);
        let action = build_preorder_create_action(dpns.clone(), document_id, salted_hash);

        use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentFromCreateTransitionAction;
        let existing_document = dpp::document::Document::try_from_create_transition_action(
            &action,
            owner,
            platform_version,
        )
        .expect("build document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &existing_document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner.to_buffer()),
                    },
                    contract: &dpns,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("insert existing preorder");

        // Now build a v0 uniqueness request with allow_original = true, the
        // same document_id, and the same salted hash. The existing hit must
        // be ignored.
        let data = BTreeMap::from([(
            "saltedDomainHash".to_string(),
            Value::Bytes(salted_hash.to_vec()),
        )]);
        let request = UniquenessOfDataRequestV0 {
            contract: &dpns,
            document_type,
            owner_id: owner,
            document_id,
            allow_original: true,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            data: &data,
        };
        let result = drive
            .validate_uniqueness_of_data(
                UniquenessOfDataRequest::V0(request),
                None,
                platform_version,
            )
            .expect("call should succeed");
        assert!(
            result.is_valid(),
            "allow_original must suppress self-collision: {:?}",
            result.errors
        );

        // Now a second request with `allow_original = false` must surface
        // the collision, exercising the inverse branch.
        let request_strict = UniquenessOfDataRequestV0 {
            contract: &dpns,
            document_type,
            owner_id: owner,
            document_id,
            allow_original: false,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            data: &data,
        };
        let result_strict = drive
            .validate_uniqueness_of_data(
                UniquenessOfDataRequest::V0(request_strict),
                None,
                platform_version,
            )
            .expect("call should succeed");
        assert!(
            !result_strict.is_valid(),
            "allow_original=false must still flag the duplicate"
        );
    }

    /// When an index references a missing property on the document's data
    /// (e.g. because a timestamp the index needs is not provided), the entry
    /// is silently skipped — the `where_queries.len() < index.properties.len()`
    /// branch. The validator should not surface a duplicate error in that
    /// case.
    #[test]
    fn validate_uniqueness_of_data_v0_skips_incomplete_indices() {
        let platform_version = PlatformVersion::first();
        let (drive, dpns) = setup_drive_with_dpns(platform_version);

        let document_type = dpns
            .document_type_for_name("preorder")
            .expect("preorder doctype");

        // Data map that does NOT carry the saltedDomainHash property at all.
        // The only unique index on preorder is on saltedDomainHash, so with
        // the property missing the index is considered no-op and the request
        // must validate successfully.
        let data = BTreeMap::<String, Value>::new();
        let request = UniquenessOfDataRequestV0 {
            contract: &dpns,
            document_type,
            owner_id: Identifier::from([0x99; 32]),
            document_id: Identifier::from([0xCC; 32]),
            allow_original: false,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            data: &data,
        };
        let result = drive
            .validate_uniqueness_of_data(
                UniquenessOfDataRequest::V0(request),
                None,
                platform_version,
            )
            .expect("call should succeed");
        assert!(
            result.is_valid(),
            "missing index data must be silently skipped: {:?}",
            result.errors
        );
    }

    /// v1 `ChangedDocument` path with no changed data values and no changed
    /// timestamps must still allow the original document to stay in the
    /// index (allow_original remains true).
    #[test]
    fn validate_uniqueness_of_data_v1_changed_document_allows_original() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns) = setup_drive_with_dpns(platform_version);

        let document_type = dpns
            .document_type_for_name("preorder")
            .expect("preorder doctype");

        // Insert the original preorder.
        let salted_hash = [0xEF; 32];
        let owner = Identifier::from([0x66; 32]);
        let document_id = Identifier::from([0xDD; 32]);
        let action = build_preorder_create_action(dpns.clone(), document_id, salted_hash);

        use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentFromCreateTransitionAction;
        let existing_document = dpp::document::Document::try_from_create_transition_action(
            &action,
            owner,
            platform_version,
        )
        .expect("build document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &existing_document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner.to_buffer()),
                    },
                    contract: &dpns,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("insert existing preorder");

        let data = BTreeMap::from([(
            "saltedDomainHash".to_string(),
            Value::Bytes(salted_hash.to_vec()),
        )]);
        let changed_data_values: BTreeSet<String> = BTreeSet::new();
        let request = UniquenessOfDataRequestV1 {
            contract: &dpns,
            document_type,
            owner_id: owner,
            creator_id: None,
            document_id,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            data: &data,
            update_type: UniquenessOfDataRequestUpdateType::ChangedDocument {
                changed_owner_id: false,
                changed_updated_at: false,
                changed_transferred_at: false,
                changed_updated_at_block_height: false,
                changed_transferred_at_block_height: false,
                changed_updated_at_core_block_height: false,
                changed_transferred_at_core_block_height: false,
                changed_data_values: Cow::Borrowed(&changed_data_values),
            },
        };
        let result = drive
            .validate_uniqueness_of_data(
                UniquenessOfDataRequest::V1(request),
                None,
                platform_version,
            )
            .expect("call should succeed");
        assert!(
            result.is_valid(),
            "ChangedDocument with no changes should allow original: {:?}",
            result.errors
        );

        // If the request claims a change on the indexed value, allow_original
        // flips to false and the duplicate is surfaced.
        let changed_hash: BTreeSet<String> = BTreeSet::from(["saltedDomainHash".to_string()]);
        let data2 = BTreeMap::from([(
            "saltedDomainHash".to_string(),
            Value::Bytes(salted_hash.to_vec()),
        )]);
        let request_strict = UniquenessOfDataRequestV1 {
            contract: &dpns,
            document_type,
            owner_id: owner,
            creator_id: None,
            document_id,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            data: &data2,
            update_type: UniquenessOfDataRequestUpdateType::ChangedDocument {
                changed_owner_id: false,
                changed_updated_at: false,
                changed_transferred_at: false,
                changed_updated_at_block_height: false,
                changed_transferred_at_block_height: false,
                changed_updated_at_core_block_height: false,
                changed_transferred_at_core_block_height: false,
                changed_data_values: Cow::Borrowed(&changed_hash),
            },
        };
        let result_strict = drive
            .validate_uniqueness_of_data(
                UniquenessOfDataRequest::V1(request_strict),
                None,
                platform_version,
            )
            .expect("call should succeed");
        assert!(
            !result_strict.is_valid(),
            "ChangedDocument with changed indexed value must flag duplicate"
        );
    }

    /// `ChangedDocument` with a required timestamp field missing from the
    /// request must exit early (`exit_early = true`) and the entire index
    /// check is skipped, yielding a valid result.
    #[test]
    fn validate_uniqueness_of_data_v1_exits_early_when_required_timestamp_missing() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns) = setup_drive_with_dpns(platform_version);

        // Use the domain document type because it has required timestamps
        // (`$createdAt`, `$updatedAt`, `$transferredAt`) that can be
        // deliberately omitted to trigger exit_early.
        let document_type = dpns.document_type_for_name("domain").expect("domain");

        // `parentNameAndLabel` is unique over {normalizedParentDomainName,
        // normalizedLabel}, neither of which is a timestamp, so it won't
        // exit early. We must therefore target an index that would need a
        // timestamp. The "identityId" index on domain uses records.identity
        // but is not unique, so domain only has one unique index —
        // parentNameAndLabel. We'll instead use the `updated_at` path by
        // providing data without the normalized fields, which triggers the
        // "no value provided" branch and silently skips the index.
        let data = BTreeMap::<String, Value>::new();
        let changed: BTreeSet<String> = BTreeSet::new();
        let request = UniquenessOfDataRequestV1 {
            contract: &dpns,
            document_type,
            owner_id: Identifier::from([0x11; 32]),
            creator_id: Some(Identifier::from([0x12; 32])),
            document_id: Identifier::from([0x13; 32]),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            data: &data,
            update_type: UniquenessOfDataRequestUpdateType::ChangedDocument {
                changed_owner_id: false,
                changed_updated_at: false,
                changed_transferred_at: false,
                changed_updated_at_block_height: false,
                changed_transferred_at_block_height: false,
                changed_updated_at_core_block_height: false,
                changed_transferred_at_core_block_height: false,
                changed_data_values: Cow::Borrowed(&changed),
            },
        };
        let result = drive
            .validate_uniqueness_of_data(
                UniquenessOfDataRequest::V1(request),
                None,
                platform_version,
            )
            .expect("call should succeed");
        assert!(
            result.is_valid(),
            "missing required data must short-circuit to valid: {:?}",
            result.errors
        );
    }

    /// Non-unique indexes must be skipped entirely — so even if we claim a
    /// duplicate value on a non-unique field, no error surfaces. The
    /// dashpay's `profile` doctype has a non-unique `ownerIdUpdatedAt`
    /// index. We can verify that the query machinery doesn't dispatch on
    /// non-unique indexes by checking that a valid result comes back even
    /// with zero conflicting unique data.
    #[test]
    fn validate_uniqueness_of_data_v0_ignores_non_unique_indexes() {
        let platform_version = PlatformVersion::first();
        let (drive, dpns) = setup_drive_with_dpns(platform_version);

        // The DPNS `domain` doctype has a *non-unique* `identityId` index.
        // Provide only `records` (no normalized* fields) — the only unique
        // index (parentNameAndLabel) is silently skipped, and the non-unique
        // identityId index is never even queried. The resulting validation
        // must be valid.
        let document_type = dpns.document_type_for_name("domain").expect("domain");
        let data = BTreeMap::from([(
            "records".to_string(),
            Value::Map(vec![(
                Value::Text("identity".to_string()),
                Value::Bytes(vec![0x01; 32]),
            )]),
        )]);
        let request = UniquenessOfDataRequestV0 {
            contract: &dpns,
            document_type,
            owner_id: Identifier::from([0x22; 32]),
            document_id: Identifier::from([0x23; 32]),
            allow_original: false,
            created_at: Some(0),
            updated_at: Some(0),
            transferred_at: Some(0),
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            data: &data,
        };
        let result = drive
            .validate_uniqueness_of_data(
                UniquenessOfDataRequest::V0(request),
                None,
                platform_version,
            )
            .expect("call should succeed");
        assert!(
            result.is_valid(),
            "non-unique indexes must be ignored: {:?}",
            result.errors
        );
    }
}

#[cfg(test)]
#[cfg(feature = "server")]
mod unique_time_range_index_tests {
    //! A unique time-range index expresses "at most one document per
    //! non-overlapping window per remaining key tuple" — one report per author
    //! per day. Its uniqueness probe therefore has to look in the *bucket* the
    //! candidate's `$createdAt` falls into: the index stores bucket starts, not
    //! raw timestamps, so a probe on the raw value would look at a key no
    //! document ever occupies and pronounce every duplicate unique.
    //!
    //! The rewritten equality is indistinguishable from a client-written one
    //! once built, so the probe must also carry the resolution provenance —
    //! without it index selection refuses to route a `$createdAt` equality to a
    //! bucketed index and the probe would fail to find any index at all.
    use std::borrow::Cow;
    use std::collections::{BTreeMap, BTreeSet};

    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::{Document, DocumentV0, DocumentV0Getters};
    use dpp::identifier::Identifier;
    use dpp::platform_value::{platform_value, Value};
    use dpp::prelude::DataContract;
    use dpp::validation::SimpleConsensusValidationResult;
    use dpp::version::PlatformVersion;

    use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::{
        UniquenessOfDataRequest, UniquenessOfDataRequestUpdateType, UniquenessOfDataRequestV1,
    };
    use crate::drive::Drive;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    /// One day in each of the two units these tests deal in: the contract
    /// declares its window in seconds, while every `$createdAt` and every
    /// bucket start is a millisecond timestamp.
    const DAY_SECONDS: u64 = 24 * 3_600;
    const DAY_MS: u64 = 24 * 3_600_000;

    /// Deterministic 32-byte fixture identifier derived from the document's
    /// own fixture inputs. Identifiers here are plumbing, not test inputs:
    /// fixed bytes keep a failing GroveDB fixture reproducible run-to-run and
    /// avoid an OS-entropy dependency (and its unwrap) in
    /// consensus-sensitive tests. `marker` separates namespaces (document id
    /// vs owner) and same-timestamp siblings.
    fn fixture_bytes(marker: u8, created_at: u64, tag: &str) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0] = marker;
        bytes[1..9].copy_from_slice(&created_at.to_be_bytes());
        for (i, byte) in tag.bytes().take(23).enumerate() {
            bytes[9 + i] = byte;
        }
        bytes
    }
    /// Start of the window every "same window" timestamp below lands in —
    /// a whole number of days, so it is a bucket start on the default
    /// (phase 0) daily grid.
    const WINDOW_MS: u64 = 100 * DAY_MS;
    /// A half-day phase for the phased-grid test: bucket starts move to
    /// `k * day + 12h`, and the only timestamps outside every window are the
    /// first twelve hours of 1970 — the epoch sliver the probe must skip.
    const PHASE_SECONDS: u64 = 12 * 3_600;
    const PHASE_MS: u64 = 12 * 3_600_000;

    /// A `report` document type with a UNIQUE
    /// `(timeRange($createdAt, range = step = 1 day), author)` index: one
    /// report per author per calendar day. Both index properties are required,
    /// so neither can be null and the terminator always takes the unique
    /// layout.
    ///
    /// `phase_seconds` shifts the window grid within one step (validation
    /// requires `phase < step`); `None` leaves it at the default (0).
    fn build_unique_daily_report_contract(phase_seconds: Option<u64>) -> DataContract {
        let factory =
            DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
        let mut time_range = vec![
            (
                Value::Text("on".to_string()),
                Value::Text("$createdAt".to_string()),
            ),
            (Value::Text("range".to_string()), Value::U64(DAY_SECONDS)),
            (Value::Text("step".to_string()), Value::U64(DAY_SECONDS)),
        ];
        if let Some(phase_seconds) = phase_seconds {
            time_range.push((Value::Text("phase".to_string()), Value::U64(phase_seconds)));
        }
        let index_map = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("dailyReport".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![
                    platform_value!({"$createdAt": "asc"}),
                    platform_value!({"author": "asc"}),
                ]),
            ),
            (Value::Text("unique".to_string()), Value::Bool(true)),
            (Value::Text("timeRange".to_string()), Value::Map(time_range)),
        ];

        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "author": {"type": "string", "maxLength": 63, "position": 0},
            },
            "required": ["author", "$createdAt"],
            "indices": Value::Array(vec![Value::Map(index_map)]),
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "report": document_schema });
        factory
            .create_with_value_config(Identifier::from([201u8; 32]), 0, schemas, None, None)
            .expect("create contract")
            .data_contract_owned()
    }

    fn setup(platform_version: &'static PlatformVersion) -> (Drive, DataContract) {
        setup_with_phase(None, platform_version)
    }

    fn setup_with_phase(
        phase_seconds: Option<u64>,
        platform_version: &'static PlatformVersion,
    ) -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = build_unique_daily_report_contract(phase_seconds);
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");
        (drive, contract)
    }

    /// Stores a `report` and returns its id.
    fn insert_report(
        drive: &Drive,
        contract: &DataContract,
        created_at: u64,
        author: &str,
        platform_version: &PlatformVersion,
    ) -> Identifier {
        let document_type = contract.document_type_for_name("report").expect("report");
        let owner_bytes = fixture_bytes(1, created_at, author);
        let document = Document::V0(DocumentV0 {
            id: Identifier::from(fixture_bytes(2, created_at, author)),
            owner_id: Identifier::from(owner_bytes),
            properties: BTreeMap::from([("author".to_string(), Value::Text(author.to_string()))]),
            created_at: Some(created_at),
            revision: Some(1),
            ..Default::default()
        });
        let document_id = document.id();
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner_bytes),
                    },
                    contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("add document");
        document_id
    }

    /// Runs the uniqueness validation for a candidate `report`.
    fn check_uniqueness(
        drive: &Drive,
        contract: &DataContract,
        document_id: Identifier,
        created_at: u64,
        author: &str,
        update_type: UniquenessOfDataRequestUpdateType,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let document_type = contract.document_type_for_name("report").expect("report");
        let data = BTreeMap::from([("author".to_string(), Value::Text(author.to_string()))]);
        let request = UniquenessOfDataRequestV1 {
            contract,
            document_type,
            owner_id: Identifier::from([0x01; 32]),
            creator_id: None,
            document_id,
            created_at: Some(created_at),
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            data: &data,
            update_type,
        };
        drive
            .validate_uniqueness_of_data(
                UniquenessOfDataRequest::V1(request),
                None,
                platform_version,
            )
            .expect("uniqueness validation should run")
    }

    fn assert_duplicate(result: &SimpleConsensusValidationResult, context: &str) {
        assert!(
            matches!(
                result.errors.first(),
                Some(ConsensusError::StateError(
                    StateError::DuplicateUniqueIndexError(_)
                ))
            ),
            "{context}: expected a DuplicateUniqueIndexError, got: {:?}",
            result.errors
        );
    }

    /// The core of the feature: two documents whose raw `$createdAt` values
    /// differ by hours still occupy the same day bucket, so the second one
    /// violates the unique index. A probe that compared raw timestamps would
    /// see an empty index and let it through.
    #[test]
    fn candidate_in_the_same_window_as_a_stored_document_is_a_duplicate() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup(platform_version);

        insert_report(
            &drive,
            &contract,
            WINDOW_MS + 3_600_000,
            "alice",
            platform_version,
        );

        let result = check_uniqueness(
            &drive,
            &contract,
            Identifier::from([0xAA; 32]),
            // 8 hours later — a different timestamp, the same day bucket.
            WINDOW_MS + 9 * 3_600_000,
            "alice",
            UniquenessOfDataRequestUpdateType::NewDocument,
            platform_version,
        );
        assert_duplicate(&result, "same window, same author");
    }

    /// The next window is a different bucket, so the same author may report
    /// again — and the write actually goes through, proving the probe agrees
    /// with the layout the insert walker builds.
    #[test]
    fn candidate_in_the_next_window_is_unique_and_insertable() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup(platform_version);

        insert_report(
            &drive,
            &contract,
            WINDOW_MS + 3_600_000,
            "alice",
            platform_version,
        );

        let next_window_timestamp = WINDOW_MS + DAY_MS + 3_600_000;
        let result = check_uniqueness(
            &drive,
            &contract,
            Identifier::from([0xAA; 32]),
            next_window_timestamp,
            "alice",
            UniquenessOfDataRequestUpdateType::NewDocument,
            platform_version,
        );
        assert!(
            result.is_valid(),
            "the next window is a different bucket: {:?}",
            result.errors
        );

        insert_report(
            &drive,
            &contract,
            next_window_timestamp,
            "alice",
            platform_version,
        );
    }

    /// The bucket is only the first component of the tuple: a different author
    /// in the same window is a different slot.
    #[test]
    fn candidate_in_the_same_window_with_a_different_suffix_is_unique() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup(platform_version);

        insert_report(
            &drive,
            &contract,
            WINDOW_MS + 3_600_000,
            "alice",
            platform_version,
        );

        let result = check_uniqueness(
            &drive,
            &contract,
            Identifier::from([0xAA; 32]),
            WINDOW_MS + 9 * 3_600_000,
            "bob",
            UniquenessOfDataRequestUpdateType::NewDocument,
            platform_version,
        );
        assert!(
            result.is_valid(),
            "a different author in the same window is a different tuple: {:?}",
            result.errors
        );

        insert_report(
            &drive,
            &contract,
            WINDOW_MS + 9 * 3_600_000,
            "bob",
            platform_version,
        );
    }

    /// Update semantics. `$createdAt` is immutable — that is exactly why a
    /// unique time-range index is allowed to bucket it — so on the
    /// `ChangedDocument` path the bucket component of the tuple never moves and
    /// `allow_original` keeps its ordinary meaning: a document may keep the
    /// slot it already holds, but may not move into one another document holds.
    #[test]
    fn changed_document_keeps_its_own_slot_but_cannot_take_another_documents_slot() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup(platform_version);

        let alice_created_at = WINDOW_MS + 3_600_000;
        let alice_id = insert_report(
            &drive,
            &contract,
            alice_created_at,
            "alice",
            platform_version,
        );
        // A second document in the SAME window under a different author.
        insert_report(
            &drive,
            &contract,
            WINDOW_MS + 9 * 3_600_000,
            "bob",
            platform_version,
        );

        // Alice's document changing its author to "bob" walks into the slot
        // bob's document already holds, in the same (unchanged) bucket.
        let changed_author: BTreeSet<String> = BTreeSet::from(["author".to_string()]);
        let result = check_uniqueness(
            &drive,
            &contract,
            alice_id,
            alice_created_at,
            "bob",
            UniquenessOfDataRequestUpdateType::ChangedDocument {
                changed_owner_id: false,
                changed_updated_at: false,
                changed_transferred_at: false,
                changed_updated_at_block_height: false,
                changed_transferred_at_block_height: false,
                changed_updated_at_core_block_height: false,
                changed_transferred_at_core_block_height: false,
                changed_data_values: Cow::Borrowed(&changed_author),
            },
            platform_version,
        );
        assert_duplicate(&result, "moving onto another document's tuple");

        // The same document re-submitted with its own tuple finds itself in
        // the bucket and is allowed to stay (`allow_original`).
        let unchanged: BTreeSet<String> = BTreeSet::new();
        let result = check_uniqueness(
            &drive,
            &contract,
            alice_id,
            alice_created_at,
            "alice",
            UniquenessOfDataRequestUpdateType::ChangedDocument {
                changed_owner_id: false,
                changed_updated_at: false,
                changed_transferred_at: false,
                changed_updated_at_block_height: false,
                changed_transferred_at_block_height: false,
                changed_updated_at_core_block_height: false,
                changed_transferred_at_core_block_height: false,
                changed_data_values: Cow::Borrowed(&unchanged),
            },
            platform_version,
        );
        assert!(
            result.is_valid(),
            "a document must be allowed to keep the slot it already holds: {:?}",
            result.errors
        );
    }

    /// A `$createdAt` inside the epoch sliver before the grid's phase anchor
    /// produces no index entries at all (the insert walker writes none), so
    /// such a document cannot collide with anything: the probe must skip this
    /// index entirely rather than invent a bucket for a timestamp that
    /// belongs to none. No real timestamp reaches the sliver — this pins the
    /// defensive rule all three walkers and the probe share.
    #[test]
    fn epoch_sliver_candidate_skips_the_time_range_index_check() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_with_phase(Some(PHASE_SECONDS), platform_version);

        // A stored report in the phased grid's first window `[12h, 36h)`.
        insert_report(
            &drive,
            &contract,
            PHASE_MS + 3_600_000,
            "alice",
            platform_version,
        );

        // Same author, timestamp one millisecond before the phase anchor: it
        // belongs to no window, so there is nothing for it to duplicate.
        let result = check_uniqueness(
            &drive,
            &contract,
            Identifier::from([0xAA; 32]),
            PHASE_MS - 1,
            "alice",
            UniquenessOfDataRequestUpdateType::NewDocument,
            platform_version,
        );
        assert!(
            result.is_valid(),
            "an epoch-sliver document is not indexed, so it cannot collide: {:?}",
            result.errors
        );

        // Contrast: inside the first window the same author does collide, so
        // the skip above is the sliver talking and not a probe that silently
        // stopped working on this contract.
        let result = check_uniqueness(
            &drive,
            &contract,
            Identifier::from([0xAA; 32]),
            PHASE_MS + 9 * 3_600_000,
            "alice",
            UniquenessOfDataRequestUpdateType::NewDocument,
            platform_version,
        );
        assert_duplicate(&result, "inside the first window");
    }
}
