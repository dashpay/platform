//! The DPNS contract v3 (protocol version 14) through the full ABCI pipeline. A
//! domain create is checked by schema keywords where the create data trigger
//! checked it before: `normalizedLabel` and `normalizedParentDomainName` are
//! generated from their counterparts, `preorderSalt` reveals the writer's own
//! preorder from an earlier block and deletes it, and the
//! `recordsIdentityIsOwner` rule holds a new name's identity record to its owner.
//! The trigger keeps only the parent domain checks. Protocol version 13 keeps
//! DPNS v2 and the whole trigger, pinned beside each case.

use super::*;

mod dpns_v3_tests {
    use super::*;
    use crate::execution::validation::state_transition::tests::{
        create_dpns_name_contest_give_key_info, perform_votes_multi,
    };
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::consensus::state::data_trigger::DataTriggerError;
    use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::data_contracts::SystemDataContract;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey, SecurityLevel};
    use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;
    use dpp::platform_value::string_encoding::Encoding;
    use dpp::prelude::{Identifier, IdentityNonce};
    use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
    use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionMutRef;
    use dpp::state_transition::StateTransition;
    use dpp::system_data_contracts::dpns_contract;
    use dpp::util::hash::hash_double;
    use dpp::util::strings::convert_to_homograph_safe_chars;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity;
    use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
    use drive::util::storage_flags::StorageFlags;
    use rand::prelude::StdRng;
    use rand::{Rng, SeedableRng};
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    /// The block preorders are made in.
    const PREORDER_HEIGHT: u64 = 10;

    /// The next block, where a domain may reveal a preorder made in
    /// [`PREORDER_HEIGHT`].
    const REVEAL_HEIGHT: u64 = PREORDER_HEIGHT + 1;

    /// Block `height`, a second after the one before it.
    fn block(height: u64) -> BlockInfo {
        BlockInfo {
            time_ms: 1_700_000_000_000 + height * 1_000,
            height,
            ..BlockInfo::default()
        }
    }

    /// The identity registering names, and another one.
    #[derive(Clone, Copy)]
    enum Who {
        Alice,
        Bob,
    }

    struct Writer {
        identity: Identity,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        /// The identity contract nonce the next transition uses; every
        /// processed transition consumes one, a refused one included.
        next_nonce: IdentityNonce,
    }

    struct DpnsFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        platform_version: &'static PlatformVersion,
        rng: StdRng,
        alice: Writer,
        bob: Writer,
    }

    impl DpnsFixture {
        /// A chain born at `protocol_version` with its genesis state, so the
        /// DPNS system contract of that version and its `dash` domain.
        fn new(protocol_version: u32) -> Self {
            let platform_version =
                PlatformVersion::get(protocol_version).expect("expected the platform version");
            let mut platform = TestPlatformBuilder::new()
                .with_initial_protocol_version(protocol_version)
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut writer = |seed| {
                let (identity, signer, key) =
                    setup_identity(&mut platform, seed, dash_to_credits!(0.5));
                Writer {
                    identity,
                    signer,
                    key,
                    next_nonce: 1,
                }
            };
            let alice = writer(5101);
            let bob = writer(5102);

            Self {
                platform,
                platform_version,
                rng: StdRng::seed_from_u64(5103),
                alice,
                bob,
            }
        }

        fn writer(&mut self, who: Who) -> &mut Writer {
            match who {
                Who::Alice => &mut self.alice,
                Who::Bob => &mut self.bob,
            }
        }

        fn id(&self, who: Who) -> Identifier {
            match who {
                Who::Alice => self.alice.identity.id(),
                Who::Bob => self.bob.identity.id(),
            }
        }

        /// The DPNS contract as stored, which is what a write is checked against.
        fn dpns(&self) -> DataContract {
            self.platform
                .drive
                .get_contract_with_fetch_info(
                    SystemDataContract::DPNS.id().to_buffer(),
                    false,
                    None,
                    self.platform_version,
                )
                .expect("expected to fetch the DPNS contract")
                .expect("expected the DPNS contract in state")
                .contract
                .clone()
        }

        fn process(
            &self,
            transition: &StateTransition,
            height: u64,
        ) -> StateTransitionExecutionResult {
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
                    &block(height),
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
            processing_result.into_execution_results().remove(0)
        }

        /// Creates a `type_name` document owned by `who` in block `height`,
        /// with `values` set over the random required ones and `absent`
        /// removed, and returns it with the result. The transition carries the
        /// document's properties exactly as set: the client builder would
        /// regenerate a wrong `generatedFrom` value from protocol version 14.
        async fn create(
            &mut self,
            who: Who,
            type_name: &str,
            values: &[(&str, Value)],
            absent: &[&str],
            height: u64,
        ) -> (Document, StateTransitionExecutionResult) {
            let dpns = self.dpns();
            self.create_in(&dpns, who, type_name, values, absent, height)
                .await
        }

        /// [`Self::create`] of a document of `contract`.
        async fn create_in(
            &mut self,
            contract: &DataContract,
            who: Who,
            type_name: &str,
            values: &[(&str, Value)],
            absent: &[&str],
            height: u64,
        ) -> (Document, StateTransitionExecutionResult) {
            let platform_version = self.platform_version;
            let owner_id = self.id(who);
            let document_type = contract
                .document_type_for_name(type_name)
                .expect("expected the document type");
            let entropy = Bytes32::random_with_rng(&mut self.rng);
            let mut document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut self.rng,
                    owner_id,
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");
            for (property, value) in values {
                document.set(property, value.clone());
            }
            for property in absent {
                document.remove(property);
            }
            let writer = self.writer(who);
            let nonce = writer.next_nonce;
            writer.next_nonce += 1;
            document
                .set_id_for_creation(document_type, &entropy.0, nonce, platform_version)
                .expect("expected to set the document id");
            let mut transition = BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                document_type,
                entropy.0,
                &writer.key,
                nonce,
                0,
                None,
                &writer.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the create transition");
            {
                let StateTransition::Batch(batch) = &mut transition else {
                    panic!("expected a batch transition");
                };
                let Some(BatchedTransitionMutRef::Document(document_transition)) =
                    batch.first_transition_mut()
                else {
                    panic!("expected a document transition");
                };
                *document_transition
                    .data_mut()
                    .expect("the document transition carries data") = document.properties().clone();
            }
            transition
                .sign_external(
                    &writer.key,
                    &writer.signer,
                    Some(|_, _| Ok(SecurityLevel::HIGH)),
                )
                .await
                .expect("expected to sign the transition again");
            let result = self.process(&transition, height);
            (document, result)
        }

        /// A preorder by `who` in block `height` of `label` under `parent`,
        /// salted with `salt`.
        async fn preorder(
            &mut self,
            who: Who,
            salt: [u8; 32],
            label: &str,
            parent: &str,
            height: u64,
        ) -> Document {
            let (preorder, result) = self
                .create(
                    who,
                    "preorder",
                    &[(
                        "saltedDomainHash",
                        salted_domain_hash(&salt, label, parent).into(),
                    )],
                    &[],
                    height,
                )
                .await;
            assert_matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                "the preorder must be accepted"
            );
            preorder
        }

        /// A domain by `who` in block `height` of `label` under `parent`,
        /// revealing `salt`, as a client builds it, with `values` set over it
        /// and `absent` removed.
        async fn domain(
            &mut self,
            who: Who,
            salt: [u8; 32],
            label: &str,
            parent: &str,
            values: &[(&str, Value)],
            absent: &[&str],
            height: u64,
        ) -> (Document, StateTransitionExecutionResult) {
            let mut all_values = vec![
                ("label", Value::Text(label.to_string())),
                ("parentDomainName", Value::Text(parent.to_string())),
                (
                    "normalizedLabel",
                    Value::Text(convert_to_homograph_safe_chars(label)),
                ),
                (
                    "normalizedParentDomainName",
                    Value::Text(convert_to_homograph_safe_chars(parent)),
                ),
                ("preorderSalt", salt.into()),
                ("records.identity", self.id(who).into()),
                ("subdomainRules.allowSubdomains", false.into()),
            ];
            all_values.extend(values.iter().cloned());
            self.create(who, "domain", &all_values, absent, height)
                .await
        }

        /// Preorders `label` under `dash` in [`PREORDER_HEIGHT`] and reveals it
        /// in [`REVEAL_HEIGHT`], both as `who`, with `values` and `absent`
        /// applied to the domain.
        async fn register(
            &mut self,
            who: Who,
            label: &str,
            values: &[(&str, Value)],
            absent: &[&str],
        ) -> (Document, Document, StateTransitionExecutionResult) {
            let salt: [u8; 32] = self.rng.gen();
            let preorder = self
                .preorder(who, salt, label, "dash", PREORDER_HEIGHT)
                .await;
            let (domain, result) = self
                .domain(who, salt, label, "dash", values, absent, REVEAL_HEIGHT)
                .await;
            (preorder, domain, result)
        }

        /// The documents of `type_name` with every `(property, value)` pair of
        /// `equal`, decoded with the stored DPNS contract.
        fn query(&self, type_name: &str, equal: &[(&str, Value)]) -> Vec<Document> {
            query_dpns(&self.platform, type_name, equal, self.platform_version)
        }

        /// The domain `label` under `dash`, if registered.
        fn name(&self, label: &str) -> Option<Document> {
            self.query(
                "domain",
                &[
                    ("normalizedParentDomainName", Value::Text("dash".into())),
                    (
                        "normalizedLabel",
                        Value::Text(convert_to_homograph_safe_chars(label)),
                    ),
                ],
            )
            .pop()
        }

        /// The preorder whose salted hash is `preorder`'s, if still stored.
        fn stored_preorder(&self, preorder: &Document) -> Option<Document> {
            let salted_domain_hash = preorder
                .properties()
                .get("saltedDomainHash")
                .expect("expected the preorder's salted hash")
                .clone();
            self.query("preorder", &[("saltedDomainHash", salted_domain_hash)])
                .pop()
        }

        /// Runs the protocol version 14 activation at block `height` on a
        /// chain at protocol version 13, as the first block of version 14 does.
        fn activate_protocol_version_14(&mut self, height: u64) {
            let platform_version_14 =
                PlatformVersion::get(14).expect("expected platform version 14");
            let platform_state = self.platform.state.load();
            let transaction = self.platform.drive.grove.start_transaction();
            self.platform
                .perform_events_on_first_block_of_protocol_change(
                    &platform_state,
                    &block(height),
                    &transaction,
                    13,
                    platform_version_14,
                )
                .expect("expected the transition to protocol version 14 to succeed");
            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit the transition to protocol version 14");
            drop(platform_state);
            let mut upgraded_state = self.platform.state.load().as_ref().clone();
            upgraded_state.set_current_protocol_version_in_consensus(14);
            upgraded_state.set_next_epoch_protocol_version(14);
            self.platform.state.store(Arc::new(upgraded_state));
            self.platform_version = platform_version_14;
        }
    }

    /// The documents of the stored DPNS contract's `type_name` with every
    /// `(property, value)` pair of `equal`.
    fn query_dpns(
        platform: &TempPlatform<MockCoreRPCLike>,
        type_name: &str,
        equal: &[(&str, Value)],
        platform_version: &PlatformVersion,
    ) -> Vec<Document> {
        let dpns = platform
            .drive
            .get_contract_with_fetch_info(
                SystemDataContract::DPNS.id().to_buffer(),
                false,
                None,
                platform_version,
            )
            .expect("expected to fetch the DPNS contract")
            .expect("expected the DPNS contract in state")
            .contract
            .clone();
        let document_type = dpns
            .document_type_for_name(type_name)
            .expect("expected the document type");
        let drive_query = DriveDocumentQuery {
            contract: &dpns,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clauses: Vec::new(),
                range_clause: None,
                equal_clauses: equal
                    .iter()
                    .map(|(field, value)| {
                        (
                            field.to_string(),
                            WhereClause {
                                field: field.to_string(),
                                operator: WhereOperator::Equal,
                                value: value.clone(),
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>(),
            },
            offset: None,
            limit: None,
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
            resolved_time_ranges: vec![],
            sub_queries: vec![],
        };
        platform
            .drive
            .query_documents(
                drive_query,
                None,
                false,
                None,
                Some(platform_version.protocol_version),
            )
            .expect("expected to query the documents")
            .documents_owned()
    }

    /// Registers a contract, owned by Alice, whose `claim` reveals a DPNS preorder
    /// (`preorderSalt` refers to it, found by the same hash) at least
    /// `minimum_age_blocks` old, and returns it.
    fn register_preorder_claims_contract(
        fixture: &DpnsFixture,
        minimum_age_blocks: u32,
    ) -> DataContract {
        let contract_json = serde_json::json!({
            "$formatVersion": "1",
            "id": Identifier::new([0x5C; 32]).to_string(Encoding::Base58),
            "ownerId": fixture.id(Who::Alice).to_string(Encoding::Base58),
            "version": 1,
            "documentSchemas": {
                "claim": {
                    "type": "object",
                    "documentsMutable": false,
                    "canBeDeleted": false,
                    "properties": {
                        "label": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
                        "parent": { "type": "string", "maxLength": 63, "position": 1 },
                        "preorderSalt": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 32,
                            "maxItems": 32,
                            "position": 2,
                            "refersTo": {
                                "type": "deletableDocument",
                                "contractId": SystemDataContract::DPNS.id().to_string(Encoding::Base58),
                                "documentType": "preorder",
                                "findBy": {
                                    "saltedDomainHash": {
                                        "function": "sys.hash.sha256d",
                                        "params": ["preorderSalt", "label", { "const": "." }, "parent"]
                                    }
                                },
                                "minimumAgeBlocks": minimum_age_blocks
                            }
                        }
                    },
                    "required": ["label", "parent", "preorderSalt"],
                    "additionalProperties": false
                }
            }
        });
        let contract = DataContract::from_json(contract_json, true, fixture.platform_version)
            .expect("expected to parse the claims contract");
        fixture
            .platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                fixture.platform_version,
            )
            .expect("expected to apply the claims contract");
        contract
    }

    /// The commitment a preorder stores, as every client computes it:
    /// `sha256d(salt ++ normalizedLabel ++ "." ++ parentDomainName)`.
    fn salted_domain_hash(salt: &[u8; 32], label: &str, parent: &str) -> [u8; 32] {
        let mut buffer = salt.to_vec();
        buffer.extend(convert_to_homograph_safe_chars(label).as_bytes());
        buffer.extend(b".");
        buffer.extend(parent.as_bytes());
        hash_double(buffer)
    }

    /// The consensus error of a refused write, printed with its code.
    fn refusal(result: &StateTransitionExecutionResult) -> &ConsensusError {
        match result {
            StateTransitionExecutionResult::PaidConsensusError { error, .. } => error,
            other => panic!("expected a paid consensus error, got {other:?}"),
        }
    }

    fn assert_data_trigger_refusal(result: &StateTransitionExecutionResult, message: &str) {
        assert_matches!(
            refusal(result),
            ConsensusError::StateError(StateError::DataTriggerError(
                DataTriggerError::DataTriggerConditionError(e)
            )) if e.message().starts_with(message),
            "expected the create trigger to refuse with \"{message}\""
        );
    }

    #[tokio::test]
    async fn should_register_a_name_revealing_the_writers_preorder_and_delete_the_preorder() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);

        // The client leaves both normalized names out: the platform generates them
        let (preorder, _, result) = fixture
            .register(
                Who::Alice,
                "Quantum7",
                &[],
                &["normalizedLabel", "normalizedParentDomainName"],
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let name = fixture.name("Quantum7").expect("expected the name");
        assert_eq!(name.owner_id(), fixture.id(Who::Alice));
        assert_eq!(
            name.properties()
                .get_str("normalizedLabel")
                .expect("expected the generated normalized label"),
            "quantum7"
        );
        assert_eq!(
            name.properties()
                .get_str("normalizedParentDomainName")
                .expect("expected the generated normalized parent"),
            "dash"
        );
        assert!(
            fixture.stored_preorder(&preorder).is_none(),
            "the create consumes the preorder"
        );
    }

    #[tokio::test]
    async fn should_refuse_a_reveal_in_the_block_of_its_preorder() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);
        let salt: [u8; 32] = fixture.rng.gen();
        let preorder = fixture
            .preorder(Who::Alice, salt, "quantum7", "dash", PREORDER_HEIGHT)
            .await;

        let (_, result) = fixture
            .domain(
                Who::Alice,
                salt,
                "quantum7",
                "dash",
                &[],
                &[],
                PREORDER_HEIGHT,
            )
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::StateError(StateError::ReferencedDocumentRequirementNotMetError(e))
                if e.field() == "minimumAgeBlocks"
        );
        assert!(fixture.name("quantum7").is_none());
        assert!(fixture.stored_preorder(&preorder).is_some());

        // The next block reveals it
        let (_, result) = fixture
            .domain(
                Who::Alice,
                salt,
                "quantum7",
                "dash",
                &[],
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    #[tokio::test]
    async fn should_refuse_a_reveal_of_another_identitys_preorder() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);
        let salt: [u8; 32] = fixture.rng.gen();
        let preorder = fixture
            .preorder(Who::Alice, salt, "quantum7", "dash", PREORDER_HEIGHT)
            .await;

        // Bob read Alice's salt off her domain create and races it
        let (_, result) = fixture
            .domain(Who::Bob, salt, "quantum7", "dash", &[], &[], REVEAL_HEIGHT)
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::StateError(StateError::ReferencedDocumentPropertyMismatchError(_))
        );
        assert!(fixture.name("quantum7").is_none());
        assert!(fixture.stored_preorder(&preorder).is_some());
    }

    #[tokio::test]
    async fn should_refuse_a_name_without_its_preorder() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);
        let salt: [u8; 32] = fixture.rng.gen();

        let (_, result) = fixture
            .domain(
                Who::Alice,
                salt,
                "quantum7",
                "dash",
                &[],
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_))
        );
        assert!(fixture.name("quantum7").is_none());
    }

    #[tokio::test]
    async fn should_refuse_a_normalized_label_other_than_the_one_generated() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);

        // Lowercased, but "o" not replaced by "0"
        let (preorder, _, result) = fixture
            .register(
                Who::Alice,
                "Bob7x",
                &[("normalizedLabel", Value::Text("bob7x".into()))],
                &[],
            )
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::BasicError(BasicError::DocumentPropertyNotGeneratedError(e))
                if e.property() == "normalizedLabel"
        );
        assert!(fixture.name("Bob7x").is_none());
        assert!(fixture.stored_preorder(&preorder).is_some());
    }

    #[tokio::test]
    async fn should_refuse_a_name_whose_identity_record_is_not_its_owner() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);
        let bob = fixture.id(Who::Bob);

        let (preorder, _, result) = fixture
            .register(
                Who::Alice,
                "quantum7",
                &[("records.identity", bob.into())],
                &[],
            )
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(e))
                if e.constraint() == "recordsIdentityIsOwner"
        );
        assert!(fixture.name("quantum7").is_none());
        assert!(fixture.stored_preorder(&preorder).is_some());
    }

    /// The create trigger keeps the parent domain checks at protocol version 14:
    /// each case reveals a preorder made for it, so only the trigger refuses it.
    #[tokio::test]
    async fn should_keep_the_parent_domain_checks_in_the_create_trigger() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);

        // A parent domain nobody registered
        let salt: [u8; 32] = fixture.rng.gen();
        fixture
            .preorder(Who::Alice, salt, "quantum7", "n0where7", PREORDER_HEIGHT)
            .await;
        let (_, result) = fixture
            .domain(
                Who::Alice,
                salt,
                "quantum7",
                "n0where7",
                &[],
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        assert_data_trigger_refusal(&result, "Parent domain is not present");

        // A name under `dash` opening subdomains: the references found and collected the
        // preorder for consumption before the trigger refused, and the refusal keeps it
        let salt: [u8; 32] = fixture.rng.gen();
        let preorder = fixture
            .preorder(Who::Alice, salt, "quantum8", "dash", PREORDER_HEIGHT)
            .await;
        let (_, result) = fixture
            .domain(
                Who::Alice,
                salt,
                "quantum8",
                "dash",
                &[("subdomainRules.allowSubdomains", true.into())],
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        assert_data_trigger_refusal(
            &result,
            "Allowing subdomains registration is forbidden for this domain",
        );
        assert!(fixture.stored_preorder(&preorder).is_some());
        assert!(fixture.name("quantum8").is_none());

        // The same salt, without subdomains, then reveals and consumes it
        let (_, result) = fixture
            .domain(
                Who::Alice,
                salt,
                "quantum8",
                "dash",
                &[],
                &[],
                REVEAL_HEIGHT + 1,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert!(fixture.stored_preorder(&preorder).is_none());
        assert!(fixture.name("quantum8").is_some());

        // A top-level domain by anyone but the DPNS contract owner
        let salt: [u8; 32] = fixture.rng.gen();
        fixture
            .preorder(Who::Alice, salt, "quantum9", "", PREORDER_HEIGHT)
            .await;
        let (_, result) = fixture
            .domain(Who::Alice, salt, "quantum9", "", &[], &[], REVEAL_HEIGHT)
            .await;
        assert_data_trigger_refusal(&result, "Can't create top level domain for this identity");
        assert_ne!(fixture.id(Who::Alice), dpns_contract::OWNER_ID);
    }

    /// Before protocol version 14 the whole create trigger runs on DPNS v2: a
    /// missing preorder and a wrong normalized label are its refusals, and it
    /// neither deletes the preorder nor reads the identity record.
    #[tokio::test]
    async fn should_keep_the_create_trigger_at_protocol_version_13() {
        let mut fixture = DpnsFixture::new(13);

        let salt: [u8; 32] = fixture.rng.gen();
        let (_, result) = fixture
            .domain(
                Who::Alice,
                salt,
                "quantum7",
                "dash",
                &[],
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        assert_data_trigger_refusal(&result, "preorderDocument was not found");

        let (_, _, result) = fixture
            .register(
                Who::Alice,
                "Bob8x",
                &[("normalizedLabel", Value::Text("b0b8y".into()))],
                &[],
            )
            .await;
        assert_data_trigger_refusal(&result, "Normalized label doesn't match label");

        let bob = fixture.id(Who::Bob);
        let (preorder, _, result) = fixture
            .register(
                Who::Alice,
                "quantum9",
                &[("records.identity", bob.into())],
                &[],
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert!(
            fixture.stored_preorder(&preorder).is_some(),
            "the trigger leaves the preorder in place"
        );
    }

    /// A transfer is judged against `recordsIdentityIsOwner` with its own time:
    /// in a later block than the create it no longer reads the identity
    /// record, which Drive then points at the new owner; in the block that
    /// created the name it is judged as the create was, and refused.
    #[tokio::test]
    async fn should_transfer_a_name_in_a_later_block_but_not_in_the_block_that_created_it() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);
        let bob = fixture.id(Who::Bob);
        let (_, mut name, result) = fixture.register(Who::Alice, "quantum7", &[], &[]).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        name.set_revision(Some(2));

        let dpns = fixture.dpns();
        let domain = dpns
            .document_type_for_name("domain")
            .expect("expected the domain document type");
        let transfer = |nonce| {
            let writer = &fixture.alice;
            BatchTransition::new_document_transfer_transition_from_document(
                name.clone(),
                domain,
                bob,
                &writer.key,
                nonce,
                0,
                None,
                &writer.signer,
                fixture.platform_version,
                None,
            )
        };
        let same_block = transfer(fixture.alice.next_nonce)
            .await
            .expect("expected the transfer transition");
        let later_block = transfer(fixture.alice.next_nonce + 1)
            .await
            .expect("expected the transfer transition");
        fixture.alice.next_nonce += 2;

        let result = fixture.process(&same_block, REVEAL_HEIGHT);
        assert_matches!(
            refusal(&result),
            ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(e))
                if e.constraint() == "recordsIdentityIsOwner"
        );

        let result = fixture.process(&later_block, REVEAL_HEIGHT + 1);
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let name = fixture.name("quantum7").expect("expected the name");
        assert_eq!(name.owner_id(), bob);
        assert_eq!(
            name.get("records.identity"),
            Some(&Value::Identifier(bob.to_buffer()))
        );
    }

    /// A chain upgraded from protocol version 13: DPNS is re-stored as v3,
    /// every domain and preorder written under v2 decodes unchanged, and a
    /// preorder made before the upgrade, which records no creation height, is
    /// old enough to reveal.
    #[tokio::test]
    async fn should_decode_v2_documents_and_reveal_a_v2_preorder_after_the_upgrade() {
        let mut fixture = DpnsFixture::new(13);

        let (_, registered, result) = fixture.register(Who::Bob, "quantum7", &[], &[]).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let salt: [u8; 32] = fixture.rng.gen();
        let pending = fixture
            .preorder(Who::Alice, salt, "quantum8", "dash", REVEAL_HEIGHT)
            .await;

        let v2_domain = fixture.name("quantum7").expect("expected the v2 domain");
        let v2_preorder = fixture
            .stored_preorder(&pending)
            .expect("expected the v2 preorder");
        assert_eq!(v2_domain.id(), registered.id());
        assert_eq!(v2_preorder.created_at_block_height(), None);
        let v2_dpns = fixture.dpns();
        assert!(v2_dpns
            .document_type_for_name("domain")
            .expect("expected the domain document type")
            .documents_can_be_deleted());

        fixture.activate_protocol_version_14(20);

        let v3_dpns = fixture.dpns();
        let v3_domain_type = v3_dpns
            .document_type_for_name("domain")
            .expect("expected the domain document type");
        assert!(!v3_domain_type.documents_can_be_deleted());
        assert!(v3_dpns
            .document_type_for_name("preorder")
            .expect("expected the preorder document type")
            .required_fields()
            .contains("$createdAtBlockHeight"));
        assert_eq!(
            fixture.name("quantum7").expect("expected the domain"),
            v2_domain,
            "a domain written under v2 decodes unchanged under v3"
        );
        assert_eq!(
            fixture
                .stored_preorder(&pending)
                .expect("expected the preorder"),
            v2_preorder,
            "a preorder written under v2 decodes unchanged under v3"
        );

        // Another contract revealing DPNS preorders, three blocks old at least: the upgrade
        // shows only that a preorder recording no height is from an earlier block, so
        // Alice's, made a block before the upgrade, meets no minimum above 1
        let claims = register_preorder_claims_contract(&fixture, 3);
        let (_, result) = fixture
            .create_in(
                &claims,
                Who::Alice,
                "claim",
                &[
                    ("label", Value::Text("quantum8".into())),
                    ("parent", Value::Text("dash".into())),
                    ("preorderSalt", salt.into()),
                ],
                &[],
                21,
            )
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::StateError(StateError::ReferencedDocumentRequirementNotMetError(e))
                if e.field() == "minimumAgeBlocks" && e.required() == "3"
        );

        // DPNS asks for one block, which it is
        let (_, result) = fixture
            .domain(Who::Alice, salt, "quantum8", "dash", &[], &[], 21)
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert!(fixture.name("quantum8").is_some());
        assert!(fixture.stored_preorder(&pending).is_none());

        // A preorder made from protocol version 14 on records its height
        let salt: [u8; 32] = fixture.rng.gen();
        let preorder = fixture
            .preorder(Who::Alice, salt, "quantum9", "dash", 22)
            .await;
        assert_eq!(
            fixture
                .stored_preorder(&preorder)
                .expect("expected the preorder")
                .created_at_block_height(),
            Some(22)
        );
    }

    /// A chain born at protocol version 14 stores DPNS v3 and still inserts
    /// the `dash` top-level domain, which `allowSubdomains` and whose owner is
    /// the DPNS contract owner.
    #[tokio::test]
    async fn should_store_dpns_v3_and_the_dash_domain_at_genesis() {
        let fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);
        let dpns = fixture.dpns();
        assert!(!dpns
            .document_type_for_name("domain")
            .expect("expected the domain document type")
            .documents_can_be_deleted());

        let dash = fixture
            .query(
                "domain",
                &[
                    ("normalizedParentDomainName", Value::Text(String::new())),
                    ("normalizedLabel", Value::Text("dash".into())),
                ],
            )
            .pop()
            .expect("expected the dash top-level domain");
        assert_eq!(
            dash.id(),
            Identifier::new(dpns_contract::DPNS_DASH_TLD_DOCUMENT_ID)
        );
        assert_eq!(dash.owner_id(), dpns_contract::OWNER_ID);
        assert!(dash
            .properties()
            .get_bool_at_path("subdomainRules.allowSubdomains")
            .expect("expected the subdomain rule"));
    }

    /// A contested name at protocol version 14: the create that joins the
    /// contest is judged by the rules and reveals and consumes its preorder
    /// there, though the name is only stored once the contest is awarded.
    #[tokio::test]
    async fn should_check_a_contested_name_at_join_and_consume_its_preorder() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);
        let bob = fixture.id(Who::Bob);

        // "quantum" matches the contested regex
        let (preorder, _, result) = fixture
            .register(
                Who::Alice,
                "quantum",
                &[("records.identity", bob.into())],
                &[],
            )
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(e))
                if e.constraint() == "recordsIdentityIsOwner"
        );
        assert!(fixture.stored_preorder(&preorder).is_some());

        let (preorder, _, result) = fixture.register(Who::Alice, "quantum", &[], &[]).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert!(
            fixture.stored_preorder(&preorder).is_none(),
            "joining the contest consumes the preorder"
        );
        assert!(
            fixture.name("quantum").is_none(),
            "the name waits for the contest"
        );
    }

    /// Two contenders open a contest on a name at protocol version 14, each
    /// consuming its own preorder, and the award stores the winner's name.
    #[tokio::test]
    async fn should_award_a_contested_name_to_its_winner() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();

        let (
            (contender_1, _, _, (preorder_1, _), _),
            (_contender_2, _, _, (preorder_2, _), _),
            dpns_contract,
        ) = create_dpns_name_contest_give_key_info(
            &mut platform,
            &platform_state,
            7,
            "quantum",
            platform_version,
        )
        .await;
        for preorder in [&preorder_1, &preorder_2] {
            let salted_domain_hash = preorder
                .properties()
                .get("saltedDomainHash")
                .expect("expected the preorder's salted hash")
                .clone();
            assert!(
                query_dpns(
                    &platform,
                    "preorder",
                    &[("saltedDomainHash", salted_domain_hash)],
                    platform_version,
                )
                .is_empty(),
                "each join consumes its contender's preorder"
            );
        }

        perform_votes_multi(
            &mut platform,
            dpns_contract.as_ref(),
            vec![(TowardsIdentity(contender_1.id()), 1)],
            "quantum",
            10,
            None,
            platform_version,
        )
        .await;

        // Two weeks and 300 seconds on, past the contest's end
        let block_info = BlockInfo {
            time_ms: 1_209_900_000,
            height: 10_000,
            core_height: 42,
            epoch: Default::default(),
        };
        let mut ended_state = platform.state.load().as_ref().clone();
        ended_state.set_last_committed_block_info(Some(
            ExtendedBlockInfoV0 {
                basic_info: block_info,
                app_hash: platform
                    .drive
                    .grove
                    .root_hash(None, &platform_version.drive.grove_version)
                    .unwrap()
                    .expect("expected the root hash"),
                quorum_hash: [0u8; 32],
                block_id_hash: [0u8; 32],
                proposer_pro_tx_hash: [0u8; 32],
                signature: [0u8; 96],
                round: 0,
            }
            .into(),
        ));
        platform.state.store(Arc::new(ended_state));
        let platform_state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();
        platform
            .check_for_ended_vote_polls(
                &platform_state,
                &platform_state,
                &block_info,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to check for ended vote polls");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit the transaction");

        let name = query_dpns(
            &platform,
            "domain",
            &[
                ("normalizedParentDomainName", Value::Text("dash".into())),
                ("normalizedLabel", Value::Text("quantum".into())),
            ],
            platform_version,
        )
        .pop()
        .expect("expected the awarded name");
        assert_eq!(name.owner_id(), contender_1.id());
        assert_eq!(
            name.get("records.identity"),
            Some(&Value::Identifier(contender_1.id().to_buffer()))
        );
    }

    /// What a client pays for a name: the domain create at protocol version 13
    /// (create trigger, preorder kept) and at 14 (keywords, preorder consumed
    /// and its storage refunded to the writer). Printed for the record and
    /// pinned.
    #[tokio::test]
    async fn should_charge_a_domain_create_at_protocol_versions_13_and_14() {
        let mut measured = vec![];
        for protocol_version in [13, 14] {
            let mut fixture = DpnsFixture::new(protocol_version);
            let alice = fixture.id(Who::Alice);
            let (_, _, result) = fixture.register(Who::Alice, "quantum7", &[], &[]).await;
            let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = result
            else {
                panic!("expected the domain create to succeed, got {result:?}");
            };
            let refund = fee_result
                .fee_refunds
                .calculate_refunds_amount_for_identity(alice)
                .unwrap_or_default();
            println!(
                "protocol version {protocol_version}: processing fee {}, storage fee {}, \
                 refund {refund}",
                fee_result.processing_fee, fee_result.storage_fee
            );
            measured.push((fee_result.processing_fee, fee_result.storage_fee, refund));
        }
        assert_eq!(
            measured,
            vec![
                (1_876_280, 42_660_000, 0),
                (2_318_400, 42_687_000, 15_181_239)
            ]
        );
    }
}
