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
//! A `post` added by some tests reads its `authorHandle` string into a `findBy`
//! as is, no function, finding a `handle` by its unique `name`.

use super::*;

mod commit_reveal_lookup_tests {
    use dpp::data_contract::document_type::property_constraints::DocumentSystemValues;
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
    use dpp::consensus::codes::ErrorWithCode;
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
    use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
    use dpp::tokens::token_payment_info::TokenPaymentInfo;
    use crate::execution::check_tx::CheckTxLevel;
    use crate::execution::check_tx::CheckTxLevel::{FirstTimeCheck, Recheck};
    use dpp::fee::fee_result::FeeResult;
    use dpp::fee::Credits;
    use dpp::data_contract::document_type::HashFunction;
    use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::v0::choose_first_scan_budget;
    use crate::platform_types::platform::PlatformRef;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use dpp::util::hash::hash_double;
    use dpp::version::DefaultForPlatformVersion;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::ConsumedLookupDocument;
    use dpp::validation::SimpleConsensusValidationResult;
    use dpp::platform_value::string_encoding::Encoding;
    use simple_signer::signer::SimpleSigner;
    use std::collections::{BTreeMap, BTreeSet};

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

    /// The committer, an identity copying what the committer revealed, and a third.
    #[derive(Clone, Copy)]
    enum Who {
        Alice,
        Mallory,
        Carol,
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
        carol: Writer,
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
            let carol = writer(4716);

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
                carol,
            }
        }

        fn id(&self, who: Who) -> Identifier {
            match who {
                Who::Alice => self.alice.identity.id(),
                Who::Mallory => self.mallory.identity.id(),
                Who::Carol => self.carol.identity.id(),
            }
        }

        /// The writer `who` signs with.
        fn writer(&mut self, who: Who) -> &mut Writer {
            match who {
                Who::Alice => &mut self.alice,
                Who::Mallory => &mut self.mallory,
                Who::Carol => &mut self.carol,
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
            let contract = self.contract.clone();
            self.create_in(&contract, who, type_name, values, absent, height)
                .await
        }

        /// [`Self::create`] for a document type of `contract`, applied to the platform.
        async fn create_in(
            &mut self,
            contract: &DataContract,
            who: Who,
            type_name: &str,
            values: &[(&str, Value)],
            absent: &[&str],
            height: u64,
        ) -> (Document, StateTransitionExecutionResult) {
            let (document, transition) = self
                .creation_transition(contract, who, type_name, values, absent, None)
                .await;
            let result = self.process(&transition, height);
            (document, result)
        }

        /// `who`'s create of a `type_name` of `contract` with `values`, `absent` removed,
        /// carrying `token_payment_info`, signed and not processed.
        async fn creation_transition(
            &mut self,
            contract: &DataContract,
            who: Who,
            type_name: &str,
            values: &[(&str, Value)],
            absent: &[&str],
            token_payment_info: Option<TokenPaymentInfo>,
        ) -> (Document, StateTransition) {
            let platform_version = PlatformVersion::latest();
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
            let transition = BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                document_type,
                entropy.0,
                &writer.key,
                nonce,
                0,
                token_payment_info,
                &writer.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the create transition");
            (document, transition)
        }

        /// The codes of the errors check tx refuses `transition` with on its first check,
        /// judged at the block after the platform's last one.
        fn check_tx(&self, transition: &StateTransition) -> Vec<u32> {
            self.check_tx_at(transition, FirstTimeCheck).0
        }

        /// The codes of the errors check tx at `level` refuses `transition` with, and the fee
        /// it estimated when it got that far.
        fn check_tx_at(
            &self,
            transition: &StateTransition,
            level: CheckTxLevel,
        ) -> (Vec<u32>, Option<FeeResult>) {
            let platform_version = PlatformVersion::latest();
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
            let result = self
                .platform
                .check_tx(&raw, level, &platform_ref, platform_version)
                .expect("expected to check the transaction");
            let codes = result.errors.iter().map(|error| error.code()).collect();
            (codes, result.data.and_then(|check| check.fee_result))
        }

        /// Sets `who`'s balance to `credits`.
        fn set_balance(&self, who: Who, credits: Credits) {
            let platform_version = PlatformVersion::latest();
            let id = self.id(who).to_buffer();
            let balance = self
                .platform
                .drive
                .fetch_identity_balance(id, None, platform_version)
                .expect("expected to fetch the balance")
                .expect("expected a balance");
            if balance > credits {
                self.platform
                    .drive
                    .remove_from_identity_balance(
                        id,
                        balance - credits,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                        None,
                    )
                    .expect("expected to lower the balance");
            } else if credits > balance {
                self.platform
                    .drive
                    .add_to_identity_balance(
                        id,
                        credits - balance,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to raise the balance");
            }
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
            let contract = self.contract.clone();
            let document_type = contract
                .document_type_for_name(type_name)
                .expect("expected the document type");
            let writer = self.writer(who);
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
            let contract = self.contract.clone();
            let document_type = contract
                .document_type_for_name(type_name)
                .expect("expected the document type");
            let writer = self.writer(who);
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

        /// The references of a `type_name` document holding `data`, written by `writer` in
        /// block `height` (a create when `changed_fields` is `None`), validated as the document
        /// create and replace state validations do at `platform_version`, with the operations
        /// it billed and the documents it would consume.
        fn validate_references(
            &self,
            type_name: &str,
            data: &BTreeMap<String, Value>,
            writer: Identifier,
            changed_fields: Option<&BTreeSet<String>>,
            height: u64,
            platform_version: &PlatformVersion,
        ) -> (
            SimpleConsensusValidationResult,
            StateTransitionExecutionContext,
            Vec<ConsumedLookupDocument>,
        ) {
            let (_, contract_fetch_info) = self
                .platform
                .drive
                .get_contract_with_fetch_info_and_fee(
                    self.contract.id().to_buffer(),
                    None,
                    false,
                    None,
                    platform_version,
                )
                .expect("expected to fetch the contract");
            let base = DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
                id: Identifier::from([0xAA; 32]),
                identity_contract_nonce: 1,
                document_type_name: type_name.to_string(),
                data_contract: contract_fetch_info.expect("the contract is in state"),
                token_cost: None,
                shielded_token_payment: None,
                gas_fees_paid_by: GasFeesPaidBy::default(),
                contract_gas_fees_paid_by: GasFeesPaidBy::default(),
                declared_action_fee: None,
            });
            let platform_state = self.platform.state.load();
            let platform_ref = PlatformStateRef {
                drive: &self.platform.drive,
                state: &platform_state,
                config: &self.platform.config,
            };
            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected an execution context");
            let mut consumed = Vec::new();
            let result = base
                .validate_document_references(
                    data,
                    writer,
                    Some(writer),
                    &DocumentSystemValues::default(),
                    changed_fields,
                    None,
                    &platform_ref,
                    &block(height),
                    &mut consumed,
                    None,
                    &mut BTreeMap::new(),
                    None,
                    &mut execution_context,
                    platform_version,
                )
                .expect("expected the references to be validated");
            (result, execution_context, consumed)
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
    /// object holds `salt` twice is refused before the lookup reads state, since
    /// the key would hash one value and storage keep the other, and consumes nothing.
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
            &result,
            PaidConsensusError {
                error: error @ ConsensusError::BasicError(BasicError::ValueError(e)),
                ..
            } if error.code() == 10103
                && e.value_error() == "document properties contain a repeated map key"
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

        let data: BTreeMap<String, Value> = subdomain("Alice", "al1ce", "dash", salt)
            .into_iter()
            .map(|(property, value)| (property.to_string(), value))
            .collect();
        let validate = |changed_fields: Option<&BTreeSet<String>>| {
            fixture.validate_references(
                "domain",
                &data,
                alice,
                changed_fields,
                REVEAL_HEIGHT,
                platform_version,
            )
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
        let changed = BTreeSet::from(["label".to_string()]);
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

    /// The fixture with a `voucher`, a deletable document keyed by a unique `serial`, and a
    /// `redemption` that finds one by its serial through a plain `findBy`, no function: the
    /// writer's own (`where $ownerId`), from an earlier block, and consumes it. A redemption
    /// is never replaced, so its lookup is judged on the create alone, as a commitment's is.
    fn voucher_fixture() -> CommitRevealFixture {
        CommitRevealFixture::new_with(|contract| {
            let schemas = &mut contract["documentSchemas"];
            schemas["voucher"] = serde_json::json!({
                "type": "object",
                "documentsMutable": false,
                "canBeDeleted": true,
                "properties": {
                    "serial": { "type": "identifier", "position": 0 }
                },
                "indices": [
                    { "name": "serial", "properties": [{ "serial": "asc" }], "unique": true }
                ],
                "required": ["$createdAtBlockHeight", "serial"],
                "additionalProperties": false
            });
            schemas["redemption"] = serde_json::json!({
                "type": "object",
                "documentsMutable": false,
                "canBeDeleted": false,
                "properties": {
                    "voucherSerial": {
                        "type": "identifier",
                        "position": 0,
                        "refersTo": {
                            "type": "deletableDocument",
                            "documentType": "voucher",
                            "findBy": { "serial": "." },
                            "where": { "$ownerId": "$ownerId" },
                            "constraints": { "minimumAgeBlocks": 1 },
                            "consume": true
                        }
                    }
                },
                "required": ["voucherSerial"],
                "additionalProperties": false
            });
        })
    }

    const VOUCHER_SERIAL: [u8; 32] = [0x5e; 32];

    /// A voucher of `who` with [`VOUCHER_SERIAL`], made in `height`.
    async fn make_voucher(fixture: &mut CommitRevealFixture, who: Who, height: u64) -> Document {
        let (voucher, result) = fixture
            .create(
                who,
                "voucher",
                &[("serial", Value::Identifier(VOUCHER_SERIAL))],
                &[],
                height,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        voucher
    }

    /// The redemption of the voucher with [`VOUCHER_SERIAL`] by `who`, in `height`.
    async fn redeem(
        fixture: &mut CommitRevealFixture,
        who: Who,
        height: u64,
    ) -> (Document, StateTransitionExecutionResult) {
        fixture
            .create(
                who,
                "redemption",
                &[("voucherSerial", Value::Identifier(VOUCHER_SERIAL))],
                &[],
                height,
            )
            .await
    }

    #[tokio::test]
    async fn should_consume_a_document_a_plain_lookup_finds_on_a_type_never_replaced() {
        let mut fixture = voucher_fixture();
        let voucher = make_voucher(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;

        let (redemption, result) = redeem(&mut fixture, Who::Alice, REVEAL_HEIGHT).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert!(!fixture.document_exists("voucher", voucher.id()));
        assert!(fixture.document_exists("redemption", redemption.id()));
    }

    #[tokio::test]
    async fn should_refuse_a_plain_lookup_redemption_in_the_block_of_its_voucher() {
        let mut fixture = voucher_fixture();
        let voucher = make_voucher(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;

        let (_, result) = redeem(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentRequirementNotMetError(e)
                ),
                ..
            } if *e.document_id() == voucher.id()
                && e.field() == "minimumAgeBlocks"
                && e.required() == "1"
                && e.path() == "voucherSerial"
        );
        assert!(fixture.document_exists("voucher", voucher.id()));
    }

    #[tokio::test]
    async fn should_refuse_redeeming_another_identitys_voucher_through_a_plain_lookup() {
        let mut fixture = voucher_fixture();
        let voucher = make_voucher(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;

        let (_, result) = redeem(&mut fixture, Who::Mallory, REVEAL_HEIGHT).await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(e)
                ),
                ..
            } if e.path() == "voucherSerial"
                && e.referring_property() == "$ownerId"
                && e.referenced_property() == "$ownerId"
        );
        assert!(fixture.document_exists("voucher", voucher.id()));
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
        assert!(lookup.hash_key().is_some());
        assert!(lookup.is_checked_on_create_only(true));
        assert_eq!(lookup.minimum_age_blocks, Some(1));
        assert!(lookup.consume);
    }

    /// The fixture with a `handle`, unique by its `name` and `kind` (a
    /// `permanentDocument` handle can not be deleted, a `deletableDocument` one
    /// can), and a mutable `post` whose `authorHandle` string refers to it by that
    /// name, read as is: `findBy: { "name": "." }`, no function.
    fn handle_fixture(kind: &'static str) -> CommitRevealFixture {
        handle_fixture_with(kind, |_, _| {})
    }

    /// [`handle_fixture`] with `change` applied to the `handle` and the `post` schemas.
    fn handle_fixture_with(
        kind: &'static str,
        change: impl FnOnce(&mut serde_json::Value, &mut serde_json::Value),
    ) -> CommitRevealFixture {
        CommitRevealFixture::new_with(|contract| {
            let mut handle = serde_json::json!({
                "type": "object",
                "documentsMutable": false,
                "canBeDeleted": kind == "deletableDocument",
                "properties": {
                    "name": { "type": "string", "maxLength": 63, "position": 0 }
                },
                "indices": [
                    { "name": "name", "properties": [{ "name": "asc" }], "unique": true }
                ],
                "required": ["name"],
                "additionalProperties": false
            });
            let mut post = serde_json::json!({
                "type": "object",
                "documentsMutable": true,
                "canBeDeleted": true,
                "properties": {
                    "authorHandle": {
                        "type": "string",
                        "maxLength": 63,
                        "position": 0,
                        "refersTo": {
                            "type": kind,
                            "documentType": "handle",
                            "findBy": { "name": "." }
                        }
                    },
                    "body": { "type": "string", "maxLength": 100, "position": 1 }
                },
                "required": ["authorHandle", "body"],
                "additionalProperties": false
            });
            change(&mut handle, &mut post);
            let schemas = &mut contract["documentSchemas"];
            schemas["handle"] = handle;
            schemas["post"] = post;
        })
    }

    fn text(value: &str) -> Value {
        Value::Text(value.to_string())
    }

    /// The handle `name` of `who`, made in `height`.
    async fn make_handle(
        fixture: &mut CommitRevealFixture,
        who: Who,
        name: &str,
        height: u64,
    ) -> Document {
        let (handle, result) = fixture
            .create(who, "handle", &[("name", text(name))], &[], height)
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        handle
    }

    /// A post of `who` by `author`, in `height`.
    async fn post(
        fixture: &mut CommitRevealFixture,
        who: Who,
        author: &str,
        height: u64,
    ) -> (Document, StateTransitionExecutionResult) {
        fixture
            .create(
                who,
                "post",
                &[("authorHandle", text(author)), ("body", text("gm"))],
                &[],
                height,
            )
            .await
    }

    /// A paid refusal of the post's `authorHandle` with
    /// `ReferencedEntityNotFoundError` (40120): no handle has the name.
    fn assert_no_handle(result: StateTransitionExecutionResult) {
        assert_not_found(result, "authorHandle");
    }

    /// A paid refusal of the property at `path` with `ReferencedEntityNotFoundError`
    /// (40120): no document holds its value. The value is no id and no hash stands for
    /// it, so the error's id is all zeros.
    fn assert_not_found(result: StateTransitionExecutionResult, path: &str) {
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                ..
            } if e.path() == path
                && *e.entity_id() == Identifier::default()
                && matches!(
                    e.entity_type(),
                    DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. }
                        | DocumentPropertyReferenceTarget::DeletableDocumentLookup { .. }
                ),
            "expected 40120 at {path}"
        );
    }

    fn assert_written(result: StateTransitionExecutionResult) {
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    #[tokio::test]
    async fn should_create_a_document_whose_string_property_finds_a_document_by_that_value() {
        let mut fixture = handle_fixture("permanentDocument");
        make_handle(&mut fixture, Who::Alice, "alice", COMMIT_HEIGHT).await;

        let (_, result) = post(&mut fixture, Who::Mallory, "alice", REVEAL_HEIGHT).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    #[tokio::test]
    async fn should_refuse_a_string_value_no_document_has() {
        let mut fixture = handle_fixture("permanentDocument");
        make_handle(&mut fixture, Who::Alice, "alice", COMMIT_HEIGHT).await;

        // The key is the value as is: a name differing by case finds nothing
        for author in ["bob", "Alice"] {
            let (_, result) = post(&mut fixture, Who::Alice, author, REVEAL_HEIGHT).await;
            assert_no_handle(result);
        }
    }

    /// A value read as is is judged again when a replace changes it, as an
    /// identifier's reference is, unlike a value revealed into a computed key
    #[tokio::test]
    async fn should_re_validate_a_string_value_a_replace_changes() {
        let mut fixture = handle_fixture("permanentDocument");
        make_handle(&mut fixture, Who::Alice, "alice", COMMIT_HEIGHT).await;
        make_handle(&mut fixture, Who::Alice, "bob", COMMIT_HEIGHT).await;
        let (post, result) = post(&mut fixture, Who::Alice, "alice", REVEAL_HEIGHT).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let result = fixture
            .replace(
                Who::Alice,
                "post",
                &post,
                |post| post.set("authorHandle", text("carol")),
                REVEAL_HEIGHT,
            )
            .await;
        assert_no_handle(result);

        let result = fixture
            .replace(
                Who::Alice,
                "post",
                &post,
                |post| post.set("authorHandle", text("bob")),
                REVEAL_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// A `deletableDocument` found by a string value is asked for on every
    /// replace, as one by id is: once its handle is deleted, a replace keeping
    /// the value is refused, and one naming another handle goes through
    #[tokio::test]
    async fn should_re_validate_a_deletable_document_found_by_a_string_value_on_every_replace() {
        let mut fixture = handle_fixture("deletableDocument");
        let alice = make_handle(&mut fixture, Who::Alice, "alice", COMMIT_HEIGHT).await;
        make_handle(&mut fixture, Who::Alice, "bob", COMMIT_HEIGHT).await;
        let (post, result) = post(&mut fixture, Who::Alice, "alice", REVEAL_HEIGHT).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let result = fixture
            .delete(Who::Alice, "handle", &alice, REVEAL_HEIGHT)
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let result = fixture
            .replace(
                Who::Alice,
                "post",
                &post,
                |post| post.set("body", text("still here")),
                REVEAL_HEIGHT,
            )
            .await;
        assert_no_handle(result);

        let result = fixture
            .replace(
                Who::Alice,
                "post",
                &post,
                |post| post.set("authorHandle", text("bob")),
                REVEAL_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// The index finds a document by its value's key, which reads the empty string as
    /// `"\0"`: a name sharing its key with a handle's, but not its value, finds nothing
    #[tokio::test]
    async fn should_refuse_a_string_value_that_only_shares_its_index_key_with_a_handle() {
        let mut fixture = handle_fixture("permanentDocument");
        make_handle(&mut fixture, Who::Alice, "", COMMIT_HEIGHT).await;

        let (_, result) = post(&mut fixture, Who::Alice, "\0", REVEAL_HEIGHT).await;
        assert_no_handle(result);

        let (_, result) = post(&mut fixture, Who::Alice, "", REVEAL_HEIGHT).await;
        assert_written(result);
    }

    /// A value longer than an index key may be (255 bytes, here 130 two-byte
    /// characters) is held by no document: it finds nothing, on a create and on a
    /// replace, as any other value no handle has, rather than failing the transition
    #[tokio::test]
    async fn should_refuse_a_string_value_too_long_for_the_index_as_finding_no_document() {
        let mut fixture = handle_fixture_with("permanentDocument", |_, post| {
            post["properties"]["authorHandle"]["maxLength"] = serde_json::json!(300);
        });
        make_handle(&mut fixture, Who::Alice, "alice", COMMIT_HEIGHT).await;
        let too_long = "é".repeat(130);

        let (_, result) = post(&mut fixture, Who::Alice, &too_long, REVEAL_HEIGHT).await;
        assert_no_handle(result);

        let (post, result) = post(&mut fixture, Who::Alice, "alice", REVEAL_HEIGHT).await;
        assert_written(result);
        let result = fixture
            .replace(
                Who::Alice,
                "post",
                &post,
                |post| post.set("authorHandle", text(&too_long)),
                REVEAL_HEIGHT,
            )
            .await;
        assert_no_handle(result);
    }

    /// A handle unique by its name within a scope: a replace changing only the
    /// post's `scope`, another part of the key, judges the name again
    #[tokio::test]
    async fn should_re_validate_a_string_value_when_another_key_part_changes() {
        let mut fixture = handle_fixture_with("permanentDocument", |handle, post| {
            handle["properties"]["scope"] =
                serde_json::json!({ "type": "string", "maxLength": 63, "position": 1 });
            handle["indices"] = serde_json::json!([{
                "name": "nameInScope",
                "properties": [{ "name": "asc" }, { "scope": "asc" }],
                "unique": true
            }]);
            handle["required"] = serde_json::json!(["name", "scope"]);
            post["properties"]["scope"] =
                serde_json::json!({ "type": "string", "maxLength": 63, "position": 2 });
            post["properties"]["authorHandle"]["refersTo"]["findBy"] =
                serde_json::json!({ "name": ".", "scope": "scope" });
            post["required"] = serde_json::json!(["authorHandle", "body", "scope"]);
        });
        for scope in ["art", "music"] {
            let (_, result) = fixture
                .create(
                    Who::Alice,
                    "handle",
                    &[("name", text("alice")), ("scope", text(scope))],
                    &[],
                    COMMIT_HEIGHT,
                )
                .await;
            assert_written(result);
        }
        let (post, result) = fixture
            .create(
                Who::Alice,
                "post",
                &[
                    ("authorHandle", text("alice")),
                    ("scope", text("art")),
                    ("body", text("gm")),
                ],
                &[],
                REVEAL_HEIGHT,
            )
            .await;
        assert_written(result);

        let result = fixture
            .replace(
                Who::Alice,
                "post",
                &post,
                |post| post.set("scope", text("sports")),
                REVEAL_HEIGHT,
            )
            .await;
        assert_no_handle(result);

        let result = fixture
            .replace(
                Who::Alice,
                "post",
                &post,
                |post| post.set("scope", text("music")),
                REVEAL_HEIGHT,
            )
            .await;
        assert_written(result);
    }

    /// The fixture with a `handle`, never changed, unique by an optional `tag` of up
    /// to 20 bytes, and a mutable `post` whose `authorTag` byte array, of up to 300
    /// bytes, refers to it by that tag, read as is: `findBy: { "tag": "." }`.
    fn tag_fixture() -> CommitRevealFixture {
        CommitRevealFixture::new_with(|contract| {
            let schemas = &mut contract["documentSchemas"];
            schemas["handle"] = serde_json::json!({
                "type": "object",
                "documentsMutable": false,
                "canBeDeleted": false,
                "properties": {
                    "tag": {
                        "type": "array", "byteArray": true, "maxItems": 20, "position": 0
                    }
                },
                "indices": [
                    { "name": "tag", "properties": [{ "tag": "asc" }], "unique": true }
                ],
                "additionalProperties": false
            });
            schemas["post"] = serde_json::json!({
                "type": "object",
                "documentsMutable": true,
                "canBeDeleted": true,
                "properties": {
                    "authorTag": {
                        "type": "array",
                        "byteArray": true,
                        "maxItems": 300,
                        "position": 0,
                        "refersTo": {
                            "type": "permanentDocument",
                            "documentType": "handle",
                            "findBy": { "tag": "." }
                        }
                    }
                },
                "required": ["authorTag"],
                "additionalProperties": false
            });
        })
    }

    /// A handle of Alice's with `tag`, or with none, made in [`COMMIT_HEIGHT`].
    async fn make_tagged_handle(fixture: &mut CommitRevealFixture, tag: Option<Value>) {
        let values = tag.map(|tag| vec![("tag", tag)]).unwrap_or_default();
        let (_, result) = fixture
            .create(Who::Alice, "handle", &values, &[], COMMIT_HEIGHT)
            .await;
        assert_written(result);
    }

    /// A post of Alice's by the handle with `tag`, in [`REVEAL_HEIGHT`].
    async fn tagged_post(
        fixture: &mut CommitRevealFixture,
        tag: Value,
    ) -> (Document, StateTransitionExecutionResult) {
        fixture
            .create(
                Who::Alice,
                "post",
                &[("authorTag", tag)],
                &[],
                REVEAL_HEIGHT,
            )
            .await
    }

    /// A byte array value finds the document holding the same bytes, whichever binary
    /// form the transition carries it in, is refused when no handle has it, and is
    /// judged again when a replace changes it
    #[tokio::test]
    async fn should_find_a_document_by_a_byte_array_value_through_the_write_pipeline() {
        let mut fixture = tag_fixture();
        make_tagged_handle(&mut fixture, Some(Value::Bytes(vec![0x5a; 20]))).await;
        make_tagged_handle(&mut fixture, Some(Value::Bytes(vec![0x5c; 7]))).await;

        let (post, result) = tagged_post(&mut fixture, Value::Bytes(vec![0x5a; 20])).await;
        assert_written(result);
        let (_, result) = tagged_post(&mut fixture, Value::Bytes20([0x5a; 20])).await;
        assert_written(result);
        let (_, result) = tagged_post(&mut fixture, Value::Bytes(vec![0x5c; 7])).await;
        assert_written(result);

        let (_, result) = tagged_post(&mut fixture, Value::Bytes(vec![0x5b; 20])).await;
        assert_not_found(result, "authorTag");

        let result = fixture
            .replace(
                Who::Alice,
                "post",
                &post,
                |post| post.set("authorTag", Value::Bytes(vec![0x5b; 20])),
                REVEAL_HEIGHT,
            )
            .await;
        assert_not_found(result, "authorTag");
        let result = fixture
            .replace(
                Who::Alice,
                "post",
                &post,
                |post| post.set("authorTag", Value::Bytes(vec![0x5c; 7])),
                REVEAL_HEIGHT,
            )
            .await;
        assert_written(result);
    }

    /// An empty byte array has the key of no value at all: it does not find a handle
    /// without a tag
    #[tokio::test]
    async fn should_refuse_an_empty_byte_array_value_as_finding_no_document() {
        let mut fixture = tag_fixture();
        make_tagged_handle(&mut fixture, None).await;

        let (_, result) = tagged_post(&mut fixture, Value::Bytes(vec![])).await;
        assert_not_found(result, "authorTag");
    }

    /// A byte array longer than an index key may be finds nothing, on a create and on
    /// a replace, rather than failing the transition
    #[tokio::test]
    async fn should_refuse_a_byte_array_value_too_long_for_the_index_as_finding_no_document() {
        let mut fixture = tag_fixture();
        make_tagged_handle(&mut fixture, Some(Value::Bytes(vec![0x5a; 20]))).await;

        let (_, result) = tagged_post(&mut fixture, Value::Bytes(vec![0x5a; 256])).await;
        assert_not_found(result, "authorTag");

        let (post, result) = tagged_post(&mut fixture, Value::Bytes(vec![0x5a; 20])).await;
        assert_written(result);
        let result = fixture
            .replace(
                Who::Alice,
                "post",
                &post,
                |post| post.set("authorTag", Value::Bytes(vec![0x5a; 256])),
                REVEAL_HEIGHT,
            )
            .await;
        assert_not_found(result, "authorTag");
    }

    /// Adds to the commit and reveal contract's JSON a name registration whose preorders carry
    /// two hashes: a `namePreorder` holds `ownerNameHash`, the sha256d of its owner's id and
    /// the name, and `nameHash`, the sha256d of the name alone, each unique per owner, the
    /// latter also indexed, not uniquely, with `$createdAtBlockHeight`; it lives an hour
    /// (`ttl`). A transferable `name`, unique by its label and parent when `unique_names`,
    /// reveals through `creatorRefersTo` the writer's own preorder from an earlier block,
    /// consumes it, and must be the first valid one among the preorders of the name
    /// (`chooseFirst`).
    fn add_name_registration(contract: &mut serde_json::Value, unique_names: bool) {
        let hash = |position: u32| {
            serde_json::json!({
                "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                "position": position
            })
        };
        let schemas = &mut contract["documentSchemas"];
        schemas["namePreorder"] = serde_json::json!({
            "type": "object",
            "documentsMutable": false,
            "canBeDeleted": true,
            "ttl": 3600,
            "properties": { "ownerNameHash": hash(0), "nameHash": hash(1) },
            "indices": [
                {
                    "name": "ownerAndNameHash",
                    "properties": [{ "$ownerId": "asc" }, { "ownerNameHash": "asc" }],
                    "unique": true
                },
                {
                    "name": "ownerAndName",
                    "properties": [{ "$ownerId": "asc" }, { "nameHash": "asc" }],
                    "unique": true
                },
                {
                    "name": "nameHashByHeight",
                    "properties": [{ "nameHash": "asc" }, { "$createdAtBlockHeight": "asc" }]
                }
            ],
            "required": ["$createdAt", "$createdAtBlockHeight", "ownerNameHash", "nameHash"],
            "additionalProperties": false
        });
        schemas["name"] = name_schema(
            serde_json::json!({
                "type": "deletableDocument",
                "documentType": "namePreorder",
                "findBy": name_find_by(),
                "where": { "$ownerId": "$ownerId" },
                "constraints": { "minimumAgeBlocks": 1, "chooseFirst": name_choose_first() },
                "consume": true
            }),
            unique_names,
        );
    }

    /// The `findBy` of a name's reveal: the writer's own preorder of the name, by the hash of
    /// the name alone, which every preorder of it holds.
    fn name_find_by() -> serde_json::Value {
        serde_json::json!({
            "$ownerId": "$ownerId",
            "nameHash": {
                "function": "sys.hash.sha256d",
                "params": ["normalizedLabel", { "const": "." }, "normalizedParentDomainName"]
            }
        })
    }

    /// The `chooseFirst` of a name's reveal, ordering its preorders by creation height.
    fn name_choose_first() -> serde_json::Value {
        name_choose_first_by(serde_json::json!(["$createdAtBlockHeight"]))
    }

    /// The `chooseFirst` of a name's reveal, ordering its preorders by `by`: valid when a
    /// preorder holds the hash of its owner's id and the name.
    fn name_choose_first_by(by: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "among": "nameHash",
            "validWhen": {
                "ownerNameHash": {
                    "function": "sys.hash.sha256d",
                    "params": ["$ownerId", "normalizedLabel", { "const": "." }, "normalizedParentDomainName"]
                }
            },
            "by": by
        })
    }

    /// A transferable `name` revealing through `creator_refers_to`, unique by its label and
    /// parent when `unique_names`.
    fn name_schema(creator_refers_to: serde_json::Value, unique_names: bool) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "documentsMutable": false,
            "canBeDeleted": false,
            "transferable": 1,
            "properties": {
                "normalizedLabel": { "type": "string", "minLength": 3, "maxLength": 63, "position": 0 },
                "normalizedParentDomainName": {
                    "type": "string", "minLength": 1, "maxLength": 63, "position": 1
                }
            },
            "indices": [{
                "name": "byName",
                "properties": [{ "normalizedParentDomainName": "asc" }, { "normalizedLabel": "asc" }],
                "unique": unique_names
            }],
            "required": ["normalizedLabel", "normalizedParentDomainName"],
            "creatorRefersTo": creator_refers_to,
            "additionalProperties": false
        })
    }

    /// The fixture with the name registration of [`add_name_registration`].
    fn choose_first_fixture(unique_names: bool) -> CommitRevealFixture {
        CommitRevealFixture::new_with(|contract| add_name_registration(contract, unique_names))
    }

    /// The name the choose-first tests register.
    const NAME_LABEL: &str = "al1ce";
    const NAME_PARENT: &str = "dash";

    /// `normalizedLabel ++ "." ++ normalizedParentDomainName`, the name's bytes.
    fn full_name() -> String {
        format!("{NAME_LABEL}.{NAME_PARENT}")
    }

    /// The name's hash, which every preorder of it holds: the sha256d of the name.
    fn name_hash() -> Value {
        Value::Bytes32(hash_double(full_name().into_bytes()))
    }

    /// `owner`'s hash of the name, which only an identity knowing the name can compute: the
    /// sha256d of its id and the name, the shape [`salted_hash`] hashes.
    fn owner_name_hash(owner: Identifier) -> Value {
        salted_hash(&owner.to_buffer(), &full_name())
    }

    /// A `namePreorder` of the name by `who` in block `height`, holding `owner_name_hash`.
    async fn preorder_name(
        fixture: &mut CommitRevealFixture,
        who: Who,
        owner_name_hash: Value,
        height: u64,
    ) -> Document {
        preorder_name_holding(fixture, who, owner_name_hash, name_hash(), height).await
    }

    /// A `namePreorder` by `who` in block `height` holding `owner_name_hash` and `name_hash`.
    async fn preorder_name_holding(
        fixture: &mut CommitRevealFixture,
        who: Who,
        owner_name_hash: Value,
        name_hash: Value,
        height: u64,
    ) -> Document {
        let (preorder, result) = fixture
            .create(
                who,
                "namePreorder",
                &[("ownerNameHash", owner_name_hash), ("nameHash", name_hash)],
                &[],
                height,
            )
            .await;
        assert_written(result);
        preorder
    }

    /// `who`'s valid preorder of the name, made in block `height`.
    async fn commit_name(fixture: &mut CommitRevealFixture, who: Who, height: u64) -> Document {
        let owner = fixture.id(who);
        preorder_name(fixture, who, owner_name_hash(owner), height).await
    }

    /// The values of a `name` for the name.
    fn name_values() -> Vec<(&'static str, Value)> {
        vec![
            ("normalizedLabel", NAME_LABEL.into()),
            ("normalizedParentDomainName", NAME_PARENT.into()),
        ]
    }

    /// `who`'s `name` for the name, created in block `height`.
    async fn reveal_name(
        fixture: &mut CommitRevealFixture,
        who: Who,
        height: u64,
    ) -> StateTransitionExecutionResult {
        fixture
            .create(who, "name", &name_values(), &[], height)
            .await
            .1
    }

    /// A paid refusal with `ReferencedDocumentPrecededError` (40149): `preorder`, the one the
    /// reveal found, is preceded by `preceding`.
    fn assert_preceded(
        result: StateTransitionExecutionResult,
        preorder: &Document,
        preceding: &Document,
    ) {
        let PaidConsensusError { error, .. } = result else {
            panic!("expected a paid refusal, got {result:?}");
        };
        assert_eq!(error.code(), 40149);
        assert_matches!(
            error,
            ConsensusError::StateError(StateError::ReferencedDocumentPrecededError(e))
                if *e.document_id() == preorder.id()
                    && *e.preceding_document_id() == preceding.id()
                    && e.property() == "nameHash"
                    && e.path() == "$creatorId",
            "expected 40149 naming the preceding preorder"
        );
    }

    /// A paid refusal of the name's reveal with `ReferencedEntityNotFoundError` (40120).
    fn assert_no_name_commitment(result: StateTransitionExecutionResult) {
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                ..
            } if e.path() == "$creatorId",
            "expected 40120 at $creatorId"
        );
    }

    /// The type in the fixture parses `chooseFirst` in the constraints beside `findBy`.
    #[test]
    fn should_parse_the_fixture_name_as_choose_first_its_preorders() {
        let fixture = choose_first_fixture(true);
        let name: DocumentTypeRef = fixture
            .contract
            .document_type_for_name("name")
            .expect("expected the name type");
        let declarations = name.reference_declarations().collect::<Vec<_>>();
        let [(ReferenceHolder::Creator, PropertyReference::Value(target))] =
            declarations.as_slice()
        else {
            panic!("expected the creator's reference alone");
        };
        let lookup = target
            .as_any_document_reference()
            .and_then(|declaration| declaration.lookup)
            .expect("expected the creator's lookup");
        assert_eq!(
            lookup.choose_first.as_ref().map(|choose_first| (
                choose_first.among.as_str(),
                choose_first.valid_when_property.as_str()
            )),
            Some(("nameHash", "ownerNameHash"))
        );
    }

    /// Mallory reads the name from Alice's pending reveal and lands a preorder of her own
    /// before the reveal: Alice's older valid preorder refuses Mallory's reveal, which
    /// consumes nothing, and Alice's own reveal, the first committer's, goes through.
    #[tokio::test]
    async fn should_refuse_a_reveal_an_older_valid_commitment_of_another_identity_precedes() {
        let mut fixture = choose_first_fixture(true);
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        let mallory = commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 1).await;

        let result = reveal_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 2).await;
        assert_preceded(result, &mallory, &alice);
        assert!(fixture.document_exists("namePreorder", mallory.id()));

        let result = reveal_name(&mut fixture, Who::Alice, COMMIT_HEIGHT + 2).await;
        assert_written(result);
        assert!(!fixture.document_exists("namePreorder", alice.id()));
    }

    /// Only an older competitor counts: Alice reveals while Mallory's later preorder of the
    /// same name waits.
    #[tokio::test]
    async fn should_create_the_first_committers_reveal_beside_a_younger_competitor() {
        let mut fixture = choose_first_fixture(true);
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        let mallory = commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 1).await;

        let result = reveal_name(&mut fixture, Who::Alice, COMMIT_HEIGHT + 2).await;
        assert_written(result);
        assert!(!fixture.document_exists("namePreorder", alice.id()));
        assert!(fixture.document_exists("namePreorder", mallory.id()));
    }

    /// Anyone can copy a pending preorder's name hash without knowing the name, and land it
    /// first. A copy is owned by an identity its `ownerNameHash` does not name, whether it
    /// holds the copied preorder's own value (Mallory's) or garbage (Carol's), so it refuses
    /// nothing.
    #[tokio::test]
    async fn should_not_count_a_copied_name_hash() {
        let mut fixture = choose_first_fixture(true);
        let alice_id = fixture.id(Who::Alice);
        preorder_name(
            &mut fixture,
            Who::Mallory,
            owner_name_hash(alice_id),
            COMMIT_HEIGHT - 2,
        )
        .await;
        preorder_name(
            &mut fixture,
            Who::Carol,
            Value::Bytes32([0xC0; 32]),
            COMMIT_HEIGHT - 1,
        )
        .await;
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;

        let result = reveal_name(&mut fixture, Who::Alice, REVEAL_HEIGHT).await;
        assert_written(result);
        assert!(!fixture.document_exists("namePreorder", alice.id()));
    }

    /// Two valid preorders from one block: neither precedes the other, so both reveals pass
    /// `chooseFirst`, and the name's unique index takes the first create.
    #[tokio::test]
    async fn should_not_count_a_competitor_from_the_same_block() {
        let mut fixture = choose_first_fixture(true);
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        let mallory = commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT).await;

        let result = reveal_name(&mut fixture, Who::Mallory, REVEAL_HEIGHT).await;
        assert_written(result);
        assert!(!fixture.document_exists("namePreorder", mallory.id()));

        let result = reveal_name(&mut fixture, Who::Alice, REVEAL_HEIGHT).await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::DuplicateUniqueIndexError(_)),
                ..
            },
            "expected the name's unique index to refuse the second create"
        );
        assert!(fixture.document_exists("namePreorder", alice.id()));
    }

    /// The fixture with a name registration whose `chooseFirst` orders preorders by `by`.
    fn choose_first_fixture_by(by: serde_json::Value) -> CommitRevealFixture {
        CommitRevealFixture::new_with(move |contract| {
            add_name_registration(contract, true);
            contract["documentSchemas"]["name"]["creatorRefersTo"]["constraints"]["chooseFirst"] =
                name_choose_first_by(by);
        })
    }

    /// Ordered by `$createdAtBlockHeight` and then `$id`, two valid preorders from one block
    /// are not tied: the one with the lower id comes first, so the other's reveal is refused
    /// and its own stands.
    #[tokio::test]
    async fn should_choose_the_lower_id_from_one_block_when_ordered_by_id_last() {
        let mut fixture =
            choose_first_fixture_by(serde_json::json!(["$createdAtBlockHeight", "$id"]));
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        let mallory = commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT).await;
        let ((first_who, first), (second_who, second)) = if alice.id() < mallory.id() {
            ((Who::Alice, alice), (Who::Mallory, mallory))
        } else {
            ((Who::Mallory, mallory), (Who::Alice, alice))
        };

        let result = reveal_name(&mut fixture, second_who, REVEAL_HEIGHT).await;
        assert_preceded(result, &second, &first);
        let result = reveal_name(&mut fixture, first_who, REVEAL_HEIGHT).await;
        assert_written(result);
        assert!(!fixture.document_exists("namePreorder", first.id()));
    }

    /// Ordered by `$createdAtBlockHeight` and then `$id`, both descending, the preorders of one
    /// block are read by id the way `by` orders them: with a limit of 1, the higher id's
    /// commitment is the one read, and refuses the lower id's reveal, while the higher id's own
    /// reveal stands.
    #[tokio::test]
    async fn should_read_one_blocks_ids_the_way_by_orders_them_within_the_candidate_limit() {
        let mut fixture = choose_first_fixture_by(serde_json::json!([
            { "$createdAtBlockHeight": "desc" },
            { "$id": "desc" }
        ]));
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        let mallory = commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT).await;
        let ((first_who, first), (second_who, second)) = if alice.id() > mallory.id() {
            ((Who::Alice, alice), (Who::Mallory, mallory))
        } else {
            ((Who::Mallory, mallory), (Who::Alice, alice))
        };
        let data: BTreeMap<String, Value> = name_values()
            .into_iter()
            .map(|(property, value)| (property.to_string(), value))
            .collect();
        let mut one_candidate = PlatformVersion::latest().clone();
        one_candidate.system_limits.max_choose_first_candidates = 1;

        let (result, _, _) = fixture.validate_references(
            "name",
            &data,
            fixture.id(second_who),
            None,
            REVEAL_HEIGHT,
            &one_candidate,
        );
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::ReferencedDocumentPrecededError(e))]
                if *e.document_id() == second.id() && *e.preceding_document_id() == first.id()
        );
        let (result, _, _) = fixture.validate_references(
            "name",
            &data,
            fixture.id(first_who),
            None,
            REVEAL_HEIGHT,
            &one_candidate,
        );
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    /// Ordered through a second index level running the other way, `$createdAtBlockHeight`
    /// ascending and then `$ownerId` descending (over an index of (`nameHash`,
    /// `$createdAtBlockHeight`, `$ownerId`)), the preorders of one block are read by owner the
    /// way `by` orders them: with a limit of 1, the higher owner's commitment is the one read,
    /// and refuses the lower owner's reveal, while the higher owner's own reveal stands.
    #[tokio::test]
    async fn should_read_a_second_index_level_the_way_by_orders_it_within_the_candidate_limit() {
        let mut fixture = CommitRevealFixture::new_with(|contract| {
            add_name_registration(contract, true);
            contract["documentSchemas"]["namePreorder"]["indices"][2]["properties"] = serde_json::json!([
                { "nameHash": "asc" },
                { "$createdAtBlockHeight": "asc" },
                { "$ownerId": "asc" }
            ]);
            contract["documentSchemas"]["name"]["creatorRefersTo"]["constraints"]["chooseFirst"] =
                name_choose_first_by(serde_json::json!([
                    "$createdAtBlockHeight",
                    { "$ownerId": "desc" }
                ]));
        });
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        let mallory = commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT).await;
        let ((first_who, first), (second_who, second)) =
            if fixture.id(Who::Alice) > fixture.id(Who::Mallory) {
                ((Who::Alice, alice), (Who::Mallory, mallory))
            } else {
                ((Who::Mallory, mallory), (Who::Alice, alice))
            };
        let data: BTreeMap<String, Value> = name_values()
            .into_iter()
            .map(|(property, value)| (property.to_string(), value))
            .collect();
        let mut one_candidate = PlatformVersion::latest().clone();
        one_candidate.system_limits.max_choose_first_candidates = 1;

        let (result, _, _) = fixture.validate_references(
            "name",
            &data,
            fixture.id(second_who),
            None,
            REVEAL_HEIGHT,
            &one_candidate,
        );
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::ReferencedDocumentPrecededError(e))]
                if *e.document_id() == second.id() && *e.preceding_document_id() == first.id()
        );
        let (result, _, _) = fixture.validate_references(
            "name",
            &data,
            fixture.id(first_who),
            None,
            REVEAL_HEIGHT,
            &one_candidate,
        );
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    /// Ordered by `$createdAtBlockHeight` descending, the newest valid preorder comes first:
    /// Alice's older one is preceded by Mallory's, whose reveal stands.
    #[tokio::test]
    async fn should_choose_the_newest_when_ordered_descending() {
        let mut fixture =
            choose_first_fixture_by(serde_json::json!([{ "$createdAtBlockHeight": "desc" }]));
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        let mallory = commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 1).await;

        let result = reveal_name(&mut fixture, Who::Alice, COMMIT_HEIGHT + 2).await;
        assert_preceded(result, &alice, &mallory);
        let result = reveal_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 2).await;
        assert_written(result);
    }

    /// A competitor past its `ttl` refuses nothing, though the block-end cleanup has not
    /// deleted it yet. Blocks are a second apart, so Alice's preorder, made in block
    /// `COMMIT_HEIGHT`, expires 3,600 blocks later.
    #[tokio::test]
    async fn should_not_count_an_expired_competitor() {
        let mut fixture = choose_first_fixture(true);
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        let mallory = commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 3_000).await;

        // Up to its expiry it precedes Mallory's
        let result = reveal_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 3_599).await;
        assert_preceded(result, &mallory, &alice);

        // From its expiry on it does not, still in state
        let result = reveal_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 3_600).await;
        assert_written(result);
        assert!(fixture.document_exists("namePreorder", alice.id()));
        assert!(!fixture.document_exists("namePreorder", mallory.id()));
    }

    /// The writer's own commitment past its `ttl` is gone too, whether or not the cleanup has
    /// reached it: its reveal finds no document, as a competitor past it precedes nobody.
    #[tokio::test]
    async fn should_refuse_revealing_a_commitment_past_its_ttl() {
        let mut fixture = choose_first_fixture(true);
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;

        let result = reveal_name(&mut fixture, Who::Alice, COMMIT_HEIGHT + 3_600).await;
        assert_no_name_commitment(result);
        assert!(fixture.document_exists("namePreorder", alice.id()));

        // The block before its expiry it is revealed
        let result = reveal_name(&mut fixture, Who::Alice, COMMIT_HEIGHT + 3_599).await;
        assert_written(result);
    }

    /// The writer's own preorder must be valid too: Mallory copies Alice's pending preorder,
    /// her copy landing first, then learns the name from Alice's reveal. Her copy holds the
    /// name's hash, so `findBy` finds it, but not the hash of her own id and the name, so her
    /// reveal finds no commitment, and her copy precedes nobody: Alice's reveal stands.
    #[tokio::test]
    async fn should_refuse_revealing_from_a_copied_commitment() {
        let mut fixture = choose_first_fixture(true);
        let alice_id = fixture.id(Who::Alice);
        let copy = preorder_name(
            &mut fixture,
            Who::Mallory,
            owner_name_hash(alice_id),
            COMMIT_HEIGHT - 1,
        )
        .await;
        commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;

        let result = reveal_name(&mut fixture, Who::Mallory, REVEAL_HEIGHT).await;
        assert_no_name_commitment(result);
        assert!(fixture.document_exists("namePreorder", copy.id()));

        let result = reveal_name(&mut fixture, Who::Alice, REVEAL_HEIGHT).await;
        assert_written(result);
    }

    /// `findBy` finds the writer's preorder by the name's hash: one holding another name hash
    /// is no commitment to the name.
    #[tokio::test]
    async fn should_refuse_a_reveal_whose_commitment_holds_another_name_hash() {
        let mut fixture = choose_first_fixture(true);
        let alice_id = fixture.id(Who::Alice);
        preorder_name_holding(
            &mut fixture,
            Who::Alice,
            owner_name_hash(alice_id),
            Value::Bytes32(hash_double(b"b0b.dash")),
            COMMIT_HEIGHT,
        )
        .await;

        let result = reveal_name(&mut fixture, Who::Alice, REVEAL_HEIGHT).await;
        assert_no_name_commitment(result);
    }

    /// A reveal consumes its preorder, which then precedes nobody: once Alice reveals, a
    /// later reveal of the same name by Mallory, a join where names are not unique, goes
    /// through.
    #[tokio::test]
    async fn should_not_count_a_consumed_competitor() {
        let mut fixture = choose_first_fixture(false);
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        let mallory = commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 1).await;

        let result = reveal_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 2).await;
        assert_preceded(result, &mallory, &alice);

        let result = reveal_name(&mut fixture, Who::Alice, COMMIT_HEIGHT + 2).await;
        assert_written(result);

        let result = reveal_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 3).await;
        assert_written(result);
        assert!(!fixture.document_exists("namePreorder", mallory.id()));
    }

    /// The writer holds at most one commitment to a name, the one its reveal finds: the
    /// unique (`$ownerId`, `nameHash`) index registration demands refuses a second, so none of
    /// the writer's own documents is ever among the competitors.
    #[tokio::test]
    async fn should_hold_one_commitment_of_a_writer_to_a_name() {
        let mut fixture = choose_first_fixture(true);
        preorder_name(
            &mut fixture,
            Who::Alice,
            Value::Bytes32([0xA0; 32]),
            COMMIT_HEIGHT - 1,
        )
        .await;
        let alice_id = fixture.id(Who::Alice);
        let (_, result) = fixture
            .create(
                Who::Alice,
                "namePreorder",
                &[
                    ("ownerNameHash", owner_name_hash(alice_id)),
                    ("nameHash", name_hash()),
                ],
                &[],
                COMMIT_HEIGHT,
            )
            .await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::DuplicateUniqueIndexError(_)),
                ..
            },
            "expected the writer's second commitment to the name to be refused"
        );
    }

    /// The competitors are read oldest first, at most `max_choose_first_candidates`. When none
    /// of those read is a valid competitor the create stands: here, with a limit of 1, Carol's
    /// copy is the one document read and Alice's valid commitment behind it, in the first
    /// position left out, is not; with a limit of 2 Alice's is the last one read, and refuses,
    /// as at the protocol version 14 limit.
    #[tokio::test]
    async fn should_read_at_most_the_candidate_limit_and_stand_when_none_read_refuses() {
        let mut fixture = choose_first_fixture(true);
        preorder_name(
            &mut fixture,
            Who::Carol,
            Value::Bytes32([0xC0; 32]),
            COMMIT_HEIGHT - 2,
        )
        .await;
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT - 1).await;
        commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT).await;
        let mallory_id = fixture.id(Who::Mallory);
        let data: BTreeMap<String, Value> = name_values()
            .into_iter()
            .map(|(property, value)| (property.to_string(), value))
            .collect();

        let mut one_candidate = PlatformVersion::latest().clone();
        one_candidate.system_limits.max_choose_first_candidates = 1;
        let (result, execution_context, _) = fixture.validate_references(
            "name",
            &data,
            mallory_id,
            None,
            REVEAL_HEIGHT,
            &one_candidate,
        );
        assert!(result.is_valid(), "{:?}", result.errors);
        // Carol's copy is the one competitor hashed
        assert_eq!(
            execution_context
                .operations_slice()
                .iter()
                .filter(|operation| matches!(operation, ValidationOperation::DoubleSha256(_)))
                .count(),
            3
        );

        let mut two_candidates = PlatformVersion::latest().clone();
        two_candidates.system_limits.max_choose_first_candidates = 2;
        for platform_version in [&two_candidates, PlatformVersion::latest()] {
            let (result, execution_context, _) = fixture.validate_references(
                "name",
                &data,
                mallory_id,
                None,
                REVEAL_HEIGHT,
                platform_version,
            );
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::ReferencedDocumentPrecededError(e))]
                    if *e.preceding_document_id() == alice.id()
            );
            // Carol's copy and Alice's commitment are each hashed once
            assert_eq!(
                execution_context
                    .operations_slice()
                    .iter()
                    .filter(|operation| matches!(operation, ValidationOperation::DoubleSha256(_)))
                    .count(),
                4
            );
        }
    }

    /// A document holding the name's hash from a block after an older valid commitment's is
    /// read after it, whatever its id: a copy made once the commitment is in state can not push
    /// it past the candidate limit. With a limit of 1, Alice's commitment is the one read, ahead
    /// of Carol's later copy, and refuses Mallory's reveal.
    #[tokio::test]
    async fn should_read_an_older_valid_commitment_before_a_later_copy() {
        let mut fixture = choose_first_fixture(true);
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT - 2).await;
        preorder_name(
            &mut fixture,
            Who::Carol,
            Value::Bytes32([0xC0; 32]),
            COMMIT_HEIGHT - 1,
        )
        .await;
        commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT).await;
        let data: BTreeMap<String, Value> = name_values()
            .into_iter()
            .map(|(property, value)| (property.to_string(), value))
            .collect();

        let mut one_candidate = PlatformVersion::latest().clone();
        one_candidate.system_limits.max_choose_first_candidates = 1;
        let (result, _, _) = fixture.validate_references(
            "name",
            &data,
            fixture.id(Who::Mallory),
            None,
            REVEAL_HEIGHT,
            &one_candidate,
        );
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::ReferencedDocumentPrecededError(e))]
                if *e.preceding_document_id() == alice.id()
        );
    }

    /// The `findBy` and `validWhen` hashes are billed as the double SHA-256s they are, the
    /// lookup and the competitors as the queries they are, and each competitor's `validWhen`
    /// value once: Carol's copy, then Mallory's valid preorder, which refuses.
    #[tokio::test]
    async fn should_bill_the_competitors_query_and_one_hash_per_competitor() {
        let mut fixture = choose_first_fixture(true);
        let platform_version = PlatformVersion::latest();
        let alice = fixture.id(Who::Alice);
        preorder_name(
            &mut fixture,
            Who::Carol,
            Value::Bytes32([0xC0; 32]),
            COMMIT_HEIGHT - 2,
        )
        .await;
        let mallory = commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT - 1).await;
        let alice_preorder = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        let data: BTreeMap<String, Value> = name_values()
            .into_iter()
            .map(|(property, value)| (property.to_string(), value))
            .collect();

        let (result, execution_context, consumed) = fixture.validate_references(
            "name",
            &data,
            alice,
            None,
            REVEAL_HEIGHT,
            platform_version,
        );

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::ReferencedDocumentPrecededError(e))]
                if *e.document_id() == alice_preorder.id()
                    && *e.preceding_document_id() == mallory.id()
        );
        // The writer's id and the name (42 bytes) and the name alone (10 bytes) each pad to
        // one block, plus the second pass over the digest
        let [ValidationOperation::DoubleSha256(2), ValidationOperation::PrecalculatedOperation(lookup), ValidationOperation::DoubleSha256(2), ValidationOperation::PrecalculatedOperation(competitors), ValidationOperation::DoubleSha256(2), ValidationOperation::DoubleSha256(2)] =
            execution_context.operations_slice()
        else {
            panic!(
                "the hashes, the lookup and the competitors query are the billed operations, got \
                 {:?}",
                execution_context.operations_slice()
            );
        };
        // The lookup reads the found preorder; the competitors query reads the two preorders
        // of earlier blocks holding the name's hash
        assert_eq!(
            (lookup.processing_fee, competitors.processing_fee),
            (46_340, 106_160),
            "the lookup and the competitors query bill what they read"
        );
        assert!(consumed.is_empty(), "a refused reveal consumes nothing");
    }

    /// The fees of a reveal choosing the first of the preorders of its name, refused and
    /// accepted: the found preorder's `validWhen` hash, the competitors query and each
    /// competitor's `validWhen` hash are processing, on top of what a reveal without
    /// `chooseFirst` costs.
    #[tokio::test]
    async fn should_pin_the_fees_of_a_reveal_choosing_the_first_of_its_preorders() {
        let mut fixture = choose_first_fixture(true);
        commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 1).await;

        let result = reveal_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 2).await;
        let PaidConsensusError { actual_fees, .. } = result else {
            panic!("expected a paid refusal, got {result:?}");
        };
        assert_eq!(
            (actual_fees.processing_fee, actual_fees.storage_fee),
            (639_160, 0),
            "the refused reveal pays the reads and hashes it made"
        );

        let result = reveal_name(&mut fixture, Who::Alice, COMMIT_HEIGHT + 2).await;
        let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = result else {
            panic!("expected the first committer's reveal to be accepted, got {result:?}");
        };
        assert_eq!(
            (fee_result.processing_fee, fee_result.storage_fee),
            (3_574_720, 25_299_000),
            "the accepted reveal"
        );
    }

    /// A signer who cannot pay the fee floor gets past the minimum balance pre-check only by
    /// asking the contract owner to pay their gas, and check tx then validates such a batch
    /// against the state in full, since nobody else could be charged for its failure. The
    /// `name` type offers no sponsorship (registration refuses `chooseFirst` beside an offer), so
    /// the request resolves to the signer, and check tx refuses the reveal for the signer's
    /// balance before any of its state is validated: the competitors are never read for free.
    #[tokio::test]
    async fn should_refuse_an_unfunded_signers_sponsorship_request_before_reading_competitors() {
        const IDENTITY_INSUFFICIENT_BALANCE: u32 = 40210;
        let (mut fixture, _) = preceded_reveal_fixture().await;
        let contract = fixture.contract.clone();

        // Funded, Mallory's reveal is admitted and left to the block, as any other
        let (_, funded) = fixture
            .creation_transition(
                &contract,
                Who::Mallory,
                "name",
                &name_values(),
                &[],
                prefer_contract_owner(),
            )
            .await;
        assert_eq!(fixture.check_tx(&funded), Vec::<u32>::new());

        fixture.set_balance(Who::Mallory, 0);

        // Unfunded, refused for the balance, not by Alice's preorder (40149), which only the
        // competitors read would find
        let (_, unfunded) = fixture
            .creation_transition(
                &contract,
                Who::Mallory,
                "name",
                &name_values(),
                &[],
                prefer_contract_owner(),
            )
            .await;
        assert_eq!(
            fixture.check_tx(&unfunded),
            vec![IDENTITY_INSUFFICIENT_BALANCE]
        );
        // As the same reveal asking nobody to pay is, by the pre-check, before any read
        let (_, unrequested) = fixture
            .creation_transition(&contract, Who::Mallory, "name", &name_values(), &[], None)
            .await;
        assert_eq!(
            fixture.check_tx(&unrequested),
            vec![IDENTITY_INSUFFICIENT_BALANCE]
        );
    }

    /// The token payment info of a create asking the contract owner to pay its gas, which the
    /// `name` type does not offer (registration refuses `chooseFirst` beside an offer).
    fn prefer_contract_owner() -> Option<TokenPaymentInfo> {
        Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
            payment_token_contract_id: None,
            token_contract_position: 0,
            minimum_token_cost: None,
            maximum_token_cost: None,
            gas_fees_paid_by: GasFeesPaidBy::PreferContractOwner,
        }))
    }

    /// Alice's preorder and then Mallory's, the platform's last block the one after Mallory's,
    /// so check tx judges a reveal where Mallory's preorder is old enough and Alice's precedes
    /// it. Returns Alice's preorder.
    async fn preceded_reveal_fixture() -> (CommitRevealFixture, Document) {
        let mut fixture = choose_first_fixture(true);
        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 1).await;
        let last = block(COMMIT_HEIGHT + 1);
        fast_forward_to_block(&fixture.platform, last.time_ms, last.height, 0, 0, false);
        (fixture, alice)
    }

    /// Check tx does not validate the state of an unsponsored reveal, so its fee estimate holds
    /// none of the competitor scan a block runs: it reserves the scan's whole budget on top. A
    /// signer one credit short of the estimate so reserved is kept out of the mempool; one who
    /// holds it is admitted, and in the block the refused reveal is paid for and its nonce
    /// spent, rather than refused unpaid for a scan the signer could not cover.
    #[tokio::test]
    async fn should_reserve_the_competitor_scan_budget_when_admitting_a_reveal() {
        const IDENTITY_INSUFFICIENT_BALANCE: u32 = 40210;
        let (mut fixture, alice) = preceded_reveal_fixture().await;
        let platform_version = PlatformVersion::latest();
        let contract = fixture.contract.clone();
        let (_, reveal) = fixture
            .creation_transition(&contract, Who::Mallory, "name", &name_values(), &[], None)
            .await;

        let (errors, estimated) = fixture.check_tx_at(&reveal, FirstTimeCheck);
        assert_eq!(errors, Vec::<u32>::new());
        let estimated = estimated.expect("expected an estimated fee");
        let budget = choose_first_scan_budget(platform_version);
        assert_eq!(budget, 100_000_000);
        assert!(
            estimated.processing_fee > budget,
            "the estimate holds the scan's budget: {estimated:?}"
        );
        let required = estimated.total_base_fee();

        fixture.set_balance(Who::Mallory, required - 1);
        assert_eq!(
            fixture.check_tx(&reveal),
            vec![IDENTITY_INSUFFICIENT_BALANCE]
        );

        fixture.set_balance(Who::Mallory, required);
        assert_eq!(fixture.check_tx(&reveal), Vec::<u32>::new());
        let result = fixture.process(&reveal, COMMIT_HEIGHT + 2);
        assert_matches!(
            result,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedDocumentPrecededError(e)),
                ..
            } if *e.preceding_document_id() == alice.id()
        );
    }

    /// Check tx reserves the scan's budget on a recheck too, where it validates no state of an
    /// unsponsored reveal either: a signer whose balance falls one credit short of the
    /// rechecked estimate, the budget included, leaves the mempool, though it still covers
    /// the estimate without the budget; one holding it stays.
    #[tokio::test]
    async fn should_reserve_the_competitor_scan_budget_on_recheck() {
        const IDENTITY_INSUFFICIENT_BALANCE: u32 = 40210;
        let (mut fixture, _) = preceded_reveal_fixture().await;
        let contract = fixture.contract.clone();
        let (_, reveal) = fixture
            .creation_transition(&contract, Who::Mallory, "name", &name_values(), &[], None)
            .await;
        assert_eq!(fixture.check_tx(&reveal), Vec::<u32>::new());

        let (errors, estimated) = fixture.check_tx_at(&reveal, Recheck);
        assert_eq!(errors, Vec::<u32>::new());
        let required = estimated
            .expect("expected the recheck to estimate the fee")
            .total_base_fee();
        let budget = choose_first_scan_budget(PlatformVersion::latest());
        assert!(required > budget, "the recheck estimate holds the budget");

        fixture.set_balance(Who::Mallory, required - 1);
        assert_eq!(
            fixture.check_tx_at(&reveal, Recheck).0,
            vec![IDENTITY_INSUFFICIENT_BALANCE]
        );
        fixture.set_balance(Who::Mallory, required);
        assert_eq!(fixture.check_tx_at(&reveal, Recheck).0, Vec::<u32>::new());
    }

    /// A `paddedName`: a `name` whose `findBy` and `chooseFirst` functions both read a
    /// transient `pad` of up to 5,120 bytes, which no stored document keeps.
    fn padded_name_schema() -> serde_json::Value {
        let name_params = serde_json::json!([
            "pad",
            { "const": "|" },
            "normalizedLabel",
            { "const": "." },
            "normalizedParentDomainName"
        ]);
        let mut valid_when_params = name_params.clone();
        valid_when_params
            .as_array_mut()
            .expect("an array")
            .insert(0, serde_json::json!("$ownerId"));
        let mut schema = name_schema(
            serde_json::json!({
                "type": "deletableDocument",
                "documentType": "namePreorder",
                "findBy": {
                    "$ownerId": "$ownerId",
                    "nameHash": { "function": "sys.hash.sha256d", "params": name_params }
                },
                "constraints": {
                    "minimumAgeBlocks": 1,
                    "chooseFirst": {
                        "among": "nameHash",
                        "validWhen": {
                            "ownerNameHash": { "function": "sys.hash.sha256d", "params": valid_when_params }
                        },
                        "by": ["$createdAtBlockHeight"]
                    }
                }
            }),
            false,
        );
        schema["properties"]["pad"] = serde_json::json!({
            "type": "array", "byteArray": true, "maxItems": 5120, "position": 2
        });
        schema["transient"] = serde_json::json!(["pad"]);
        schema
    }

    /// Check tx reserves the two hashes a reveal's `chooseFirst` check computes as their own
    /// values make them: a transient param, which no stored document keeps and so no write
    /// estimate prices, makes the reservation grow by the double SHA-256 blocks it adds to
    /// both preimages, and by nothing else.
    #[tokio::test]
    async fn should_reserve_the_upfront_hashes_by_the_reveals_own_values() {
        let mut fixture = CommitRevealFixture::new_with(|contract| {
            add_name_registration(contract, true);
            contract["documentSchemas"]["paddedName"] = padded_name_schema();
        });
        let platform_version = PlatformVersion::latest();
        let contract = fixture.contract.clone();
        let padded = |pad: usize| {
            let mut values = name_values();
            values.push(("pad", Value::Bytes(vec![0x07; pad])));
            values
        };
        let (_, small) = fixture
            .creation_transition(&contract, Who::Mallory, "paddedName", &padded(1), &[], None)
            .await;
        let (_, large) = fixture
            .creation_transition(
                &contract,
                Who::Mallory,
                "paddedName",
                &padded(5_000),
                &[],
                None,
            )
            .await;
        let fee = |transition: &StateTransition| {
            let (errors, estimated) = fixture.check_tx_at(transition, FirstTimeCheck);
            assert_eq!(errors, Vec::<u32>::new());
            estimated.expect("expected an estimated fee")
        };
        let (small, large) = (fee(&small), fee(&large));

        // The findBy preimage is the writer's id and the chooseFirst one, `pad | label . parent`
        let choose_first_len = |pad: usize| pad + 1 + NAME_LABEL.len() + 1 + NAME_PARENT.len();
        let function = HashFunction::Sha256d;
        let blocks = |pad: usize| {
            u64::from(function.block_count(32 + choose_first_len(pad)))
                + u64::from(function.block_count(choose_first_len(pad)))
        };
        assert_eq!(
            large.processing_fee - small.processing_fee,
            (blocks(5_000) - blocks(1)) * platform_version.fee_version.hashing.sha256_per_block,
        );
        assert_eq!(
            large.storage_fee, small.storage_fee,
            "the pad is not stored"
        );
    }

    /// The competitor scan bills as it runs, up to its budget: past it, it bills what is left
    /// of the budget and nothing more, and reads on. With one credit per competitor the budget
    /// is 1,000 credits, less than the competitors query alone.
    #[tokio::test]
    async fn should_bill_the_competitor_scan_up_to_its_budget() {
        let mut fixture = choose_first_fixture(true);
        preorder_name(
            &mut fixture,
            Who::Carol,
            Value::Bytes32([0xC0; 32]),
            COMMIT_HEIGHT - 2,
        )
        .await;
        let mallory = commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT - 1).await;
        commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        let alice = fixture.id(Who::Alice);
        let data: BTreeMap<String, Value> = name_values()
            .into_iter()
            .map(|(property, value)| (property.to_string(), value))
            .collect();

        let mut tight_budget = PlatformVersion::latest().clone();
        tight_budget
            .fee_version
            .document_reference
            .choose_first_credits_per_candidate = 1;
        let (result, execution_context, _) =
            fixture.validate_references("name", &data, alice, None, REVEAL_HEIGHT, &tight_budget);

        // Billing stops at the budget, the scan does not: Mallory's valid preorder refuses
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::ReferencedDocumentPrecededError(e))]
                if *e.preceding_document_id() == mallory.id()
        );
        let [ValidationOperation::DoubleSha256(2), ValidationOperation::PrecalculatedOperation(_), ValidationOperation::DoubleSha256(2), ValidationOperation::PrecalculatedOperation(scan)] =
            execution_context.operations_slice()
        else {
            panic!(
                "the scan bills one capped operation, got {:?}",
                execution_context.operations_slice()
            );
        };
        assert_eq!(scan.processing_fee, 1_000);
    }

    /// The payer check of a signer who relies on a sponsor runs on every recheck too: a reveal
    /// asking a contract owner who offers nothing to pay its gas, admitted while its signer
    /// could pay, leaves the mempool for its signer's balance once it is spent, before any
    /// competitor is read.
    #[tokio::test]
    async fn should_drop_an_unfunded_signers_sponsorship_request_on_recheck() {
        const IDENTITY_INSUFFICIENT_BALANCE: u32 = 40210;
        let (mut fixture, _) = preceded_reveal_fixture().await;
        let contract = fixture.contract.clone();
        let (_, reveal) = fixture
            .creation_transition(
                &contract,
                Who::Mallory,
                "name",
                &name_values(),
                &[],
                prefer_contract_owner(),
            )
            .await;
        assert_eq!(fixture.check_tx(&reveal), Vec::<u32>::new());
        assert_eq!(fixture.check_tx_at(&reveal, Recheck).0, Vec::<u32>::new());

        fixture.set_balance(Who::Mallory, 0);
        assert_eq!(
            fixture.check_tx_at(&reveal, Recheck).0,
            vec![IDENTITY_INSUFFICIENT_BALANCE]
        );
    }

    /// A `chooseFirst` into a preorder type of another contract reads that contract's
    /// competitors at write time, as one into its own contract does (it can not consume
    /// there, so the preorders stay).
    #[tokio::test]
    async fn should_judge_choose_first_into_another_contracts_preorders() {
        let mut fixture = choose_first_fixture(true);
        let platform_version = PlatformVersion::latest();
        let mut names = serde_json::json!({
            "$formatVersion": "1",
            "id": Identifier::from([0x51; 32]).to_string(Encoding::Base58),
            "ownerId": Identifier::from([0x52; 32]).to_string(Encoding::Base58),
            "version": 1,
            "documentSchemas": {}
        });
        names["documentSchemas"]["name"] = name_schema(
            serde_json::json!({
                "type": "deletableDocument",
                "contractId": fixture.contract.id().to_string(Encoding::Base58),
                "documentType": "namePreorder",
                "findBy": name_find_by(),
                "constraints": { "minimumAgeBlocks": 1, "chooseFirst": name_choose_first() }
            }),
            true,
        );
        let names = DataContract::from_json(names, true, platform_version)
            .expect("expected to parse the names contract");
        fixture
            .platform
            .drive
            .apply_contract(
                &names,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply the names contract");

        let alice = commit_name(&mut fixture, Who::Alice, COMMIT_HEIGHT).await;
        let mallory = commit_name(&mut fixture, Who::Mallory, COMMIT_HEIGHT + 1).await;

        let (_, result) = fixture
            .create_in(
                &names,
                Who::Mallory,
                "name",
                &name_values(),
                &[],
                COMMIT_HEIGHT + 2,
            )
            .await;
        assert_preceded(result, &mallory, &alice);

        let (_, result) = fixture
            .create_in(
                &names,
                Who::Alice,
                "name",
                &name_values(),
                &[],
                COMMIT_HEIGHT + 2,
            )
            .await;
        assert_written(result);
        assert!(fixture.document_exists("namePreorder", alice.id()));
    }

    /// At protocol version 13 the contract parses without its references, `chooseFirst`
    /// included, and document reference validation (generation 0, which that version also
    /// selects) reads and bills nothing for the name: the in-place changes to that generation
    /// are unreachable there.
    #[test]
    fn should_judge_no_choose_first_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 exists");
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut contract_json =
            json_document_to_json_value(CONTRACT_PATH).expect("expected the contract json");
        add_name_registration(&mut contract_json, true);
        let contract = DataContract::from_json(contract_json, false, platform_version)
            .expect("protocol version 13 parses the contract, ignoring refersTo");
        let name = contract
            .document_type_for_name("name")
            .expect("expected the name type");
        assert_eq!(name.reference_declarations().count(), 0);
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

        let (_, contract_fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                contract.id().to_buffer(),
                None,
                false,
                None,
                platform_version,
            )
            .expect("expected to fetch the contract");
        let base = DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
            id: Identifier::from([0xAB; 32]),
            identity_contract_nonce: 1,
            document_type_name: "name".to_string(),
            data_contract: contract_fetch_info.expect("the contract is in state"),
            token_cost: None,
            shielded_token_payment: None,
            gas_fees_paid_by: GasFeesPaidBy::default(),
            contract_gas_fees_paid_by: GasFeesPaidBy::default(),
            declared_action_fee: None,
        });
        let platform_state = platform.state.load();
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
        };
        let data: BTreeMap<String, Value> = name_values()
            .into_iter()
            .map(|(property, value)| (property.to_string(), value))
            .collect();
        let writer = Identifier::from([0x5D; 32]);
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("expected an execution context");
        let result = base
            .validate_document_references(
                &data,
                writer,
                Some(writer),
                &DocumentSystemValues::default(),
                None,
                None,
                &platform_ref,
                &block(REVEAL_HEIGHT),
                &mut Vec::new(),
                None,
                &mut BTreeMap::new(),
                None,
                &mut execution_context,
                platform_version,
            )
            .expect("expected the references to be validated");
        assert!(result.is_valid(), "{:?}", result.errors);
        assert!(execution_context.operations_slice().is_empty());
    }
}
