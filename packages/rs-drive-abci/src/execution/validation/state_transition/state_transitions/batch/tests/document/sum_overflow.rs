//! A document write whose summed value would take a document sum past what a sum tree holds:
//! a per-value sum (an index `summable` by the property), a range sum (`rangeSummable`, a
//! provable sum tree) and a document type's sum (`documentsSummable`), written by a create, a
//! replace or a transfer.

use super::*;

mod sum_overflow_tests {
    use super::*;
    use crate::execution::check_tx::CheckTxLevel;
    use crate::platform_types::platform::PlatformRef;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::fee::fee_result::FeeResult;
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

    /// The document type whose per-owner index is `summable` by `amount`.
    const PER_VALUE: &str = "perValue";
    /// The document type whose per-owner index is `rangeSummable` by `amount`.
    const RANGED: &str = "ranged";
    /// The document type that is `documentsSummable` by `amount`.
    const TYPED: &str = "typed";

    /// A mutable, transferable type with a signed `amount` (which a sum takes up to
    /// `i64::MAX`), declaring the sums `sums` adds to the schema.
    fn summed_schema(sums: impl FnOnce(&mut Value)) -> Value {
        let mut schema = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "transferable": 1,
            "properties": {
                "amount": {
                    "type": "integer",
                    "minimum": -1000,
                    "maximum": i64::MAX,
                    "position": 0
                }
            },
            "required": ["amount"],
            "additionalProperties": false
        });
        sums(&mut schema);
        schema
    }

    fn owner_index(range: bool) -> Value {
        let mut index = platform_value!({
            "name": "byOwner",
            "properties": [{ "$ownerId": "asc" }],
            "summable": "amount"
        });
        if range {
            index
                .insert("rangeSummable".to_string(), Value::Bool(true))
                .expect("expected a map");
        }
        index
    }

    type Writer = (Identity, SimpleSigner, IdentityPublicKey);

    /// Two identities and one contract with the three summed types.
    struct SumFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        platform_version: &'static PlatformVersion,
        writers: Vec<Writer>,
        contract: DataContract,
        next_nonces: BTreeMap<Identifier, IdentityNonce>,
        block_info: BlockInfo,
        /// Whether transitions are processed as the proposer builds its block, each in a
        /// savepoint of its own, rather than as a validator checks a received one.
        proposing: bool,
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
            let schemas = [
                (
                    PER_VALUE,
                    summed_schema(|schema| {
                        schema
                            .insert(
                                "indices".to_string(),
                                Value::Array(vec![owner_index(false)]),
                            )
                            .expect("expected a map")
                    }),
                ),
                (
                    RANGED,
                    summed_schema(|schema| {
                        schema
                            .insert("indices".to_string(), Value::Array(vec![owner_index(true)]))
                            .expect("expected a map")
                    }),
                ),
                (
                    TYPED,
                    summed_schema(|schema| {
                        schema
                            .insert(
                                "documentsSummable".to_string(),
                                Value::Text("amount".to_string()),
                            )
                            .expect("expected a map")
                    }),
                ),
            ];
            for (name, schema) in schemas {
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
                proposing: false,
            }
        }

        fn latest() -> Self {
            Self::new(PlatformVersion::latest().protocol_version)
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
                    self.proposing,
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

    /// Prints what a transition that would overflow a sum did, for the record.
    fn report(what: &str, outcome: &Outcome) {
        let result = match &outcome.result {
            StateTransitionExecutionResult::SuccessfulExecution { .. } => "success".to_string(),
            StateTransitionExecutionResult::PaidConsensusError { error, .. } => {
                format!("paid consensus error {}", error.code())
            }
            StateTransitionExecutionResult::UnpaidConsensusError(error) => {
                format!("unpaid consensus error {}", error.code())
            }
            StateTransitionExecutionResult::InternalError(error) => {
                format!("internal error: {error}")
            }
            StateTransitionExecutionResult::NotExecuted(reason) => {
                format!("not executed: {reason:?}")
            }
        };
        println!(
            "{what}: check_tx valid {}, block: {result}, nonce {:?} -> {:?}, charged {}",
            outcome.check_tx_valid,
            outcome.nonce_before,
            outcome.nonce_after,
            outcome.charged()
        );
    }

    /// Asserts a write that would overflow a sum ended as a paid consensus error that bumped
    /// the writer's nonce and charged a fee, leaving the documents as they were.
    fn expect_paid_overflow(
        fixture: &SumFixture,
        what: &str,
        outcome: &Outcome,
        document_type: &str,
        stored_before: &BTreeMap<Identifier, (Identifier, i64)>,
    ) {
        report(what, outcome);
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
        println!("{what}: fee {actual_fees:?}");
        assert_eq!(actual_fees.storage_fee, 0, "{what}: nothing is stored");
        assert_matches!(
            error,
            ConsensusError::StateError(StateError::DocumentSumOverflowError(_)),
            "{what}"
        );
        assert_eq!(error.code(), 40154, "{what}");
        assert_eq!(
            outcome
                .nonce_after
                .map(|nonce| nonce & IDENTITY_NONCE_VALUE_FILTER),
            Some(outcome.transition_nonce),
            "{what}: the nonce is bumped to the transition's"
        );
        assert!(outcome.charged() > 0, "{what}: the writer pays");
        assert_eq!(
            &fixture.stored(document_type),
            stored_before,
            "{what}: the documents are as they were"
        );
    }

    /// A create, then a replace, of a document whose amount takes the writer's or the type's
    /// sum past `i64::MAX`.
    async fn create_and_replace_past_the_sum(document_type: &str) {
        let mut fixture = SumFixture::latest();
        let (outcome, _) = fixture.create(0, document_type, i64::MAX).await;
        SumFixture::expect_success(&outcome, "the create of i64::MAX");

        let stored_before = fixture.stored(document_type);
        let (outcome, _) = fixture.create(0, document_type, 1).await;
        expect_paid_overflow(
            &fixture,
            &format!("{document_type}: create of 1"),
            &outcome,
            document_type,
            &stored_before,
        );

        let (outcome, zero) = fixture.create(0, document_type, 0).await;
        SumFixture::expect_success(&outcome, "the create of 0");
        let stored_before = fixture.stored(document_type);
        let (outcome, _) = fixture.replace(0, document_type, &zero, 1).await;
        expect_paid_overflow(
            &fixture,
            &format!("{document_type}: replace of 0 by 1"),
            &outcome,
            document_type,
            &stored_before,
        );

        // The writer's next write within the sum still goes through.
        let (outcome, _) = fixture.replace(0, document_type, &zero, -5).await;
        SumFixture::expect_success(&outcome, "the replace of 0 by -5");
    }

    /// A transfer of a document of 1 to an owner whose sum is `i64::MAX`, the sender's sum
    /// being 0 (it also owns a document of -1).
    async fn transfer_past_the_owner_sum(document_type: &str) {
        let mut fixture = SumFixture::latest();
        let (outcome, _) = fixture.create(0, document_type, i64::MAX).await;
        SumFixture::expect_success(&outcome, "the create of i64::MAX");
        let (outcome, _) = fixture.create(1, document_type, -1).await;
        SumFixture::expect_success(&outcome, "the create of -1");
        let (outcome, one) = fixture.create(1, document_type, 1).await;
        SumFixture::expect_success(&outcome, "the create of 1");

        let stored_before = fixture.stored(document_type);
        let outcome = fixture.transfer(1, document_type, &one, 0).await;
        expect_paid_overflow(
            &fixture,
            &format!("{document_type}: transfer of 1"),
            &outcome,
            document_type,
            &stored_before,
        );
    }

    #[tokio::test]
    async fn should_refuse_a_create_or_replace_past_a_per_value_sum_as_a_paid_error() {
        create_and_replace_past_the_sum(PER_VALUE).await;
    }

    #[tokio::test]
    async fn should_refuse_a_create_or_replace_past_a_range_sum_as_a_paid_error() {
        create_and_replace_past_the_sum(RANGED).await;
    }

    #[tokio::test]
    async fn should_refuse_a_create_or_replace_past_a_document_type_sum_as_a_paid_error() {
        create_and_replace_past_the_sum(TYPED).await;
    }

    #[tokio::test]
    async fn should_refuse_a_transfer_past_a_per_value_sum_as_a_paid_error() {
        transfer_past_the_owner_sum(PER_VALUE).await;
    }

    #[tokio::test]
    async fn should_refuse_a_transfer_past_a_range_sum_as_a_paid_error() {
        transfer_past_the_owner_sum(RANGED).await;
    }

    /// The proposer, which takes a savepoint per transition, refuses the write the same way.
    #[tokio::test]
    async fn should_refuse_a_create_past_a_sum_as_a_paid_error_when_proposing() {
        let mut fixture = SumFixture::latest();
        fixture.proposing = true;
        let (outcome, _) = fixture.create(0, PER_VALUE, i64::MAX).await;
        SumFixture::expect_success(&outcome, "the create of i64::MAX");

        let stored_before = fixture.stored(PER_VALUE);
        let (outcome, _) = fixture.create(0, PER_VALUE, 1).await;
        expect_paid_overflow(
            &fixture,
            "proposing: create of 1",
            &outcome,
            PER_VALUE,
            &stored_before,
        );
        let (outcome, _) = fixture.create(0, PER_VALUE, -1).await;
        SumFixture::expect_success(&outcome, "the create of -1");
    }

    /// Protocol version 14 is unchanged: the write ends as an internal error, which neither
    /// bumps the nonce nor charges a fee.
    #[tokio::test]
    async fn should_leave_a_create_past_a_sum_an_internal_error_at_protocol_version_14() {
        let mut fixture = SumFixture::new(14);
        let (outcome, _) = fixture.create(0, PER_VALUE, i64::MAX).await;
        SumFixture::expect_success(&outcome, "the create of i64::MAX");

        let stored_before = fixture.stored(PER_VALUE);
        let (outcome, _) = fixture.create(0, PER_VALUE, 1).await;
        report("protocol version 14: create of 1", &outcome);
        assert!(outcome.check_tx_valid);
        assert_matches!(
            outcome.result,
            StateTransitionExecutionResult::InternalError(_)
        );
        assert_eq!(outcome.nonce_after, outcome.nonce_before);
        assert_eq!(outcome.charged(), 0);
        assert_eq!(fixture.stored(PER_VALUE), stored_before);
    }

    /// The genesis height takes no savepoint (its re-proposals rewind to the one `init_chain`
    /// set), so a write past a sum there stays an internal error.
    #[tokio::test]
    async fn should_leave_a_create_past_a_sum_an_internal_error_at_the_genesis_height() {
        let mut fixture = SumFixture::latest();
        fixture.block_info.height = fixture.platform.config.abci.genesis_height;
        let (outcome, _) = fixture.create(0, PER_VALUE, i64::MAX).await;
        SumFixture::expect_success(&outcome, "the create of i64::MAX");

        let (outcome, _) = fixture.create(0, PER_VALUE, 1).await;
        assert_matches!(
            outcome.result,
            StateTransitionExecutionResult::InternalError(_)
        );
        assert_eq!(outcome.nonce_after, outcome.nonce_before);
        assert_eq!(outcome.charged(), 0);
    }

    /// A write within the sums costs what it did before: the same writes at the genesis height,
    /// where no refusal is prepared, and at another height are charged the same fees.
    #[tokio::test]
    async fn should_charge_a_write_within_the_sums_what_it_cost_before() {
        async fn fees(at_genesis: bool) -> Vec<FeeResult> {
            let mut fixture = SumFixture::latest();
            if at_genesis {
                fixture.block_info.height = fixture.platform.config.abci.genesis_height;
            }
            let mut fees = vec![];
            for document_type in [PER_VALUE, RANGED, TYPED] {
                let (outcome, document) = fixture.create(0, document_type, 40).await;
                fees.push(fee_of(&outcome));
                let (outcome, document) = fixture.replace(0, document_type, &document, -7).await;
                fees.push(fee_of(&outcome));
                let outcome = fixture.transfer(0, document_type, &document, 1).await;
                fees.push(fee_of(&outcome));
            }
            fees
        }
        fn fee_of(outcome: &Outcome) -> FeeResult {
            let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } =
                &outcome.result
            else {
                panic!("expected the write to succeed, got {:?}", outcome.result);
            };
            fee_result.clone()
        }
        assert_eq!(fees(false).await, fees(true).await);
    }
}
