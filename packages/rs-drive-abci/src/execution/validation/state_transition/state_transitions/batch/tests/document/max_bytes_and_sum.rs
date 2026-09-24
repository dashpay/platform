//! End-to-end coverage for the `maxBytes` and `sumOfProperties` property
//! keywords (protocol version 14): a string bounded by its UTF-8 length, and
//! an object whose integer members must add up to a declared total. Both are
//! checked on every create and replace, after the JSON schema validation; a
//! value that breaks either is consensus-rejected and leaves the stored
//! document untouched.

use super::*;

mod max_bytes_and_sum_tests {
    use super::*;
    use crate::execution::validation::state_transition::batch::action_validation::document::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::BasicError;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionV0};
    use std::collections::{BTreeMap, BTreeSet};
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::prelude::Identifier;
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;

    /// A mutable `profile` type: a `bio` of at most 64 characters and 16 bytes,
    /// and a `split` of two percentages that must add up to 100.
    fn profile_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "bio": {
                    "type": "string",
                    "maxLength": 64,
                    "maxBytes": 16,
                    "position": 0
                },
                "split": {
                    "type": "object",
                    "position": 1,
                    "properties": {
                        "leader": { "type": "integer", "minimum": 0, "maximum": 100, "position": 0 },
                        "rest": { "type": "integer", "minimum": 0, "maximum": 100, "position": 1 }
                    },
                    "required": ["leader", "rest"],
                    "additionalProperties": false,
                    "sumOfProperties": 100
                }
            },
            "required": ["bio", "split"],
            "additionalProperties": false
        })
    }

    fn split(leader: u8, rest: u8) -> Value {
        platform_value!({ "leader": leader, "rest": rest })
    }

    /// One identity and one contract whose `profile` type is the one above.
    struct ProfileFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        identity: Identity,
        contract: DataContract,
        /// The identity contract nonce the next transition uses. Every
        /// processed transition consumes one, including the ones that fail
        /// with a paid consensus error.
        next_nonce: IdentityNonce,
    }

    impl ProfileFixture {
        fn new() -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 959, dash_to_credits!(0.5));

            let mut contract = get_data_contract_fixture(
                Some(identity.id()),
                0,
                platform_version.protocol_version,
            )
            .data_contract_owned();
            contract
                .set_document_schema(
                    "profile",
                    profile_schema(),
                    true,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("expected to add the profile document type");
            platform
                .drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply the contract");

            Self {
                platform,
                signer,
                key,
                identity,
                contract,
                next_nonce: 1,
            }
        }

        async fn create(&mut self, bio: &str, split: Value) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let profile_type = self
                .contract
                .document_type_for_name("profile")
                .expect("expected the profile document type");
            let mut rng = StdRng::seed_from_u64(434);
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut profile = profile_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    self.identity.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random profile");
            profile
                .set_id_for_creation(profile_type, &entropy.0, self.next_nonce, platform_version)
                .expect("expected to set the document id");
            profile.set("bio", Value::Text(bio.to_string()));
            profile.set("split", split);

            let transition = BatchTransition::new_document_creation_transition_from_document(
                profile,
                profile_type,
                entropy.0,
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the create transition");
            self.next_nonce += 1;
            self.process(&transition)
        }

        /// Replaces `stored` with `mutate` applied and the revision bumped.
        async fn replace(
            &mut self,
            stored: &Document,
            mutate: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut replacement = stored.clone();
            mutate(&mut replacement);
            replacement
                .increment_revision()
                .expect("expected the revision to increment");
            let profile_type = self
                .contract
                .document_type_for_name("profile")
                .expect("expected the profile document type");
            let transition = BatchTransition::new_document_replacement_transition_from_document(
                replacement,
                profile_type,
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the replace transition");
            self.next_nonce += 1;
            self.process(&transition)
        }

        fn process(&self, transition: &StateTransition) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let platform_state = self.platform.state.load();
            let serialized = transition
                .serialize_to_bytes()
                .expect("expected the transition to serialize");
            let transaction = self.platform.drive.grove.start_transaction();
            let processing_result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process the state transition");
            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit the transaction");
            processing_result.into_execution_results().remove(0)
        }

        /// The stored profiles, read back from Drive.
        fn stored_profiles(&self) -> Vec<Document> {
            let platform_version = PlatformVersion::latest();
            let query = DriveDocumentQuery::from_sql_expr(
                "select * from profile",
                &self.contract,
                Some(&self.platform.config.drive),
                platform_version,
            )
            .expect("expected a document query");
            self.platform
                .drive
                .query_documents(query, None, false, None, None)
                .expect("expected a query result")
                .documents()
                .to_vec()
        }
    }

    fn expect_max_bytes_error(result: StateTransitionExecutionResult, byte_length: u32) {
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        assert_matches!(
            error,
            ConsensusError::BasicError(BasicError::DocumentPropertyMaxBytesExceededError(e))
                if e.property() == "bio" && e.byte_length() == byte_length && e.max_bytes() == 16
        );
    }

    fn expect_sum_error(result: StateTransitionExecutionResult, actual_sum: i64) {
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        assert_matches!(
            error,
            ConsensusError::BasicError(BasicError::DocumentPropertySumMismatchError(e))
                if e.property() == "split"
                    && e.expected_sum() == 100
                    && e.actual_sum() == actual_sum
        );
    }

    #[tokio::test]
    async fn should_create_a_document_within_both_bounds() {
        let mut fixture = ProfileFixture::new();

        // Sixteen bytes in eight two-byte characters, and a split of exactly 100
        let result = fixture.create("éééééééé", split(40, 60)).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_profiles().len(), 1);
    }

    /// Nine two-byte characters are far inside `maxLength`, so the JSON schema
    /// accepts them; their 18 bytes are over `maxBytes`.
    #[tokio::test]
    async fn should_refuse_a_string_over_its_max_bytes_within_its_max_length() {
        let mut fixture = ProfileFixture::new();

        let result = fixture.create("ééééééééé", split(40, 60)).await;

        expect_max_bytes_error(result, 18);
        assert!(fixture.stored_profiles().is_empty());
    }

    #[tokio::test]
    async fn should_refuse_an_object_whose_members_miss_their_sum() {
        let mut fixture = ProfileFixture::new();

        for (leader, rest) in [(40, 50), (0, 0), (100, 100)] {
            let result = fixture.create("hello", split(leader, rest)).await;
            expect_sum_error(result, i64::from(leader) + i64::from(rest));
        }
        assert!(fixture.stored_profiles().is_empty());
    }

    #[tokio::test]
    async fn should_refuse_a_replace_that_breaks_either_bound() {
        let mut fixture = ProfileFixture::new();
        let result = fixture.create("hello", split(40, 60)).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let stored = fixture.stored_profiles().remove(0);

        let result = fixture
            .replace(&stored, |profile| {
                profile.set("bio", Value::Text("ééééééééé".to_string()))
            })
            .await;
        expect_max_bytes_error(result, 18);

        let result = fixture
            .replace(&stored, |profile| profile.set("split", split(10, 10)))
            .await;
        expect_sum_error(result, 20);

        let after = fixture.stored_profiles();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].get("bio"), stored.get("bio"));
        assert_eq!(after[0].get("split"), stored.get("split"));

        // A replace within both bounds still goes through
        let result = fixture
            .replace(&stored, |profile| {
                profile.set("bio", Value::Text("hé".to_string()));
                profile.set("split", split(0, 100));
            })
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// The replace structure dispatcher on both sides of the gate: structure
    /// generation 0 gained both checks in place, so at protocol version 13 it
    /// must still accept the action (no property parsed there carries either
    /// keyword and both dpp gates are `None`), and at 14 refuse it. The action
    /// is built by hand the way the transformer would build it, against the
    /// contract as Drive hands it back.
    #[test]
    fn should_not_check_either_bound_on_replace_before_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        let fixture = ProfileFixture::new();
        let owner_id = fixture.identity.id();

        let (_, contract_fetch_info) = fixture
            .platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                fixture.contract.id().to_buffer(),
                None,
                false,
                None,
                platform_version,
            )
            .expect("expected to fetch the contract");
        let contract_fetch_info = contract_fetch_info.expect("the contract is in state");

        let action = |bio: &str, leader: u8, rest: u8| {
            DocumentReplaceTransitionAction::V0(DocumentReplaceTransitionActionV0 {
                base: DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
                    id: Identifier::from([0xAA; 32]),
                    identity_contract_nonce: 1,
                    document_type_name: "profile".to_string(),
                    data_contract: contract_fetch_info.clone(),
                    token_cost: None,
                    gas_fees_paid_by: GasFeesPaidBy::default(),
                    contract_gas_fees_paid_by: GasFeesPaidBy::default(),
                    declared_action_fee: None,
                }),
                revision: 2,
                created_at: None,
                updated_at: None,
                transferred_at: None,
                created_at_block_height: None,
                updated_at_block_height: None,
                transferred_at_block_height: None,
                created_at_core_block_height: None,
                updated_at_core_block_height: None,
                transferred_at_core_block_height: None,
                data: BTreeMap::from([
                    ("bio".to_string(), Value::Text(bio.to_string())),
                    ("split".to_string(), split(leader, rest)),
                ]),
                changed_data_fields: BTreeSet::new(),
                added_data_fields: BTreeSet::new(),
                removed_identifier_fields: BTreeMap::new(),
                stored_changed_values: BTreeMap::new(),
                creator_id: None,
            })
        };
        let platform_version_13 =
            PlatformVersion::get(13).expect("platform version 13 should exist");

        for (bio, leader, rest) in [("ééééééééé", 40, 60), ("hello", 10, 10)] {
            let before = action(bio, leader, rest)
                .validate_structure(owner_id, platform_version_13)
                .expect("structure validation should run");
            assert!(
                before.is_valid(),
                "structure generation 0 must check neither bound before 14: {:?}",
                before.errors
            );
        }

        let at = action("ééééééééé", 40, 60)
            .validate_structure(owner_id, platform_version)
            .expect("structure validation should run");
        assert_matches!(
            at.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DocumentPropertyMaxBytesExceededError(e))]
                if e.property() == "bio" && e.byte_length() == 18
        );
        let at = action("hello", 10, 10)
            .validate_structure(owner_id, platform_version)
            .expect("structure validation should run");
        assert_matches!(
            at.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DocumentPropertySumMismatchError(e))]
                if e.property() == "split" && e.actual_sum() == 20
        );
    }
}
