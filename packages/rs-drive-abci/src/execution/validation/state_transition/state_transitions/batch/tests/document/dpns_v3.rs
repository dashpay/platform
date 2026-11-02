//! The DPNS contract v3 (protocol version 15) through the full ABCI pipeline. A
//! domain create is checked by schema keywords where the create data trigger
//! checked it before: `normalizedLabel` and `normalizedParentDomainName` are
//! generated from their counterparts, `creatorRefersTo` finds the creator's own
//! preorder of the name (by the creator and the hash of the normalized name) from
//! an earlier block, refuses it while another identity holds a valid preorder of
//! the name from an earlier block (`chooseFirst`, a preorder counting when it holds
//! the hash of its own owner's id and the name), and deletes it, and the
//! `recordsIdentityIsOwner` rule holds a new name's identity record to its owner. A
//! preorder carries no salt from v3 and lives an hour (`ttl`).
//! The trigger keeps only the parent domain checks. Protocol version 14 keeps
//! DPNS v2 and the whole trigger, pinned beside each case.

use super::*;

mod dpns_v3_tests {
    use super::*;
    use crate::execution::validation::state_transition::tests::{
        create_dpns_name_contest_give_key_info, perform_votes_multi,
    };
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::dpns::dpns_preorder_properties;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::consensus::state::data_trigger::DataTriggerError;
    use dpp::data_contract::DataContract;
    use dpp::data_contracts::SystemDataContract;
    use dpp::document::{Document, DocumentV0};
    use dpp::identity::{Identity, IdentityPublicKey, SecurityLevel};
    use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;
    use dpp::prelude::{Identifier, IdentityNonce};
    use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
    use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionMutRef;
    use dpp::state_transition::StateTransition;
    use dpp::system_data_contracts::dpns_contract;
    use dpp::util::strings::convert_to_homograph_safe_chars;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity;
    use drive::drive::RootTree;
    use drive::grovedb::Element;
    use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
    use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
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
        /// regenerate a wrong `generatedFrom` value from protocol version 14 on.
        async fn create(
            &mut self,
            who: Who,
            type_name: &str,
            values: &[(&str, Value)],
            absent: &[&str],
            height: u64,
        ) -> (Document, StateTransitionExecutionResult) {
            let platform_version = self.platform_version;
            let owner_id = self.id(who);
            let dpns = self.dpns();
            let document_type = dpns
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

        /// A preorder by `who` in block `height` of `label` under `parent`, with
        /// the hashes the DPNS contract of the fixture's protocol version declares
        /// (`salt` is read under DPNS v2 only).
        async fn preorder(
            &mut self,
            who: Who,
            salt: [u8; 32],
            label: &str,
            parent: &str,
            height: u64,
        ) -> Document {
            let hashes = dpns_preorder_properties(
                self.id(who),
                &salt,
                &convert_to_homograph_safe_chars(label),
                parent,
                self.platform_version,
            );
            let values: Vec<(&str, Value)> = hashes
                .iter()
                .map(|(property, value)| (property.as_str(), value.clone()))
                .collect();
            let (preorder, result) = self.create(who, "preorder", &values, &[], height).await;
            assert_matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                "the preorder must be accepted"
            );
            preorder
        }

        /// A domain by `who` in block `height` of `label` under `parent`, as a
        /// client builds it (revealing `salt` under DPNS v2 only), with `values`
        /// set over it and `absent` removed.
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
                ("records.identity", self.id(who).into()),
                ("subdomainRules.allowSubdomains", false.into()),
            ];
            if self.platform_version.system_data_contracts.dpns < 3 {
                all_values.push(("preorderSalt", salt.into()));
            }
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

        /// The stored preorder `preorder` is, found by its id.
        fn stored_preorder(&self, preorder: &Document) -> Option<Document> {
            let dpns = self.dpns();
            let document_type = dpns
                .document_type_for_name("preorder")
                .expect("expected the preorder document type");
            self.platform
                .drive
                .query_documents(
                    DriveDocumentQuery::new_primary_key_single_item_query(
                        &dpns,
                        document_type,
                        preorder.id(),
                    ),
                    None,
                    false,
                    None,
                    Some(self.platform_version.protocol_version),
                )
                .expect("expected to query the preorder")
                .documents_owned()
                .pop()
        }

        /// Runs the protocol version 15 activation at block `height` on a
        /// chain at protocol version 14, as the first block of version 15 does.
        fn activate_protocol_version_15(&mut self, height: u64) {
            self.activate_protocol_version_15_from(14, height);
        }

        /// Runs the protocol version 15 activation at block `height` on a chain
        /// at `previous`, as the first block of version 15 does: every rung of
        /// the upgrade ladder above `previous`, each with version 15's tables.
        fn activate_protocol_version_15_from(&mut self, previous: u32, height: u64) {
            let platform_version_15 =
                PlatformVersion::get(15).expect("expected platform version 15");
            let platform_state = self.platform.state.load();
            let transaction = self.platform.drive.grove.start_transaction();
            self.platform
                .perform_events_on_first_block_of_protocol_change(
                    &platform_state,
                    &block(height),
                    &transaction,
                    previous,
                    platform_version_15,
                )
                .expect("expected the transition to protocol version 15 to succeed");
            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit the transition to protocol version 15");
            drop(platform_state);
            let mut upgraded_state = self.platform.state.load().as_ref().clone();
            upgraded_state.set_current_protocol_version_in_consensus(15);
            upgraded_state.set_next_epoch_protocol_version(15);
            self.platform.state.store(Arc::new(upgraded_state));
            self.platform_version = platform_version_15;
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

    /// The elements laying out the DPNS `preorder` type: its tree, its primary key
    /// tree, its `ownerAndNameHash` and `nameHashAndHeight` index trees, and
    /// whatever its v2 `saltedHash` index left under `saltedDomainHash`.
    fn preorder_type_layout(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) -> (Element, Element, Element, Element, Option<Element>) {
        let dpns_id = SystemDataContract::DPNS.id().to_buffer();
        let documents_root: &[u8] = Into::<&[u8; 1]>::into(RootTree::DataContractDocuments);
        let documents_path: [&[u8]; 3] = [documents_root, &dpns_id, &[1]];
        let type_path: [&[u8]; 4] = [documents_root, &dpns_id, &[1], b"preorder"];
        let grove_version = &platform_version.drive.grove_version;
        let get = |path: &[&[u8]], key: &[u8]| {
            platform
                .drive
                .grove
                .get_raw_optional(path.into(), key, None, grove_version)
                .unwrap()
                .expect("expected to read the preorder layout")
        };
        (
            get(&documents_path, b"preorder").expect("expected the preorder type tree"),
            get(&type_path, &[0]).expect("expected the primary key tree"),
            get(&type_path, b"$ownerId").expect("expected the owner index tree"),
            get(&type_path, b"nameHash").expect("expected the name hash index tree"),
            get(&type_path, b"saltedDomainHash"),
        )
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

        // Bob read the name off Alice's domain create and races it: his reveal looks among
        // his own preorders, so it finds nothing
        let (_, result) = fixture
            .domain(Who::Bob, salt, "quantum7", "dash", &[], &[], REVEAL_HEIGHT)
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_))
        );
        assert!(fixture.name("quantum7").is_none());
        assert!(fixture.stored_preorder(&preorder).is_some());
    }

    /// Preorders are unique per owner and count only when their `ownerNameHash` binds
    /// their own owner, so a copy of someone's preorder is harmless: stored in an earlier
    /// block, it neither stops the owner's own preorder nor precedes it, and the copier,
    /// who learns the name from the owner's reveal, cannot reveal it with the copy.
    #[tokio::test]
    async fn should_neither_block_nor_reveal_a_name_with_a_copied_preorder() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);
        let alice = fixture.id(Who::Alice);
        let salt: [u8; 32] = fixture.rng.gen();
        let alice_hashes =
            dpns_preorder_properties(alice, &salt, "quantum7", "dash", fixture.platform_version);
        let copied: Vec<(&str, Value)> = alice_hashes
            .iter()
            .map(|(property, value)| (property.as_str(), value.clone()))
            .collect();

        // Bob copies Alice's pending commitment, and his copy lands a block earlier
        let (copy, result) = fixture
            .create(Who::Bob, "preorder", &copied, &[], PREORDER_HEIGHT - 1)
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let preorder = fixture
            .preorder(Who::Alice, salt, "quantum7", "dash", PREORDER_HEIGHT)
            .await;

        // Bob races Alice's reveal with the name and his copy
        let (_, result) = fixture
            .domain(Who::Bob, salt, "quantum7", "dash", &[], &[], REVEAL_HEIGHT)
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_))
        );

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
        assert_eq!(
            fixture
                .name("quantum7")
                .expect("expected the name")
                .owner_id(),
            alice
        );
        assert!(fixture.stored_preorder(&preorder).is_none());
        assert!(
            fixture.stored_preorder(&copy).is_some(),
            "the copy is Bob's, untouched"
        );
    }

    /// Without a salt, an owner's preorders of one name share their `nameHash`, so
    /// an owner holds one pending preorder per name: a second is refused while the first
    /// stands, and the first reveals the name
    #[tokio::test]
    async fn should_hold_one_pending_preorder_per_name_and_owner() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);
        let salt: [u8; 32] = fixture.rng.gen();
        let preorder = fixture
            .preorder(Who::Alice, salt, "quantum7", "dash", PREORDER_HEIGHT)
            .await;

        let alice = fixture.id(Who::Alice);
        let hashes =
            dpns_preorder_properties(alice, &salt, "quantum7", "dash", fixture.platform_version);
        let values: Vec<(&str, Value)> = hashes
            .iter()
            .map(|(property, value)| (property.as_str(), value.clone()))
            .collect();
        let (_, result) = fixture
            .create(Who::Alice, "preorder", &values, &[], REVEAL_HEIGHT)
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::StateError(StateError::DuplicateUniqueIndexError(e))
                if e.duplicating_properties() == &vec!["$ownerId".to_string(), "nameHash".to_string()]
        );

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
        assert!(fixture.stored_preorder(&preorder).is_none());
    }

    /// The race a full block opens: Bob reads the name off Alice's pending domain create,
    /// preorders it himself, his preorder valid as he knows the name, and reveals before
    /// Alice's create lands. Alice's preorder is older and valid, so his reveal is refused
    /// and hers then registers the name.
    #[tokio::test]
    async fn should_refuse_a_reveal_preceded_by_another_identitys_valid_preorder() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);
        let salt: [u8; 32] = fixture.rng.gen();
        let alice_preorder = fixture
            .preorder(Who::Alice, salt, "quantum7", "dash", PREORDER_HEIGHT)
            .await;
        let bob_preorder = fixture
            .preorder(Who::Bob, salt, "quantum7", "dash", REVEAL_HEIGHT)
            .await;

        let (_, result) = fixture
            .domain(
                Who::Bob,
                salt,
                "quantum7",
                "dash",
                &[],
                &[],
                REVEAL_HEIGHT + 1,
            )
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::StateError(StateError::ReferencedDocumentPrecededError(_))
        );
        assert!(fixture.name("quantum7").is_none());
        assert!(fixture.stored_preorder(&bob_preorder).is_some());

        let (_, result) = fixture
            .domain(
                Who::Alice,
                salt,
                "quantum7",
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
        assert_eq!(
            fixture
                .name("quantum7")
                .expect("expected the name")
                .owner_id(),
            fixture.id(Who::Alice)
        );
        assert!(fixture.stored_preorder(&alice_preorder).is_none());
    }

    /// A name holds at most `max_choose_first_candidates` preorders (`atMostAThousandPerName`),
    /// so the preorders a reveal reads are always all of those before its own, and no copy can
    /// push an older valid preorder past the read. Here 999 copies of the name's hash, each
    /// another owner's and none valid, come a block before Alice's valid preorder, the
    /// thousandth. Bob, who learned the name, cannot preorder it after her (10422), and Alice's
    /// reveal, which reads all 999 copies before her preorder, registers the name.
    #[tokio::test]
    async fn should_refuse_a_preorder_past_the_preorders_a_reveal_reads() {
        let mut fixture = DpnsFixture::new(PlatformVersion::latest().protocol_version);
        let platform_version = fixture.platform_version;
        let cap = usize::from(platform_version.system_limits.max_choose_first_candidates);
        let salt: [u8; 32] = fixture.rng.gen();
        let name_hash = dpns_preorder_properties(
            fixture.id(Who::Alice),
            &salt,
            "quantum7",
            "dash",
            platform_version,
        )
        .into_iter()
        .find_map(|(property, value)| (property == "nameHash").then_some(value))
        .expect("a DPNS v3 preorder holds the name's hash");

        // The copies are stored directly: as transitions they would need 999 identities
        let dpns = fixture.dpns();
        let preorder_type = dpns
            .document_type_for_name("preorder")
            .expect("expected the preorder type");
        let copied_in = block(PREORDER_HEIGHT - 1);
        for copy in 0..cap - 1 {
            let mut id = [0u8; 32];
            id[..8].copy_from_slice(&(copy as u64).to_be_bytes());
            let document: Document = DocumentV0 {
                id: Identifier::new(id),
                owner_id: Identifier::new(fixture.rng.gen()),
                properties: BTreeMap::from([
                    ("nameHash".to_string(), name_hash.clone()),
                    (
                        "ownerNameHash".to_string(),
                        Value::Bytes32(fixture.rng.gen()),
                    ),
                ]),
                created_at: Some(copied_in.time_ms),
                created_at_block_height: Some(copied_in.height),
                ..Default::default()
            }
            .into();
            fixture
                .platform
                .drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&document, None)),
                            owner_id: None,
                        },
                        contract: &dpns,
                        document_type: preorder_type,
                    },
                    false,
                    copied_in,
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to store the copy");
        }

        let alice_preorder = fixture
            .preorder(Who::Alice, salt, "quantum7", "dash", PREORDER_HEIGHT)
            .await;

        let bob_hashes = dpns_preorder_properties(
            fixture.id(Who::Bob),
            &salt,
            "quantum7",
            "dash",
            platform_version,
        );
        let bob_values: Vec<(&str, Value)> = bob_hashes
            .iter()
            .map(|(property, value)| (property.as_str(), value.clone()))
            .collect();
        let (_, result) = fixture
            .create(Who::Bob, "preorder", &bob_values, &[], PREORDER_HEIGHT)
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(e))
                if e.constraint() == "atMostAThousandPerName"
        );

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
        assert_eq!(
            fixture
                .name("quantum7")
                .expect("expected the name")
                .owner_id(),
            fixture.id(Who::Alice)
        );
        assert!(fixture.stored_preorder(&alice_preorder).is_none());
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

    /// The create trigger keeps the parent domain checks at protocol version 15:
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

    /// Before protocol version 15 the whole create trigger runs on DPNS v2: a
    /// missing preorder and a wrong normalized label are its refusals, and it
    /// neither deletes the preorder nor reads the identity record.
    #[tokio::test]
    async fn should_keep_the_create_trigger_at_protocol_version_14() {
        let mut fixture = DpnsFixture::new(14);

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

    /// A chain upgraded from protocol version 14: DPNS is re-stored as v3, every
    /// domain written under v2 decodes unchanged, and the `preorder` type is
    /// rebuilt: every preorder made before the upgrade is gone, a pending one can
    /// no longer be revealed, and the type is laid out exactly as on a chain born
    /// at protocol version 15.
    #[tokio::test]
    async fn should_decode_v2_domains_and_rebuild_the_preorders_at_the_upgrade() {
        let mut fixture = DpnsFixture::new(14);

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
        assert_eq!(v2_domain.id(), registered.id());
        assert!(fixture.stored_preorder(&pending).is_some());
        assert!(fixture
            .dpns()
            .document_type_for_name("domain")
            .expect("expected the domain document type")
            .documents_can_be_deleted());

        fixture.activate_protocol_version_15(20);

        let v3_dpns = fixture.dpns();
        assert!(!v3_dpns
            .document_type_for_name("domain")
            .expect("expected the domain document type")
            .documents_can_be_deleted());
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

        // The rebuilt preorder type holds nothing and is laid out as a chain born at
        // protocol version 15 lays it out
        assert!(fixture.stored_preorder(&pending).is_none());
        // A genesis state is all such a chain needs: no identities
        let born_at_15 = TestPlatformBuilder::new()
            .with_initial_protocol_version(15)
            .build_with_mock_rpc()
            .set_genesis_state();
        assert_eq!(
            preorder_type_layout(&fixture.platform, fixture.platform_version),
            preorder_type_layout(&born_at_15, fixture.platform_version)
        );

        // Alice's pending preorder is gone, so she preorders again
        let (_, result) = fixture
            .domain(Who::Alice, salt, "quantum8", "dash", &[], &[], 21)
            .await;
        assert_matches!(
            refusal(&result),
            ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_))
        );
        let salt: [u8; 32] = fixture.rng.gen();
        let preorder = fixture
            .preorder(Who::Alice, salt, "quantum8", "dash", 22)
            .await;
        assert_eq!(
            fixture
                .stored_preorder(&preorder)
                .expect("expected the preorder")
                .created_at_block_height(),
            Some(22)
        );
        let (_, result) = fixture
            .domain(Who::Alice, salt, "quantum8", "dash", &[], &[], 23)
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert!(fixture.stored_preorder(&preorder).is_none());
    }

    /// A chain at protocol version 12 voting straight for 15: the ladder runs the rungs for
    /// 13, 14 and 15 in one block, each with version 15's tables. The rung for 13 re-stores
    /// DPNS as protocol version 14 still declares it, v2, so the rung for 15 finds the
    /// `preorder` type stored as v1 and v2 declare it and rebuilds it: the preorders made
    /// before are gone and the layout is a chain born at 15's.
    #[tokio::test]
    async fn should_rebuild_the_preorders_on_a_ladder_from_protocol_version_12_to_15() {
        let mut fixture = DpnsFixture::new(12);
        let salt: [u8; 32] = fixture.rng.gen();
        let pending = fixture
            .preorder(Who::Alice, salt, "quantum8", "dash", PREORDER_HEIGHT)
            .await;
        assert!(fixture.stored_preorder(&pending).is_some());

        fixture.activate_protocol_version_15_from(12, 20);

        assert!(fixture
            .dpns()
            .document_type_for_name("preorder")
            .expect("expected the preorder document type")
            .required_fields()
            .contains("$createdAtBlockHeight"));
        assert!(fixture.stored_preorder(&pending).is_none());
        let born_at_15 = TestPlatformBuilder::new()
            .with_initial_protocol_version(15)
            .build_with_mock_rpc()
            .set_genesis_state();
        assert_eq!(
            preorder_type_layout(&fixture.platform, fixture.platform_version),
            preorder_type_layout(&born_at_15, fixture.platform_version)
        );
    }

    /// A chain born at protocol version 15 stores DPNS v3 and still inserts
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

    /// A contested name at protocol version 15: the create that joins the
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

        // The refused create left the preorder, which reveals the name again
        let salt: [u8; 32] = fixture.rng.gen();
        let (_, result) = fixture
            .domain(
                Who::Alice,
                salt,
                "quantum",
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
        assert!(
            fixture.stored_preorder(&preorder).is_none(),
            "joining the contest consumes the preorder"
        );
        assert!(
            fixture.name("quantum").is_none(),
            "the name waits for the contest"
        );
    }

    /// Two contenders open a contest on a name at protocol version 15, each
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
            let name_hash = preorder
                .properties()
                .get("nameHash")
                .expect("expected the preorder's name hash")
                .clone();
            assert!(
                query_dpns(
                    &platform,
                    "preorder",
                    &[
                        (
                            "$ownerId",
                            Value::Identifier(preorder.owner_id().to_buffer())
                        ),
                        ("nameHash", name_hash),
                    ],
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
        fast_forward_to_block(
            &platform,
            block_info.time_ms,
            block_info.height,
            42,
            0,
            false,
        );
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

    /// What a client pays for a name: the preorder and the domain create at protocol
    /// version 14 (create trigger, preorder kept, stored for good) and at 15 (keywords, a
    /// preorder living an hour and priced for that time, counted toward its name's thousand
    /// through the `nameHash` count tree, which its create reads, consumed by the create
    /// without a refund). Printed for the record and pinned.
    #[tokio::test]
    async fn should_charge_a_name_at_protocol_versions_14_and_15() {
        let mut measured = vec![];
        for protocol_version in [14, 15] {
            let mut fixture = DpnsFixture::new(protocol_version);
            let alice = fixture.id(Who::Alice);
            let salt: [u8; 32] = fixture.rng.gen();
            let hashes = dpns_preorder_properties(
                alice,
                &salt,
                "quantum7",
                "dash",
                fixture.platform_version,
            );
            let values: Vec<(&str, Value)> = hashes
                .iter()
                .map(|(property, value)| (property.as_str(), value.clone()))
                .collect();
            let (_, preorder_result) = fixture
                .create(Who::Alice, "preorder", &values, &[], PREORDER_HEIGHT)
                .await;
            let (_, domain_result) = fixture
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
            for (document, result) in [("preorder", preorder_result), ("domain", domain_result)] {
                let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = result
                else {
                    panic!("expected the {document} create to succeed, got {result:?}");
                };
                let refund = fee_result
                    .fee_refunds
                    .calculate_refunds_amount_for_identity(alice)
                    .unwrap_or_default();
                println!(
                    "protocol version {protocol_version} {document}: processing fee {}, \
                     storage fee {}, refund {refund}",
                    fee_result.processing_fee, fee_result.storage_fee
                );
                measured.push((fee_result.processing_fee, fee_result.storage_fee, refund));
            }
        }
        // At 15 each create also reads the DPNS contract through a contracts tree holding the
        // decision proposals contract, one more sibling registered at genesis, which reshapes
        // the tree so the DPNS contract's node is billed more bytes to reach (+95_920 a create).
        assert_eq!(
            measured,
            vec![
                (1_159_720, 31_158_000, 0),
                (1_846_280, 42_687_000, 0),
                (5_591_000, 11_558_570, 0),
                (3_739_640, 42_687_000, 0)
            ]
        );
    }
}
