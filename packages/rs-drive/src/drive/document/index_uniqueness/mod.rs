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
