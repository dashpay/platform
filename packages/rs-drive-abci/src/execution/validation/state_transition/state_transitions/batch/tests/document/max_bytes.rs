//! End-to-end coverage for the `maxBytes` property keyword (protocol version
//! 14): a string, or each string element of a typed array, bounded by its
//! UTF-8 length. The document validation checks it after the JSON schema on
//! every create and replace; a longer value is consensus-rejected and leaves
//! the stored document untouched.

use super::*;

mod max_bytes_tests {
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
    /// and `tags`, up to four strings of at most 8 bytes each.
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
                "tags": {
                    "type": "array",
                    "maxItems": 4,
                    "items": { "type": "string", "maxLength": 16, "maxBytes": 8 },
                    "position": 1
                }
            },
            "required": ["bio", "tags"],
            "additionalProperties": false
        })
    }

    fn tags(tags: &[&str]) -> Value {
        Value::Array(
            tags.iter()
                .map(|tag| Value::Text(tag.to_string()))
                .collect(),
        )
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

        async fn create(&mut self, bio: &str, tags: Value) -> StateTransitionExecutionResult {
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
            profile.set("tags", tags);

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

    fn expect_max_bytes_error(
        result: StateTransitionExecutionResult,
        property: &str,
        byte_length: u32,
        max_bytes: u16,
    ) {
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        assert_matches!(
            error,
            ConsensusError::BasicError(BasicError::DocumentPropertyMaxBytesExceededError(e))
                if e.property() == property
                    && e.byte_length() == byte_length
                    && e.max_bytes() == max_bytes
        );
    }

    #[tokio::test]
    async fn should_create_a_document_within_its_byte_caps() {
        let mut fixture = ProfileFixture::new();

        // Sixteen bytes in eight two-byte characters, and tags of eight bytes at most
        let result = fixture.create("éééééééé", tags(&["éééé", "short"])).await;

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

        let result = fixture.create("ééééééééé", tags(&[])).await;

        expect_max_bytes_error(result, "bio", 18, 16);
        assert!(fixture.stored_profiles().is_empty());
    }

    /// Each element of a typed array of strings is bounded on its own, and the
    /// refusal names the element.
    #[tokio::test]
    async fn should_refuse_a_typed_array_element_over_its_max_bytes() {
        let mut fixture = ProfileFixture::new();

        let result = fixture
            .create("hello", tags(&["ok", "fine", "ééééé"]))
            .await;

        expect_max_bytes_error(result, "tags[2]", 10, 8);
        assert!(fixture.stored_profiles().is_empty());
    }

    #[tokio::test]
    async fn should_refuse_a_replace_over_a_byte_cap() {
        let mut fixture = ProfileFixture::new();
        let result = fixture.create("hello", tags(&["a"])).await;
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
        expect_max_bytes_error(result, "bio", 18, 16);

        let result = fixture
            .replace(&stored, |profile| profile.set("tags", tags(&["ééééé"])))
            .await;
        expect_max_bytes_error(result, "tags[0]", 10, 8);

        let after = fixture.stored_profiles();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].get("bio"), stored.get("bio"));
        assert_eq!(after[0].get("tags"), stored.get("tags"));

        // A replace within both caps still goes through
        let result = fixture
            .replace(&stored, |profile| {
                profile.set("bio", Value::Text("hé".to_string()));
                profile.set("tags", tags(&["éééé"]));
            })
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// The replace structure dispatcher on both sides of the gate: the document
    /// validation it runs gained the check in place, so at protocol version 13
    /// it must still accept the action (the dpp gate is `None` there), and at
    /// 14 refuse it. The action is built by hand the way the transformer would
    /// build it, against the contract as Drive hands it back.
    #[test]
    fn should_not_check_the_byte_cap_on_replace_before_protocol_version_14() {
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

        let action = |bio: &str| {
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
                    ("tags".to_string(), tags(&[])),
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

        let before = action("ééééééééé")
            .validate_structure(owner_id, platform_version_13)
            .expect("structure validation should run");
        assert!(
            before.is_valid(),
            "the document validation must not check the byte cap before 14: {:?}",
            before.errors
        );

        let at = action("ééééééééé")
            .validate_structure(owner_id, platform_version)
            .expect("structure validation should run");
        assert_matches!(
            at.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DocumentPropertyMaxBytesExceededError(e))]
                if e.property() == "bio" && e.byte_length() == 18
        );
    }
}
