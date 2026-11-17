//! A document create or replace on a type that keeps the total of its summed property
//! (`documentsSummable`): from protocol version 15 state validation holds the total to what
//! the type's sums can hold, so a write GroveDB would refuse to apply is refused before it,
//! as a paid consensus error.

use super::*;

mod sum_overflow_tests {
    use super::*;
    use crate::execution::check_tx::CheckTxLevel;
    use crate::platform_types::platform::PlatformRef;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::data_contract::config::v1::DataContractConfigSettersV1;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::query::DriveDocumentQuery;
    use drive::util::storage_flags::StorageFlags;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    /// The document type keeping the total of `amount`, with a per-owner sum and its range
    /// sums.
    const TIP: &str = "tip";
    /// The document type keeping the total of `amount`, with an index keying a document by
    /// each of its `tags`.
    const TAGGED: &str = "tagged";
    /// The most tags a `tagged` document holds.
    const MAX_TAGS: i64 = 3;

    /// A mutable, transferable type keeping the total of an `amount` that is never negative.
    /// The contract does not size its integers, so `amount` reaches `i64::MAX`.
    fn tip_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "canBeDeleted": true,
            "transferable": 1,
            "documentsSummable": "amount",
            "properties": {
                "amount": { "type": "integer", "minimum": 0, "position": 0 }
            },
            "required": ["amount"],
            "indices": [{
                "name": "byOwner",
                "properties": [{ "$ownerId": "asc" }],
                "summable": "amount",
                "rangeSummable": true
            }],
            "additionalProperties": false
        })
    }

    /// A type keeping the total of `amount`, whose index adds one entry for each tag of a
    /// document.
    fn tagged_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsSummable": "amount",
            "properties": {
                "amount": { "type": "integer", "minimum": 0, "position": 0 },
                "tags": {
                    "type": "array",
                    "items": { "type": "string", "maxLength": 20 },
                    "minItems": 1,
                    "maxItems": MAX_TAGS,
                    "uniqueItems": true,
                    "position": 1
                }
            },
            "required": ["amount", "tags"],
            "indices": [{
                "name": "byTag",
                "properties": [{ "tags": "asc" }],
                "summable": "amount",
                "rangeSummable": true
            }],
            "additionalProperties": false
        })
    }

    type Writer = (Identity, SimpleSigner, IdentityPublicKey);

    /// Two identities and one contract with the summed types.
    struct SumFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        platform_version: &'static PlatformVersion,
        writers: Vec<Writer>,
        contract: DataContract,
        next_nonces: BTreeMap<Identifier, IdentityNonce>,
        block_info: BlockInfo,
    }

    /// What a transition did: what `check_tx` said, the block's result, and the writer's
    /// identity contract nonce and balance before and after.
    struct Outcome {
        /// The identity contract nonce the transition carries.
        transition_nonce: IdentityNonce,
        check_tx_valid: bool,
        result: StateTransitionExecutionResult,
        nonce_before: Option<IdentityNonce>,
        nonce_after: Option<IdentityNonce>,
        balance_before: Credits,
        balance_after: Credits,
    }

    impl Outcome {
        fn charged(&self) -> Credits {
            self.balance_before.saturating_sub(self.balance_after)
        }
    }

    impl SumFixture {
        fn new(protocol_version: u32) -> Self {
            let platform_version =
                PlatformVersion::get(protocol_version).expect("expected a known protocol version");
            let mut platform = TestPlatformBuilder::new()
                .with_initial_protocol_version(protocol_version)
                .build_with_mock_rpc()
                .set_genesis_state();

            let writers = vec![
                setup_identity(&mut platform, 6101, dash_to_credits!(1.0)),
                setup_identity(&mut platform, 6102, dash_to_credits!(1.0)),
            ];

            let mut contract =
                get_data_contract_fixture(Some(writers[0].0.id()), 0, protocol_version)
                    .data_contract_owned();
            contract.config_mut().set_sized_integer_types_enabled(false);
            for (name, schema) in [(TIP, tip_schema()), (TAGGED, tagged_schema())] {
                contract
                    .set_document_schema(name, schema, true, &mut Vec::new(), platform_version)
                    .expect("expected to add the document type");
            }
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
                platform_version,
                writers,
                contract,
                next_nonces: BTreeMap::new(),
                block_info: BlockInfo::default(),
            }
        }

        fn latest() -> Self {
            Self::new(PlatformVersion::latest().protocol_version)
        }

        /// The total `document_type` keeps of `amount`.
        fn total(&self, document_type: &str) -> i64 {
            self.platform
                .drive
                .fetch_document_type_sum(
                    &self.contract,
                    self.contract
                        .document_type_for_name(document_type)
                        .expect("expected the document type"),
                    None,
                    None,
                    self.platform_version,
                )
                .expect("expected to fetch the total")
                .0
        }

        fn take_nonce(&mut self, writer: usize) -> IdentityNonce {
            let next = self
                .next_nonces
                .entry(self.writers[writer].0.id())
                .or_insert(1);
            let nonce = *next;
            *next += 1;
            nonce
        }

        fn nonce(&self, writer: usize) -> Option<IdentityNonce> {
            self.platform
                .drive
                .fetch_identity_contract_nonce(
                    self.writers[writer].0.id().to_buffer(),
                    self.contract.id().to_buffer(),
                    true,
                    None,
                    self.platform_version,
                )
                .expect("expected to fetch the nonce")
        }

        fn balance(&self, writer: usize) -> Credits {
            self.platform
                .drive
                .fetch_identity_balance(
                    self.writers[writer].0.id().to_buffer(),
                    None,
                    self.platform_version,
                )
                .expect("expected to fetch the balance")
                .expect("expected a balance")
        }

        fn check_tx(&self, transition: &StateTransition) -> bool {
            let state = self.platform.state.load();
            let platform_ref = PlatformRef {
                drive: &self.platform.drive,
                state: &state,
                config: &self.platform.config,
                core_rpc: &self.platform.core_rpc,
            };
            let raw = transition
                .serialize_to_bytes()
                .expect("expected the transition to serialize");
            self.platform
                .check_tx(
                    &raw,
                    CheckTxLevel::FirstTimeCheck,
                    &platform_ref,
                    self.platform_version,
                )
                .expect("expected to check the transition")
                .is_valid()
        }

        fn run(
            &self,
            writer: usize,
            transition_nonce: IdentityNonce,
            transition: &StateTransition,
        ) -> Outcome {
            let check_tx_valid = self.check_tx(transition);
            let nonce_before = self.nonce(writer);
            let balance_before = self.balance(writer);
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
                    &self.block_info,
                    &transaction,
                    self.platform_version,
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
            let result = processing_result.into_execution_results().remove(0);
            Outcome {
                transition_nonce,
                check_tx_valid,
                result,
                nonce_before,
                nonce_after: self.nonce(writer),
                balance_before,
                balance_after: self.balance(writer),
            }
        }

        /// A create by `writer` of a `document_type` document with `amount`, and the document.
        async fn create(
            &mut self,
            writer: usize,
            document_type: &str,
            amount: i64,
        ) -> (Outcome, Document) {
            let platform_version = self.platform_version;
            let nonce = self.take_nonce(writer);
            let (identity, signer, key) = &self.writers[writer];
            let document_type_ref = self
                .contract
                .document_type_for_name(document_type)
                .expect("expected the document type");
            let mut rng = StdRng::seed_from_u64(nonce * 31 + writer as u64);
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = document_type_ref
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");
            document
                .set_id_for_creation(document_type_ref, &entropy.0, nonce, platform_version)
                .expect("expected to set the document id");
            document.set("amount", Value::I64(amount));
            if document_type == TAGGED {
                document.set(
                    "tags",
                    Value::Array(
                        (0..MAX_TAGS)
                            .map(|tag| Value::Text(format!("tag{tag}")))
                            .collect(),
                    ),
                );
            }
            let transition = BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                document_type_ref,
                entropy.0,
                key,
                nonce,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the create transition");
            (self.run(writer, nonce, &transition), document)
        }

        /// A replace by the owner `writer` of `document` with `amount`, and the replacement.
        async fn replace(
            &mut self,
            writer: usize,
            document_type: &str,
            document: &Document,
            amount: i64,
        ) -> (Outcome, Document) {
            let platform_version = self.platform_version;
            let nonce = self.take_nonce(writer);
            let (_, signer, key) = &self.writers[writer];
            let document_type_ref = self
                .contract
                .document_type_for_name(document_type)
                .expect("expected the document type");
            let mut replacement = document.clone();
            replacement.set("amount", Value::I64(amount));
            replacement
                .increment_revision()
                .expect("expected the revision to increment");
            let transition = BatchTransition::new_document_replacement_transition_from_document(
                replacement.clone(),
                document_type_ref,
                key,
                nonce,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the replace transition");
            (self.run(writer, nonce, &transition), replacement)
        }

        /// A transfer by the owner `writer` of `document` to `recipient`.
        async fn transfer(
            &mut self,
            writer: usize,
            document_type: &str,
            document: &Document,
            recipient: usize,
        ) -> Outcome {
            let platform_version = self.platform_version;
            let nonce = self.take_nonce(writer);
            let recipient_id = self.writers[recipient].0.id();
            let (_, signer, key) = &self.writers[writer];
            let document_type_ref = self
                .contract
                .document_type_for_name(document_type)
                .expect("expected the document type");
            let mut transferred = document.clone();
            transferred
                .increment_revision()
                .expect("expected the revision to increment");
            let transition = BatchTransition::new_document_transfer_transition_from_document(
                transferred,
                document_type_ref,
                recipient_id,
                key,
                nonce,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the transfer transition");
            self.run(writer, nonce, &transition)
        }

        /// A delete by the owner `writer` of `document`.
        async fn delete(
            &mut self,
            writer: usize,
            document_type: &str,
            document: &Document,
        ) -> Outcome {
            let platform_version = self.platform_version;
            let nonce = self.take_nonce(writer);
            let (_, signer, key) = &self.writers[writer];
            let document_type_ref = self
                .contract
                .document_type_for_name(document_type)
                .expect("expected the document type");
            let transition = BatchTransition::new_document_deletion_transition_from_document(
                document.clone(),
                document_type_ref,
                key,
                nonce,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the delete transition");
            self.run(writer, nonce, &transition)
        }

        /// The stored documents of `document_type`, by id, with their owners and amounts.
        fn stored(&self, document_type: &str) -> BTreeMap<Identifier, (Identifier, i64)> {
            let query = DriveDocumentQuery::from_sql_expr(
                &format!("select * from {document_type}"),
                &self.contract,
                Some(&self.platform.config.drive),
                self.platform_version,
            )
            .expect("expected a document query");
            self.platform
                .drive
                .query_documents(query, None, false, None, None)
                .expect("expected a query result")
                .documents()
                .iter()
                .map(|document| {
                    let amount = document
                        .get("amount")
                        .and_then(|value| value.to_integer::<i64>().ok())
                        .expect("expected an amount");
                    (document.id(), (document.owner_id(), amount))
                })
                .collect()
        }

        /// Asserts `outcome` succeeded.
        fn expect_success(outcome: &Outcome, what: &str) {
            assert_matches!(
                outcome.result,
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                "{what} must succeed"
            );
        }
    }

    /// Asserts a write that would take its type's total past `limit` was refused as a paid
    /// consensus error that bumped the writer's nonce and charged `expected_charge`, the fee
    /// the result reports, leaving the documents as they were. The charge is the nonce bump
    /// and the validation, with the read of the total: a writer's first nonce on the contract
    /// stores bytes, a later one rewrites them.
    fn expect_refused(
        fixture: &SumFixture,
        what: &str,
        outcome: &Outcome,
        document_type: &str,
        limit: i64,
        stored_before: &BTreeMap<Identifier, (Identifier, i64)>,
        expected_charge: Credits,
    ) {
        assert!(outcome.check_tx_valid, "{what}: check_tx admits the write");
        let StateTransitionExecutionResult::PaidConsensusError {
            error, actual_fees, ..
        } = &outcome.result
        else {
            panic!(
                "{what}: expected a paid consensus error, got {:?}",
                outcome.result
            );
        };
        let ConsensusError::StateError(StateError::DocumentSumOverflowError(overflow)) = error
        else {
            panic!("{what}: expected a document sum overflow error, got {error:?}");
        };
        assert_eq!(error.code(), 40154, "{what}");
        assert_eq!(overflow.contract_id(), &fixture.contract.id(), "{what}");
        assert_eq!(overflow.document_type_name(), document_type, "{what}");
        assert_eq!(overflow.summed_property(), "amount", "{what}");
        assert_eq!(overflow.limit(), limit, "{what}");
        assert_eq!(
            outcome
                .nonce_after
                .map(|nonce| nonce & IDENTITY_NONCE_VALUE_FILTER),
            Some(outcome.transition_nonce),
            "{what}: the nonce is bumped to the transition's"
        );
        assert_eq!(
            outcome.charged(),
            actual_fees.total_base_fee(),
            "{what}: the writer pays the fee the result reports"
        );
        assert_eq!(
            outcome.charged(),
            expected_charge,
            "{what}: the nonce bump and the validation, with the read of the total"
        );
        assert_eq!(
            &fixture.stored(document_type),
            stored_before,
            "{what}: the documents are as they were"
        );
    }

    #[tokio::test]
    async fn should_refuse_a_create_past_the_types_total_as_a_paid_error() {
        let mut fixture = SumFixture::latest();
        let (outcome, _) = fixture.create(0, TIP, i64::MAX).await;
        SumFixture::expect_success(&outcome, "the create of i64::MAX");
        assert_eq!(fixture.total(TIP), i64::MAX);

        // Whoever writes it: the other writer's own sum is empty, the type's total is not.
        for (writer, charge) in [(0, 490_540), (1, 12_172_080)] {
            let stored_before = fixture.stored(TIP);
            let (outcome, _) = fixture.create(writer, TIP, 1).await;
            expect_refused(
                &fixture,
                &format!("create of 1 by writer {writer}"),
                &outcome,
                TIP,
                i64::MAX,
                &stored_before,
                charge,
            );
        }
        assert_eq!(fixture.total(TIP), i64::MAX);

        // A write that adds nothing to the total still goes through.
        let (outcome, _) = fixture.create(1, TIP, 0).await;
        SumFixture::expect_success(&outcome, "the create of 0");
    }

    /// The limit is exact: a total of `i64::MAX` is one GroveDB applies, in the type's tree,
    /// in each owner's sum and in the range sums over the owners.
    #[tokio::test]
    async fn should_apply_a_create_that_takes_the_total_to_the_most_it_holds() {
        let mut fixture = SumFixture::latest();
        let (outcome, _) = fixture.create(0, TIP, i64::MAX - 1).await;
        SumFixture::expect_success(&outcome, "the create of i64::MAX - 1");
        let (outcome, _) = fixture.create(1, TIP, 1).await;
        SumFixture::expect_success(&outcome, "the create of 1");
        assert_eq!(fixture.total(TIP), i64::MAX);
    }

    #[tokio::test]
    async fn should_hold_a_replace_raising_the_summed_property_to_the_types_total() {
        let mut fixture = SumFixture::latest();
        let (outcome, large) = fixture.create(0, TIP, i64::MAX - 5).await;
        SumFixture::expect_success(&outcome, "the create of i64::MAX - 5");
        let (outcome, small) = fixture.create(1, TIP, 2).await;
        SumFixture::expect_success(&outcome, "the create of 2");

        // 2 leaves the total and 6 enters it: one past the limit.
        let stored_before = fixture.stored(TIP);
        let (outcome, _) = fixture.replace(1, TIP, &small, 6).await;
        expect_refused(
            &fixture,
            "replace of 2 by 6",
            &outcome,
            TIP,
            i64::MAX,
            &stored_before,
            532_020,
        );
        assert_eq!(fixture.total(TIP), i64::MAX - 3);

        let (outcome, small) = fixture.replace(1, TIP, &small, 5).await;
        SumFixture::expect_success(&outcome, "the replace of 2 by 5");
        assert_eq!(fixture.total(TIP), i64::MAX);

        // A replace that lowers the property is not held to the total.
        let (outcome, _) = fixture.replace(1, TIP, &small, 3).await;
        SumFixture::expect_success(&outcome, "the replace of 5 by 3");
        let (outcome, _) = fixture.replace(0, TIP, &large, i64::MAX - 6).await;
        SumFixture::expect_success(&outcome, "the replace of i64::MAX - 5 by i64::MAX - 6");
        assert_eq!(fixture.total(TIP), i64::MAX - 3);
    }

    /// What a replace of a tip of 10 by one of `amount` charges its writer.
    async fn charge_of_replacing_a_tip_of_ten_by(amount: i64) -> Credits {
        let mut fixture = SumFixture::latest();
        let (outcome, tip) = fixture.create(0, TIP, 10).await;
        SumFixture::expect_success(&outcome, "the create of 10");
        let (outcome, _) = fixture.replace(0, TIP, &tip, amount).await;
        SumFixture::expect_success(&outcome, "the replace of 10");
        assert_eq!(fixture.total(TIP), amount);
        outcome.charged()
    }

    /// Only a replace that raises the summed property reads the total, and pays for the
    /// read: one that leaves the property as it was, or lowers it, is charged the same as the
    /// other, and less than one raising it. The three store the same bytes.
    #[tokio::test]
    async fn should_read_the_total_only_for_a_replace_raising_the_summed_property() {
        let unchanged = charge_of_replacing_a_tip_of_ten_by(10).await;
        let lowered = charge_of_replacing_a_tip_of_ten_by(9).await;
        let raised = charge_of_replacing_a_tip_of_ten_by(11).await;
        assert_eq!(unchanged, lowered);
        assert!(
            raised > unchanged,
            "a raising replace pays for the read of the total: {raised} against {unchanged}"
        );

        // At a full total a replace that leaves the property as it was is applied.
        let mut fixture = SumFixture::latest();
        let (outcome, tip) = fixture.create(0, TIP, i64::MAX).await;
        SumFixture::expect_success(&outcome, "the create of i64::MAX");
        let (outcome, _) = fixture.replace(0, TIP, &tip, i64::MAX).await;
        SumFixture::expect_success(&outcome, "the replace of i64::MAX by itself");
        assert_eq!(fixture.total(TIP), i64::MAX);
    }

    /// A transfer and a delete leave the total as it is or lower it, and with values that are
    /// never negative no sum of the type holds more than the total: at a full total an owner's
    /// sum takes the whole of it.
    #[tokio::test]
    async fn should_apply_a_transfer_and_a_delete_at_a_full_total() {
        let mut fixture = SumFixture::latest();
        let (outcome, large) = fixture.create(0, TIP, i64::MAX - 10).await;
        SumFixture::expect_success(&outcome, "the create of i64::MAX - 10");
        let (outcome, small) = fixture.create(1, TIP, 10).await;
        SumFixture::expect_success(&outcome, "the create of 10");
        assert_eq!(fixture.total(TIP), i64::MAX);

        let outcome = fixture.transfer(1, TIP, &small, 0).await;
        SumFixture::expect_success(&outcome, "the transfer of 10 to the owner of the rest");
        assert_eq!(fixture.total(TIP), i64::MAX);

        let outcome = fixture.delete(0, TIP, &large).await;
        SumFixture::expect_success(&outcome, "the delete of i64::MAX - 10");
        assert_eq!(fixture.total(TIP), 10);
    }

    /// An index keying a document by each of its tags adds its amount once per tag, so the
    /// index's sums can hold as many times the total as a document has tags, and the total is
    /// held to that fraction of `i64::MAX`.
    #[tokio::test]
    async fn should_hold_the_total_to_what_an_index_adding_several_entries_can_hold() {
        let limit = i64::MAX / MAX_TAGS;
        let mut fixture = SumFixture::latest();

        let stored_before = fixture.stored(TAGGED);
        let (outcome, _) = fixture.create(0, TAGGED, limit + 1).await;
        expect_refused(
            &fixture,
            "create of one past the limit",
            &outcome,
            TAGGED,
            limit,
            &stored_before,
            12_109_960,
        );

        let (outcome, _) = fixture.create(0, TAGGED, limit).await;
        SumFixture::expect_success(&outcome, "the create of the limit under every tag");
        assert_eq!(fixture.total(TAGGED), limit);

        let stored_before = fixture.stored(TAGGED);
        let (outcome, _) = fixture.create(1, TAGGED, 1).await;
        expect_refused(
            &fixture,
            "create of 1 at the limit",
            &outcome,
            TAGGED,
            limit,
            &stored_before,
            12_171_220,
        );
    }

    /// Protocol version 14 reads no total before a write: GroveDB refuses to apply the create,
    /// which ends as an internal error.
    #[tokio::test]
    async fn should_leave_a_create_past_the_total_an_internal_error_at_protocol_version_14() {
        let mut fixture = SumFixture::new(14);
        let (outcome, _) = fixture.create(0, TIP, i64::MAX).await;
        SumFixture::expect_success(&outcome, "the create of i64::MAX");

        let stored_before = fixture.stored(TIP);
        let (outcome, _) = fixture.create(0, TIP, 1).await;
        assert!(outcome.check_tx_valid);
        assert_matches!(
            outcome.result,
            StateTransitionExecutionResult::InternalError(_)
        );
        assert_eq!(outcome.nonce_after, outcome.nonce_before);
        assert_eq!(outcome.charged(), 0);
        assert_eq!(fixture.stored(TIP), stored_before);
    }
}
