//! Commit and reveal through a `refersTo` whose `propertyAgreement` holds a
//! function (protocol version 14), through the full ABCI pipeline. The
//! fixture's `preorder` holds a commitment, `saltedDomainHash`, under a unique
//! index, as the DPNS preorder does. Its `domain`'s `preorderSalt` refers to
//! one found through that index, whose `saltedDomainHash` the agreement pins to
//! the `sys.hash.sha256d` of `preorderSalt ++ normalizedLabel ++ "." ++
//! parentDomainName`, byte for byte the hash the DPNS create trigger computes
//! for a name under a parent. The reveal must be the writer's own commitment
//! (`$ownerId` agreement), from an earlier block (`minimumAgeBlocks: 1`), and
//! consumes it. `openClaim` reveals the sha256d of `preorderSalt ++ label`
//! through `ownerRefersTo` and demands nothing more.
//! `renewal` is a mutable type whose `ownerRefersTo` is an `allOf` of such a
//! reveal, which consumes, and of the writer's `membership`, a deletable
//! lookup every replace asks for again. `saltedNote` is a mutable type whose
//! transient salt reveals the sha256d of `preorderSalt ++ label ++ "/" ++
//! secret`, `secret` transient too, and consumes it. `nestedClaim` reveals
//! through a salt inside an object, `meta.salt`. `ticket` reveals, through its
//! transient `secret`, an earlier ticket of its writer, and consumes it.

use super::*;

mod commit_reveal_lookup_tests {
    use super::*;
    use crate::execution::types::execution_operation::ValidationOperation;
    use crate::execution::types::state_transition_execution_context::{
        StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
    };
    use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::DocumentReferenceValidation;
    use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::fetch_document_with_id;
    use crate::execution::validation::state_transition::tests::setup_identity_without_adding_it;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::signature::SignatureError;
    use dpp::data_contract::document_type::{
        DocumentPropertyReferenceTarget, DocumentTypeRef, PropertyReference, ReferenceHolder,
    };
    use dpp::document::Document;
    use dpp::identity::accessors::IdentitySettersV0;
    use dpp::identity::contract_bounds::ContractBounds;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::tests::json_document::json_document_to_json_value;
    use dpp::state_transition::StateTransition;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::util::hash::hash_double;
    use dpp::version::DefaultForPlatformVersion;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    const CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-commit-reveal.json";

    /// The block commitments are made in.
    const COMMIT_HEIGHT: u64 = 10;

    /// The next block, where a commitment made in [`COMMIT_HEIGHT`] is old
    /// enough for the `domain` reveal's `minimumAgeBlocks: 1`.
    const REVEAL_HEIGHT: u64 = COMMIT_HEIGHT + 1;

    /// Block `height`, a second after the one before it.
    fn block(height: u64) -> BlockInfo {
        BlockInfo {
            time_ms: 1_700_000_000_000 + height * 1_000,
            height,
            ..BlockInfo::default()
        }
    }

    /// The committer, and an identity copying what the committer revealed.
    #[derive(Clone, Copy)]
    enum Who {
        Alice,
        Mallory,
    }

    struct Writer {
        identity: Identity,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        /// The identity contract nonce the next transition uses; every
        /// processed transition consumes one, a refused one included.
        next_nonce: IdentityNonce,
    }

    struct CommitRevealFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        contract: DataContract,
        rng: StdRng,
        alice: Writer,
        mallory: Writer,
    }

    impl CommitRevealFixture {
        fn new() -> Self {
            Self::new_with(|_| {})
        }

        /// The fixture with `change` applied to the contract's JSON before it is parsed.
        fn new_with(change: impl FnOnce(&mut serde_json::Value)) -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
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
            let alice = writer(4711);
            let mallory = writer(4712);

            // Parsed with full validation, so the contract-level lookup checks
            // run on the fixture too
            let mut contract_json =
                json_document_to_json_value(CONTRACT_PATH).expect("expected the contract json");
            change(&mut contract_json);
            let contract = DataContract::from_json(contract_json, true, platform_version)
                .expect("expected to parse the contract");
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
                contract,
                rng: StdRng::seed_from_u64(9061),
                alice,
                mallory,
            }
        }

        fn id(&self, who: Who) -> Identifier {
            match who {
                Who::Alice => self.alice.identity.id(),
                Who::Mallory => self.mallory.identity.id(),
            }
        }

        fn process(
            &self,
            transition: &StateTransition,
            height: u64,
        ) -> StateTransitionExecutionResult {
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
                    &block(height),
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

        /// Creates a `type_name` document owned by `who` in block `height`,
        /// with `values` set over the random required ones and `absent`
        /// removed, and returns it with the result.
        async fn create(
            &mut self,
            who: Who,
            type_name: &str,
            values: &[(&str, Value)],
            absent: &[&str],
            height: u64,
        ) -> (Document, StateTransitionExecutionResult) {
            let platform_version = PlatformVersion::latest();
            let owner_id = self.id(who);
            let document_type = self
                .contract
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
            let writer = match who {
                Who::Alice => &mut self.alice,
                Who::Mallory => &mut self.mallory,
            };
            let nonce = writer.next_nonce;
            writer.next_nonce += 1;
            document
                .set_id_for_creation(document_type, &entropy.0, nonce, platform_version)
                .expect("expected to set the document id");
            let transition = BatchTransition::new_document_creation_transition_from_document(
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
            let result = self.process(&transition, height);
            (document, result)
        }

        /// A preorder by `who` in block `height` committing to `salt ++ name`.
        async fn commit(&mut self, who: Who, salt: [u8; 32], name: &str, height: u64) -> Document {
            let (preorder, result) = self
                .create(
                    who,
                    "preorder",
                    &[("saltedDomainHash", salted_hash(&salt, name))],
                    &[],
                    height,
                )
                .await;
            assert_matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            );
            preorder
        }

        /// A `domain` by `who` in block `height` with `values`, `absent`
        /// removed.
        async fn reveal_domain(
            &mut self,
            who: Who,
            values: Vec<(&str, Value)>,
            absent: &[&str],
            height: u64,
        ) -> StateTransitionExecutionResult {
            self.create(who, "domain", &values, absent, height).await.1
        }

        /// Replaces `document`, as last accepted, with `change` applied, as
        /// `who`, its owner, in block `height`.
        async fn replace(
            &mut self,
            who: Who,
            type_name: &str,
            document: &Document,
            change: impl FnOnce(&mut Document),
            height: u64,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut replacement = document.clone();
            replacement
                .increment_revision()
                .expect("the revision increments");
            change(&mut replacement);
            let document_type = self
                .contract
                .document_type_for_name(type_name)
                .expect("expected the document type");
            let writer = match who {
                Who::Alice => &mut self.alice,
                Who::Mallory => &mut self.mallory,
            };
            let nonce = writer.next_nonce;
            writer.next_nonce += 1;
            let transition = BatchTransition::new_document_replacement_transition_from_document(
                replacement,
                document_type,
                &writer.key,
                nonce,
                0,
                None,
                &writer.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the replace transition");
            self.process(&transition, height)
        }

        /// Deletes `document`, a `type_name` document, as `who`, its owner, in block
        /// `height`.
        async fn delete(
            &mut self,
            who: Who,
            type_name: &str,
            document: &Document,
            height: u64,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let document_type = self
                .contract
                .document_type_for_name(type_name)
                .expect("expected the document type");
            let writer = match who {
                Who::Alice => &mut self.alice,
                Who::Mallory => &mut self.mallory,
            };
            let nonce = writer.next_nonce;
            writer.next_nonce += 1;
            let transition = BatchTransition::new_document_deletion_transition_from_document(
                document.clone(),
                document_type,
                &writer.key,
                nonce,
                0,
                None,
                &writer.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the delete transition");
            self.process(&transition, height)
        }

        /// Whether the `preorder` with `id` is in state.
        fn preorder_exists(&self, id: Identifier) -> bool {
            self.document_exists("preorder", id)
        }

        /// Whether the `type_name` document with `id` is in state.
        fn document_exists(&self, type_name: &str, id: Identifier) -> bool {
            let platform_version = PlatformVersion::latest();
            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected an execution context");
            fetch_document_with_id(
                &self.platform.drive,
                &self.contract,
                self.contract
                    .document_type_for_name(type_name)
                    .expect("expected the document type"),
                id,
                &BlockInfo::default().epoch,
                &mut execution_context,
                None,
                platform_version,
            )
            .expect("expected to fetch the document")
            .is_some()
        }
    }

    /// The commitment to `salt ++ name`: its sha256d, as the DPNS create
    /// trigger computes a preorder's `saltedDomainHash`.
    fn salted_hash(salt: &[u8; 32], name: &str) -> Value {
        let mut preimage = salt.to_vec();
        preimage.extend_from_slice(name.as_bytes());
        Value::Bytes32(hash_double(preimage))
    }

    /// A subdomain `label` of `parent`, revealing `salt`.
    fn subdomain<'a>(
        label: &str,
        normalized_label: &str,
        parent: &str,
        salt: [u8; 32],
    ) -> Vec<(&'a str, Value)> {
        vec![
            ("label", label.into()),
            ("normalizedLabel", normalized_label.into()),
            ("parentDomainName", parent.into()),
            ("preorderSalt", Value::Bytes32(salt)),
        ]
    }

    /// The key a `domain` revealing `salt` for `normalized_label` under
    /// `parent` computes: the commitment [`salted_hash`] makes to
    /// `salt ++ normalized_label.parent`.
    fn revealed_key(salt: [u8; 32], normalized_label: &str, parent: &str) -> Identifier {
        let Value::Bytes32(hash) = salted_hash(&salt, &format!("{normalized_label}.{parent}"))
        else {
            unreachable!("a salted hash is 32 bytes");
        };
        Identifier::from(hash)
    }

    /// A paid refusal of the domain salt's `refersTo` with
    /// `ReferencedEntityNotFoundError` (40120): no commitment matches `key`,
    /// what the create reveals.
    fn assert_no_commitment(result: StateTransitionExecutionResult, key: Identifier) {
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                ..
            } if e.path() == "preorderSalt"
                && *e.entity_id() == key
                && matches!(
                    e.entity_type(),
                    DocumentPropertyReferenceTarget::DeletableDocumentLookup { .. }
                ),
            "expected 40120 at preorderSalt"
        );
    }

    #[tokio::test]
    async fn should_create_a_domain_revealing_its_writers_old_enough_commitment_and_consume_it() {
        let mut fixture = CommitRevealFixture::new();
        let alice = fixture.id(Who::Alice);
        let salt = [0x11; 32];
        let preorder = fixture
            .commit(Who::Alice, salt, "al1ce.dash", COMMIT_HEIGHT)
            .await;
        assert!(fixture.preorder_exists(preorder.id()));

        let result = fixture
            .reveal_domain(
                Who::Alice,
                subdomain("Alice", "al1ce", "dash", salt),
                &[],
                REVEAL_HEIGHT,
            )
            .await;

        let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = result else {
            panic!("expected the reveal to be accepted, got {result:?}");
        };
        // The commitment is gone, deleted with the create, and its storage is
        // refunded to its owner, the writer
        assert!(!fixture.preorder_exists(preorder.id()));
        assert!(
            fee_result
                .fee_refunds
                .calculate_refunds_amount_for_identity(alice)
                .is_some_and(|refund| refund > 0),
            "expected the consumed commitment's storage to be refunded to its owner"
        );

        // A second reveal of the same commitment finds nothing
        let result = fixture
            .reveal_domain(
                Who::Alice,
                subdomain("Alice", "al1ce", "dash", salt),
                &[],
                REVEAL_HEIGHT + 1,
            )
            .await;
        assert_no_commitment(result, revealed_key(salt, "al1ce", "dash"));
    }

    /// `canBeDeleted: "onlyWhenConsumed"` on the commitment: its owner can not take it back
    /// with a delete, and the reveal still consumes it.
    #[tokio::test]
    async fn should_consume_a_commitment_its_owner_can_not_delete() {
        let mut fixture = CommitRevealFixture::new_with(|contract| {
            contract["documentSchemas"]["preorder"]["canBeDeleted"] =
                serde_json::json!("onlyWhenConsumed");
        });
        let alice = fixture.id(Who::Alice);
        let salt = [0x15; 32];
        let preorder = fixture
            .commit(Who::Alice, salt, "al1ce.dash", COMMIT_HEIGHT)
            .await;

        let result = fixture
            .delete(Who::Alice, "preorder", &preorder, COMMIT_HEIGHT)
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(
                    BasicError::InvalidDocumentTransitionActionError(_)
                ),
                ..
            }
        );
        assert!(fixture.preorder_exists(preorder.id()));

        let result = fixture
            .reveal_domain(
                Who::Alice,
                subdomain("Alice", "al1ce", "dash", salt),
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = result else {
            panic!("expected the reveal to be accepted, got {result:?}");
        };
        // Consumed as a delete by its owner would be: gone, its storage refunded to its owner
        assert!(!fixture.preorder_exists(preorder.id()));
        assert!(
            fee_result
                .fee_refunds
                .calculate_refunds_amount_for_identity(alice)
                .is_some_and(|refund| refund > 0),
            "expected the consumed commitment's storage to be refunded to its owner"
        );
    }

    /// The same through a contested create, whose consumed commitments are deleted beside
    /// the contest's own trees rather than with a plain insert.
    #[tokio::test]
    async fn should_consume_a_commitment_its_owner_can_not_delete_through_a_contested_create() {
        let mut fixture = CommitRevealFixture::new_with(|contract| {
            contract["documentSchemas"]["preorder"]["canBeDeleted"] =
                serde_json::json!("onlyWhenConsumed");
            contract["documentSchemas"]["domain"]["indices"] = serde_json::json!([{
                "name": "byNormalizedLabel",
                "properties": [{ "normalizedLabel": "asc" }],
                "unique": true,
                "contested": {
                    "fieldMatches": [
                        { "field": "normalizedLabel", "regexPattern": "^[a-zA-Z01-]{3,19}$" }
                    ],
                    "resolution": 0
                }
            }]);
        });
        let alice = fixture.id(Who::Alice);
        let salt = [0x16; 32];
        let preorder = fixture
            .commit(Who::Alice, salt, "al1ce.dash", COMMIT_HEIGHT)
            .await;

        let (domain, result) = fixture
            .create(
                Who::Alice,
                "domain",
                &subdomain("Alice", "al1ce", "dash", salt),
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = result else {
            panic!("expected the contested reveal to be accepted, got {result:?}");
        };
        // The domain waits in its contest, not in the type's storage, and the commitment is
        // consumed, its storage refunded to its owner
        assert!(!fixture.document_exists("domain", domain.id()));
        assert!(!fixture.preorder_exists(preorder.id()));
        assert!(
            fee_result
                .fee_refunds
                .calculate_refunds_amount_for_identity(alice)
                .is_some_and(|refund| refund > 0),
            "expected the consumed commitment's storage to be refunded to its owner"
        );
    }

    #[tokio::test]
    async fn should_refuse_a_domain_revealing_no_commitment() {
        let mut fixture = CommitRevealFixture::new();
        fixture
            .commit(Who::Alice, [0x11; 32], "al1ce.dash", COMMIT_HEIGHT)
            .await;

        // Another salt, and another name: neither was committed to
        for (values, key) in [
            (
                subdomain("Alice", "al1ce", "dash", [0x12; 32]),
                revealed_key([0x12; 32], "al1ce", "dash"),
            ),
            (
                subdomain("Bob", "b0b", "dash", [0x11; 32]),
                revealed_key([0x11; 32], "b0b", "dash"),
            ),
        ] {
            let result = fixture
                .reveal_domain(Who::Alice, values, &[], REVEAL_HEIGHT)
                .await;
            assert_no_commitment(result, key);
        }
    }

    #[tokio::test]
    async fn should_refuse_a_domain_revealing_another_identitys_commitment() {
        let mut fixture = CommitRevealFixture::new();
        let salt = [0x11; 32];
        let preorder = fixture
            .commit(Who::Alice, salt, "al1ce.dash", COMMIT_HEIGHT)
            .await;

        // Mallory copies what Alice's create reveals: the commitment exists and
        // is old enough, but it is not Mallory's
        let result = fixture
            .reveal_domain(
                Who::Mallory,
                subdomain("Alice", "al1ce", "dash", salt),
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(e)
                ),
                ..
            } if e.path() == "preorderSalt"
                && e.referring_property() == "$ownerId"
                && e.referenced_property() == "$ownerId"
        );
        // The refused create consumed nothing
        assert!(fixture.preorder_exists(preorder.id()));
    }

    #[tokio::test]
    async fn should_refuse_a_domain_revealing_a_commitment_from_the_same_block() {
        let mut fixture = CommitRevealFixture::new();
        let salt = [0x11; 32];
        let preorder = fixture
            .commit(Who::Alice, salt, "al1ce.dash", COMMIT_HEIGHT)
            .await;

        // In the block of the commitment: a proposer could order a copied reveal
        // right behind its own commitment there
        let result = fixture
            .reveal_domain(
                Who::Alice,
                subdomain("Alice", "al1ce", "dash", salt),
                &[],
                COMMIT_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentRequirementNotMetError(e)
                ),
                ..
            } if *e.document_id() == preorder.id()
                && e.field() == "minimumAgeBlocks"
                && e.required() == "1"
                && e.path() == "preorderSalt"
        );
        assert!(fixture.preorder_exists(preorder.id()));

        // The next block is old enough
        let result = fixture
            .reveal_domain(
                Who::Alice,
                subdomain("Alice", "al1ce", "dash", salt),
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert!(!fixture.preorder_exists(preorder.id()));
    }

    #[tokio::test]
    async fn should_refuse_a_domain_that_cannot_assemble_its_preimage_before_reading_state() {
        let mut fixture = CommitRevealFixture::new();
        let salt = [0x11; 32];
        fixture
            .commit(Who::Alice, salt, "al1ce.dash", COMMIT_HEIGHT)
            .await;

        // No parentDomainName: the key has nothing to hash for it
        let result = fixture
            .reveal_domain(
                Who::Alice,
                subdomain("Alice", "al1ce", "dash", salt),
                &["parentDomainName"],
                REVEAL_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::BasicError(
                    BasicError::DocumentReferencePreimageInvalidError(e)
                ),
                ..
            } if e.path() == "preorderSalt" && e.property() == "parentDomainName"
        );

        // "al.1ce" followed by "." then "dash" would read as "al" + "." +
        // "1ce.dash" as well: a value holding its separator is refused
        let result = fixture
            .reveal_domain(
                Who::Alice,
                subdomain("Al.1ce", "al.1ce", "dash", salt),
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::BasicError(
                    BasicError::DocumentReferencePreimageInvalidError(e)
                ),
                ..
            } if e.property() == "normalizedLabel"
        );
    }

    #[tokio::test]
    async fn should_let_anyone_reveal_at_once_where_the_lookup_demands_nothing_more() {
        let mut fixture = CommitRevealFixture::new();
        let salt = [0x31; 32];
        let preorder = fixture
            .commit(Who::Alice, salt, "claim", COMMIT_HEIGHT)
            .await;

        // `openClaim` declares no `$ownerId` pair, no minimum age and no
        // consume: whoever learns the preimage may reveal it, in the same block
        let claim = vec![
            ("label", "claim".into()),
            ("preorderSalt", Value::Bytes32(salt)),
        ];
        let (_, result) = fixture
            .create(Who::Mallory, "openClaim", &claim, &[], COMMIT_HEIGHT)
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert!(fixture.preorder_exists(preorder.id()));
    }

    /// An expression holding a reveal is judged whole when the document is
    /// created, and consumes only if it holds; on a replace, which the
    /// membership leaf asks for, the reveal holds without a read, though the
    /// commitment it consumed is gone.
    #[tokio::test]
    async fn should_consume_through_an_expression_and_leave_the_reveal_alone_on_replace() {
        let mut fixture = CommitRevealFixture::new();
        let salt = [0x41; 32];
        let renewal = |salt: [u8; 32], note: &str| {
            vec![
                ("label", "renew".into()),
                ("preorderSalt", Value::Bytes32(salt)),
                ("note", note.into()),
            ]
        };

        // Mallory commits but holds no membership: the allOf fails on its second
        // leaf, so the create is refused and consumes nothing
        let mallory_preorder = fixture
            .commit(Who::Mallory, [0x42; 32], "renew", COMMIT_HEIGHT)
            .await;
        let (_, result) = fixture
            .create(
                Who::Mallory,
                "renewal",
                &renewal([0x42; 32], "first"),
                &[],
                COMMIT_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                ..
            } if e.path() == "$ownerId"
        );
        assert!(fixture.preorder_exists(mallory_preorder.id()));

        // Alice holds both: the create consumes her commitment
        let (_, result) = fixture
            .create(
                Who::Alice,
                "membership",
                &[("note", "member".into())],
                &[],
                COMMIT_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let preorder = fixture
            .commit(Who::Alice, salt, "renew", COMMIT_HEIGHT)
            .await;
        let (document, result) = fixture
            .create(
                Who::Alice,
                "renewal",
                &renewal(salt, "first"),
                &[],
                COMMIT_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert!(!fixture.preorder_exists(preorder.id()));

        // A replace re-validates the expression for its membership leaf; the
        // reveal it holds is not asked for again
        let result = fixture
            .replace(
                Who::Alice,
                "renewal",
                &document,
                |renewal| renewal.set("note", "second".into()),
                REVEAL_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// A replace of a document whose salt revealed a commitment leaves the
    /// reveal alone: the commitment it consumed is gone, nothing stored that
    /// the key reads can have changed, and a transient param need not be
    /// carried again, though the salt is.
    #[tokio::test]
    async fn should_consume_through_a_revealed_salt_and_leave_it_alone_on_replace() {
        let mut fixture = CommitRevealFixture::new();
        let salt = [0x61; 32];
        let preorder = fixture
            .commit(Who::Alice, salt, "noted/s3cret", COMMIT_HEIGHT)
            .await;

        let (document, result) = fixture
            .create(
                Who::Alice,
                "saltedNote",
                &[
                    ("label", "noted".into()),
                    ("preorderSalt", Value::Bytes32(salt)),
                    ("secret", "s3cret".into()),
                    ("note", "first".into()),
                ],
                &[],
                COMMIT_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert!(!fixture.preorder_exists(preorder.id()));

        let result = fixture
            .replace(
                Who::Alice,
                "saltedNote",
                &document,
                |note| {
                    note.set("note", "second".into());
                    note.remove("secret");
                },
                REVEAL_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// A salt inside an object is read the way the key reads it: a create whose
    /// object holds `salt` twice is refused before any read, since the key
    /// would hash one value and storage keep the other, and consumes nothing.
    #[tokio::test]
    async fn should_refuse_a_nested_salt_repeated_in_its_object() {
        let mut fixture = CommitRevealFixture::new();
        let committed_salt = [0x71; 32];
        let preorder = fixture
            .commit(Who::Alice, committed_salt, "nested", COMMIT_HEIGHT)
            .await;
        let meta = |salts: &[[u8; 32]]| {
            Value::Map(
                salts
                    .iter()
                    .map(|salt| (Value::Text("salt".to_string()), Value::Bytes32(*salt)))
                    .collect(),
            )
        };

        let (_, result) = fixture
            .create(
                Who::Alice,
                "nestedClaim",
                &[
                    ("label", "nested".into()),
                    ("meta", meta(&[committed_salt, [0x72; 32]])),
                ],
                &[],
                COMMIT_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::BasicError(
                    BasicError::DocumentReferencePreimageInvalidError(e)
                ),
                ..
            } if e.path() == "meta.salt" && e.property() == "meta.salt"
        );
        assert!(fixture.preorder_exists(preorder.id()));

        // The same salt once reveals and consumes the commitment
        let (_, result) = fixture
            .create(
                Who::Alice,
                "nestedClaim",
                &[
                    ("label", "nested".into()),
                    ("meta", meta(&[committed_salt])),
                ],
                &[],
                COMMIT_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert!(!fixture.preorder_exists(preorder.id()));
    }

    /// A branch of an expression that fails consumes nothing, even when another
    /// branch lets the create through: `fallbackClaim` holds through
    /// `anyOf(allOf(<consuming reveal>, <membership>), identity)`, and a writer
    /// without a membership is accepted by the identity branch alone.
    #[tokio::test]
    async fn should_keep_a_commitment_whose_revealing_branch_failed() {
        let mut fixture = CommitRevealFixture::new();
        let salt = [0x51; 32];
        let preorder = fixture
            .commit(Who::Mallory, salt, "fallback", COMMIT_HEIGHT)
            .await;

        let claim = vec![
            ("label", "fallback".into()),
            ("preorderSalt", Value::Bytes32(salt)),
        ];
        let (_, result) = fixture
            .create(Who::Mallory, "fallbackClaim", &claim, &[], COMMIT_HEIGHT)
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        // The reveal held, but its allOf failed on the membership: the
        // commitment stays
        assert!(fixture.preorder_exists(preorder.id()));
    }

    /// The key's hash is billed as the double SHA-256 it is, once, and the
    /// lookup as the document fetch it is; a reveal that consumes reports the
    /// commitment it consumes; the check runs on a create only.
    #[tokio::test]
    async fn should_bill_the_commitment_lookup_and_report_the_consumed_commitment() {
        let mut fixture = CommitRevealFixture::new();
        let platform_version = PlatformVersion::latest();
        let alice = fixture.id(Who::Alice);
        let salt = [0x11; 32];
        let preorder = fixture
            .commit(Who::Alice, salt, "al1ce.dash", COMMIT_HEIGHT)
            .await;

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
        let base = DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
            id: Identifier::from([0xAA; 32]),
            identity_contract_nonce: 1,
            document_type_name: "domain".to_string(),
            data_contract: contract_fetch_info.expect("the contract is in state"),
            token_cost: None,
            shielded_token_payment: None,
            gas_fees_paid_by: GasFeesPaidBy::default(),
            contract_gas_fees_paid_by: GasFeesPaidBy::default(),
            declared_action_fee: None,
        });
        let platform_state = fixture.platform.state.load();
        let platform_ref = PlatformStateRef {
            drive: &fixture.platform.drive,
            state: &platform_state,
            config: &fixture.platform.config,
        };
        let data: BTreeMap<String, Value> = subdomain("Alice", "al1ce", "dash", salt)
            .into_iter()
            .map(|(property, value)| (property.to_string(), value))
            .collect();
        let validate = |changed_fields: Option<&std::collections::BTreeSet<String>>| {
            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected an execution context");
            let mut consumed = Vec::new();
            let result = base
                .validate_document_references(
                    &data,
                    alice,
                    Some(alice),
                    changed_fields,
                    None,
                    &platform_ref,
                    &block(REVEAL_HEIGHT),
                    &mut consumed,
                    None,
                    None,
                    &mut execution_context,
                    platform_version,
                )
                .expect("expected the references to be validated");
            (result, execution_context, consumed)
        };

        let (result, execution_context, consumed) = validate(None);
        assert!(result.is_valid(), "{:?}", result.errors);
        // The key is hashed once, billed by its blocks: 32 salt bytes and
        // "al1ce.dash" pad to one block, plus the second pass over the digest.
        // Then the commitment query
        assert_matches!(
            execution_context.operations_slice(),
            [
                ValidationOperation::DoubleSha256(2),
                ValidationOperation::PrecalculatedOperation(fee),
            ] if fee.processing_fee > 0,
            "the hash and the commitment query are the billed operations"
        );
        assert_matches!(
            consumed.as_slice(),
            [consumed] if consumed.document.document_id == preorder.id()
                && consumed.document.document_type_name == "preorder"
                && consumed.path == "preorderSalt"
        );

        // A replace neither reads nor consumes: the commitment was revealed once
        let changed = std::collections::BTreeSet::from(["label".to_string()]);
        let (result, execution_context, consumed) = validate(Some(&changed));
        assert!(result.is_valid(), "{:?}", result.errors);
        assert!(execution_context.operations_slice().is_empty());
        assert!(consumed.is_empty());
    }

    /// A `ticket` reveals an earlier ticket of its writer, of its own type,
    /// and consumes it. Both sit in the owner's `byOwner` index bucket, which
    /// the create adds to while the consumed ticket leaves it.
    #[tokio::test]
    async fn should_create_a_ticket_that_consumes_an_earlier_ticket_of_its_writer() {
        let mut fixture = CommitRevealFixture::new();
        let first_secret = [0x31; 32];
        let (first, result) = fixture
            .create(
                Who::Alice,
                "ticket",
                &[("commitment", Value::Bytes32(hash_double(first_secret)))],
                &["secret"],
                COMMIT_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let (second, result) = fixture
            .create(
                Who::Alice,
                "ticket",
                &[
                    ("commitment", Value::Bytes32(hash_double([0x32; 32]))),
                    ("secret", Value::Bytes32(first_secret)),
                ],
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "expected the second ticket to consume the first"
        );
        assert!(!fixture.document_exists("ticket", first.id()));
        assert!(fixture.document_exists("ticket", second.id()));
    }

    /// A key bound to the `domain` type signs a domain that reveals and
    /// consumes a `preorder`: the consume deletes a document of a type outside
    /// the key's bounds, so the batch is refused and the commitment stays.
    #[tokio::test]
    async fn should_refuse_a_reveal_consuming_outside_the_signing_keys_bounds() {
        let mut fixture = CommitRevealFixture::new();
        let platform_version = PlatformVersion::latest();
        let (mut identity, mut signer, unbound_key) =
            setup_identity_without_adding_it(4713, dash_to_credits!(0.5));
        let (mut bound_key, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
                2,
                &mut StdRng::seed_from_u64(4714),
                platform_version,
            )
            .expect("expected a key pair");
        let IdentityPublicKey::V0(ref mut key) = bound_key else {
            panic!("expected a version 0 key")
        };
        key.contract_bounds = Some(ContractBounds::SingleContractDocumentType {
            id: fixture.contract.id(),
            document_type_name: "domain".into(),
        });
        signer.add_identity_public_key(bound_key.clone(), private_key);
        identity.add_public_key(bound_key.clone());
        fixture
            .platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the identity");
        fixture.mallory = Writer {
            identity,
            signer,
            key: unbound_key,
            next_nonce: 1,
        };

        let salt = [0x61; 32];
        let preorder = fixture
            .commit(Who::Mallory, salt, "b0und.dash", COMMIT_HEIGHT)
            .await;
        fixture.mallory.key = bound_key;
        let result = fixture
            .reveal_domain(
                Who::Mallory,
                subdomain("B0und", "b0und", "dash", salt),
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::SignatureError(
                    SignatureError::ContractBoundedKeyOutOfBoundsError(e)
                ),
                ..
            } if *e.public_key_id() == 2,
            "expected 20014 for the bound key"
        );
        assert!(fixture.preorder_exists(preorder.id()));
    }

    /// The type in the fixture reads what [`DocumentTypeRef`] reports: the
    /// declaration is the salt's, revealed, and the key is computed.
    #[test]
    fn should_parse_the_fixture_domain_as_revealing_a_computed_key() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(CONTRACT_PATH, true, platform_version)
            .expect("expected to parse the contract");
        let domain: DocumentTypeRef = contract
            .document_type_for_name("domain")
            .expect("expected the domain type");
        let declarations = domain.reference_declarations().collect::<Vec<_>>();
        let [(ReferenceHolder::Property("preorderSalt"), PropertyReference::Revealed(target))] =
            declarations.as_slice()
        else {
            panic!("expected the salt's revealed reference alone");
        };
        let lookup = target
            .as_any_document_reference()
            .and_then(|declaration| declaration.lookup)
            .expect("expected the salt's lookup");
        assert!(lookup.is_checked_on_create_only());
        assert_eq!(lookup.minimum_age_blocks, Some(1));
        assert!(lookup.consume);
    }
}
