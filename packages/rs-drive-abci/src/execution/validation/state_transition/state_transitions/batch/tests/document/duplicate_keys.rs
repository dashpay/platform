use super::*;
use crate::execution::check_tx::CheckTxLevel::FirstTimeCheck;
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::codes::ErrorWithCode;
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use dpp::document::{Document, DocumentV0};
use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::platform_value;
use dpp::prelude::DataContract;
use dpp::state_transition::StateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use drive::util::object_size_info::{
    DocumentAndContractInfo, DocumentInfo::DocumentRefInfo, OwnedDocumentInfo,
};
use simple_signer::signer::SimpleSigner;

const REPEATED_KEY: u32 = 10103;

fn map(entries: &[(&str, Value)]) -> Value {
    Value::Map(
        entries
            .iter()
            .map(|(key, value)| (Value::Text((*key).into()), value.clone()))
            .collect(),
    )
}

fn nested_schema(index_only: bool) -> Value {
    let mut schema = platform_value!({
        "type": "object", "documentsMutable": !index_only,
        "properties": {
            "meta": {"type": "object", "position": 0,
                "properties": {"name": {"type": "string", "maxLength": 32, "position": 0}},
                "required": ["name"], "additionalProperties": false}
        },
        "indices": [{"name": "byName", "properties": [{"meta.name": "asc"}], "unique": !index_only}],
        "required": ["meta"], "additionalProperties": false
    });
    if index_only {
        schema.insert("indexOnly".into(), true.into()).unwrap();
    }
    schema
}

fn constraint_schema(encrypted: bool) -> Value {
    let identifier = platform_value!({"type": "array", "byteArray": true, "minItems": 32,
        "maxItems": 32, "contentMediaType": "application/x.dash.dpp.identifier", "position": 0});
    if encrypted {
        platform_value!({
            "type": "object", "documentsMutable": true,
            "properties": {
                "recipientId": identifier,
                "recipientKeyId": {"type": "integer", "minimum": 0, "maximum": 4294967295_u64, "position": 1},
                "senderKeyId": {"type": "integer", "minimum": 0, "maximum": 4294967295_u64, "position": 2},
                "meta": {"type": "object", "position": 3,
                    "properties": {"blob": {"type": "array", "byteArray": true, "minItems": 1, "maxItems": 64,
                        "position": 0, "encryptedFor": {"recipient": "recipientId", "recipientKey": "recipientKeyId",
                            "senderKey": "senderKeyId", "scheme": "ecdh-secp256k1-aes256-cbc"}}},
                    "required": ["blob"], "additionalProperties": false}
            },
            "required": ["recipientId", "recipientKeyId", "senderKeyId", "meta"], "additionalProperties": false
        })
    } else {
        let mut distinct = identifier.clone();
        distinct
            .insert("distinctFrom".into(), "meta.otherId".into())
            .unwrap();
        let mut other = identifier;
        other.insert("position".into(), 1u32.into()).unwrap();
        platform_value!({
            "type": "object", "documentsMutable": true,
            "properties": {"meta": {"type": "object", "position": 0,
                "properties": {"targetId": distinct, "otherId": other},
                "required": ["targetId", "otherId"], "additionalProperties": false}},
            "required": ["meta"], "additionalProperties": false
        })
    }
}

struct Fixture {
    platform: TempPlatform<MockCoreRPCLike>,
    contract: DataContract,
    owner: Identifier,
    signer: SimpleSigner,
    key: IdentityPublicKey,
    nonce: u64,
    version: &'static PlatformVersion,
}

impl Fixture {
    fn new(schema: Value, version: &'static PlatformVersion, credits: Credits) -> Self {
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(version.protocol_version)
            .build_with_mock_rpc()
            .set_initial_state_structure();
        Self::on(platform, schema, version, credits)
    }

    fn on(
        mut platform: TempPlatform<MockCoreRPCLike>,
        schema: Value,
        version: &'static PlatformVersion,
        credits: Credits,
    ) -> Self {
        let (identity, signer, key) = setup_identity(&mut platform, 963, credits);
        let mut contract =
            get_data_contract_fixture(Some(identity.id()), 0, version.protocol_version)
                .data_contract_owned();
        contract
            .set_document_schema("nested", schema, true, &mut Vec::new(), version)
            .expect("schema parses");
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                version,
            )
            .expect("contract applies");
        Self {
            platform,
            contract,
            owner: identity.id(),
            signer,
            key,
            nonce: 1,
            version,
        }
    }

    async fn creation(&mut self, properties: Value) -> (Document, StateTransition) {
        let document_type = self.contract.document_type_for_name("nested").unwrap();
        let entropy = [self.nonce as u8; 32];
        let mut document = Document::V0(DocumentV0 {
            owner_id: self.owner,
            properties: properties.into_btree_string_map().unwrap(),
            revision: Some(1),
            created_at: document_type
                .required_fields()
                .contains("$createdAt")
                .then_some(0),
            ..Default::default()
        });
        document
            .set_id_for_creation(document_type, &entropy, self.nonce, self.version)
            .unwrap();
        let transition = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            document_type,
            entropy,
            &self.key,
            self.nonce,
            0,
            None,
            &self.signer,
            self.version,
            None,
        )
        .await
        .unwrap();
        self.nonce += 1;
        (document, transition)
    }

    async fn replacement(&mut self, document: &Document, properties: Value) -> StateTransition {
        let mut document = document.clone();
        *document.properties_mut() = properties.into_btree_string_map().unwrap();
        document.increment_revision().unwrap();
        let transition = BatchTransition::new_document_replacement_transition_from_document(
            document,
            self.contract.document_type_for_name("nested").unwrap(),
            &self.key,
            self.nonce,
            0,
            None,
            &self.signer,
            self.version,
            None,
        )
        .await
        .unwrap();
        self.nonce += 1;
        transition
    }

    async fn deletion(&mut self, document: &Document) -> StateTransition {
        let transition = BatchTransition::new_document_deletion_transition_from_document(
            document.clone(),
            self.contract.document_type_for_name("nested").unwrap(),
            &self.key,
            self.nonce,
            0,
            None,
            &self.signer,
            self.version,
            None,
        )
        .await
        .unwrap();
        self.nonce += 1;
        transition
    }

    fn raw(&self, transition: &StateTransition) -> Vec<u8> {
        let raw = transition.serialize_to_bytes().unwrap();
        let decoded =
            StateTransition::deserialize_from_bytes_untrusted_exact_in_version(&raw, self.version)
                .unwrap();
        assert_eq!(
            &decoded, transition,
            "signed wire input must retain the nested map entries"
        );
        raw
    }

    fn check_tx(&self, transition: &StateTransition) -> Vec<u32> {
        let state = self.platform.state.load();
        let platform_ref = PlatformRef {
            drive: &self.platform.drive,
            state: &state,
            config: &self.platform.config,
            core_rpc: &self.platform.core_rpc,
        };
        self.platform
            .check_tx(
                &self.raw(transition),
                FirstTimeCheck,
                &platform_ref,
                self.version,
            )
            .unwrap()
            .errors
            .iter()
            .map(|error| error.code())
            .collect()
    }

    fn process(&self, transition: &StateTransition) -> StateTransitionExecutionResult {
        let tx = self.platform.drive.grove.start_transaction();
        let result = self
            .platform
            .platform
            .process_raw_state_transitions(
                &[self.raw(transition)],
                &self.platform.state.load(),
                &BlockInfo::default(),
                &tx,
                self.version,
                false,
                None,
            )
            .unwrap();
        self.platform
            .drive
            .grove
            .commit_transaction(tx)
            .unwrap()
            .unwrap();
        result.execution_results()[0].clone()
    }

    fn balance(&self) -> Credits {
        self.platform
            .drive
            .fetch_identity_balance(self.owner.to_buffer(), None, self.version)
            .unwrap()
            .unwrap()
    }

    fn stored(&self, predicate: Option<&str>) -> Vec<Document> {
        let query = format!(
            "select * from nested{}",
            predicate
                .map(|p| format!(" where `meta.name` = '{p}'"))
                .unwrap_or_default()
        );
        let query = DriveDocumentQuery::from_sql_expr(
            &query,
            &self.contract,
            Some(&self.platform.config.drive),
            self.version,
        )
        .unwrap();
        self.platform
            .drive
            .query_documents(
                query,
                None,
                false,
                None,
                Some(self.version.protocol_version),
            )
            .unwrap()
            .documents()
            .to_vec()
    }

    fn assert_paid(&self, transition: &StateTransition, code: u32) {
        let before = self.balance();
        let result = self.process(transition);
        let PaidConsensusError {
            error, actual_fees, ..
        } = result
        else {
            panic!("expected paid rejection, got {result:?}")
        };
        assert_eq!(error.code(), code);
        let fee = actual_fees.total_base_fee();
        assert!(fee > 0);
        assert_eq!(self.balance(), before - fee);
        let nonce = self
            .platform
            .drive
            .fetch_identity_contract_nonce(
                self.owner.to_buffer(),
                self.contract.id().to_buffer(),
                true,
                None,
                self.version,
            )
            .unwrap()
            .unwrap();
        assert_eq!(nonce & IDENTITY_NONCE_VALUE_FILTER, self.nonce - 1);
    }
}

fn named(first: &str, last: Option<&str>) -> Value {
    let mut entries = vec![("name", first.into())];
    if let Some(last) = last {
        entries.push(("name", last.into()));
    }
    map(&[("meta", map(&entries))])
}

#[tokio::test]
async fn should_reject_ambiguous_create_in_check_tx_and_charge_its_nonce_in_block() {
    let mut fixture = Fixture::new(
        nested_schema(false),
        PlatformVersion::latest(),
        dash_to_credits!(1),
    );
    let (_, transition) = fixture.creation(named("A", Some("B"))).await;
    let mempool_errors = fixture.check_tx(&transition);
    fixture.assert_paid(&transition, REPEATED_KEY);
    assert_eq!(mempool_errors, vec![REPEATED_KEY]);
    assert!(fixture.stored(None).is_empty());
}

#[tokio::test]
async fn should_reject_ambiguous_replace_without_changing_the_row_or_indexes() {
    let mut fixture = Fixture::new(
        nested_schema(false),
        PlatformVersion::latest(),
        dash_to_credits!(1),
    );
    let (document, clean) = fixture.creation(named("original", None)).await;
    assert!(matches!(
        fixture.process(&clean),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    ));
    let before = fixture.stored(None);
    let replace = fixture.replacement(&document, named("A", Some("B"))).await;
    let mempool_errors = fixture.check_tx(&replace);
    fixture.assert_paid(&replace, REPEATED_KEY);
    assert_eq!(mempool_errors, vec![REPEATED_KEY]);
    assert_eq!(fixture.stored(None), before);
    assert_eq!(fixture.stored(Some("original")), before);
    assert!(fixture.stored(Some("A")).is_empty());
    assert!(fixture.stored(Some("B")).is_empty());
}

#[tokio::test]
async fn should_reject_ambiguous_index_only_delete() {
    let mut fixture = Fixture::new(
        nested_schema(true),
        PlatformVersion::latest(),
        dash_to_credits!(1),
    );
    let (mut document, clean) = fixture.creation(named("A", None)).await;
    assert!(matches!(
        fixture.process(&clean),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    ));
    *document.properties_mut() = named("A", Some("B")).into_btree_string_map().unwrap();
    let deletion = fixture.deletion(&document).await;
    let mempool_errors = fixture.check_tx(&deletion);
    fixture.assert_paid(&deletion, REPEATED_KEY);
    assert_eq!(mempool_errors, vec![REPEATED_KEY]);
    *document.properties_mut() = named("A", None).into_btree_string_map().unwrap();
    let clean_delete = fixture.deletion(&document).await;
    assert!(
        matches!(
            fixture.process(&clean_delete),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        ),
        "refused delete must leave the original index-only entry removable"
    );
}

#[tokio::test]
async fn should_reject_ambiguous_index_only_create() {
    let mut fixture = Fixture::new(
        nested_schema(true),
        PlatformVersion::latest(),
        dash_to_credits!(1),
    );
    let (_, creation) = fixture.creation(named("C", Some("D"))).await;
    let mempool_errors = fixture.check_tx(&creation);
    fixture.assert_paid(&creation, REPEATED_KEY);
    assert_eq!(mempool_errors, vec![REPEATED_KEY]);
}

async fn assert_constraint_bypass_rejected(encrypted: bool, replace: bool) {
    let mut fixture = Fixture::new(
        constraint_schema(encrypted),
        PlatformVersion::latest(),
        dash_to_credits!(1),
    );
    let (clean, last, ambiguous, constraint_code) = if encrypted {
        let properties = |meta| {
            map(&[
                ("recipientId", Value::Identifier([7; 32])),
                ("recipientKeyId", 3u32.into()),
                ("senderKeyId", 1u32.into()),
                ("meta", meta),
            ])
        };
        (
            properties(map(&[("blob", Value::Bytes(vec![0xAB; 32]))])),
            properties(map(&[("blob", Value::Bytes(vec![0xAB; 17]))])),
            properties(map(&[
                ("blob", Value::Bytes(vec![0xAB; 32])),
                ("blob", Value::Bytes(vec![0xAB; 17])),
            ])),
            10420,
        )
    } else {
        let id = |byte| Value::Identifier([byte; 32]);
        (
            map(&[("meta", map(&[("targetId", id(1)), ("otherId", id(2))]))]),
            map(&[("meta", map(&[("targetId", id(2)), ("otherId", id(2))]))]),
            map(&[(
                "meta",
                map(&[("targetId", id(1)), ("targetId", id(2)), ("otherId", id(2))]),
            )]),
            10419,
        )
    };
    let (_, invalid) = fixture.creation(last.clone()).await;
    fixture.assert_paid(&invalid, constraint_code);
    let (document, valid) = fixture.creation(clean).await;
    assert!(matches!(
        fixture.process(&valid),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    ));
    let invalid_replace = fixture.replacement(&document, last).await;
    fixture.assert_paid(&invalid_replace, constraint_code);
    let before = fixture.stored(None);
    let transition = if replace {
        fixture.replacement(&document, ambiguous).await
    } else {
        fixture.creation(ambiguous).await.1
    };
    fixture.assert_paid(&transition, REPEATED_KEY);
    assert_eq!(fixture.stored(None), before);
}

#[tokio::test]
async fn should_reject_duplicate_distinct_from_bypass_on_create() {
    assert_constraint_bypass_rejected(false, false).await;
}

#[tokio::test]
async fn should_reject_duplicate_distinct_from_bypass_on_replace() {
    assert_constraint_bypass_rejected(false, true).await;
}

#[tokio::test]
async fn should_reject_duplicate_encrypted_for_bypass_on_create() {
    assert_constraint_bypass_rejected(true, false).await;
}

#[tokio::test]
async fn should_reject_duplicate_encrypted_for_bypass_on_replace() {
    assert_constraint_bypass_rejected(true, true).await;
}

#[tokio::test]
async fn should_keep_prior_version_nested_index_behavior_unchanged() {
    let mut fixture = Fixture::new(
        nested_schema(false),
        PlatformVersion::get(13).unwrap(),
        dash_to_credits!(1),
    );
    let (_, create) = fixture.creation(named("A", Some("B"))).await;
    assert!(fixture.check_tx(&create).is_empty());
    assert!(matches!(
        fixture.process(&create),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    ));
    let stored = fixture.stored(None);
    assert_eq!(
        stored[0].get("meta").unwrap(),
        &map(&[("name", "B".into())])
    );
    assert_eq!(fixture.stored(Some("A")), stored);
    assert!(fixture.stored(Some("B")).is_empty());
    // Serialization has already discarded the ambiguity; current validation cannot
    // infer the index key from the canonical stored row or repair its old index.
    assert!(fixture
        .contract
        .validate_document("nested", &stored[0], PlatformVersion::latest())
        .unwrap()
        .is_valid());
}

#[tokio::test]
async fn should_keep_prior_version_wrong_slot_removal_visible() {
    let mut fixture = Fixture::new(
        nested_schema(false),
        PlatformVersion::get(13).unwrap(),
        dash_to_credits!(1),
    );
    let (_, honest) = fixture.creation(named("B", None)).await;
    assert!(matches!(
        fixture.process(&honest),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    ));
    let (attacker, ambiguous) = fixture.creation(named("A", Some("B"))).await;
    assert!(matches!(
        fixture.process(&ambiguous),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    ));
    let stored_attacker = fixture
        .stored(None)
        .into_iter()
        .find(|row| row.id() == attacker.id())
        .unwrap();
    let deletion = fixture.deletion(&stored_attacker).await;
    let result = fixture.process(&deletion);
    assert!(
        matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        ),
        "{result:?}"
    );
    assert_eq!(
        fixture.stored(None).len(),
        1,
        "the honest primary row remains stored"
    );
    assert!(
        fixture.stored(Some("B")).is_empty(),
        "historical delete removes the honest row's unique reference"
    );
}

#[tokio::test]
async fn should_preserve_an_honest_unique_slot_after_rejecting_an_ambiguous_attacker() {
    let mut fixture = Fixture::new(
        nested_schema(false),
        PlatformVersion::latest(),
        dash_to_credits!(1),
    );
    let (_, honest) = fixture.creation(named("B", None)).await;
    assert!(matches!(
        fixture.process(&honest),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    ));
    let before = fixture.stored(Some("B"));
    let (_, attacker) = fixture.creation(named("A", Some("B"))).await;
    fixture.assert_paid(&attacker, REPEATED_KEY);
    assert_eq!(fixture.stored(Some("B")), before);
    assert_eq!(fixture.stored(None), before);
}

#[tokio::test]
async fn should_keep_unfunded_duplicate_input_unpaid_without_consuming_a_nonce() {
    let mut fixture = Fixture::new(nested_schema(false), PlatformVersion::latest(), 0);
    let (_, transition) = fixture.creation(named("A", Some("B"))).await;
    let result = fixture.process(&transition);
    assert!(
        matches!(
            result,
            StateTransitionExecutionResult::UnpaidConsensusError(ref error) if error.code() == 40210
        ),
        "{result:?}"
    );
    assert_eq!(fixture.balance(), 0);
    assert_eq!(
        fixture
            .platform
            .drive
            .fetch_identity_contract_nonce(
                fixture.owner.to_buffer(),
                fixture.contract.id().to_buffer(),
                true,
                None,
                fixture.version
            )
            .unwrap(),
        None
    );
    assert!(fixture.stored(None).is_empty());
}

#[tokio::test]
async fn should_reject_ambiguous_ttl_creation_and_expire_the_clean_nested_index() {
    let version = PlatformVersion::latest();
    let platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_genesis_state();
    let mut schema = nested_schema(false);
    schema.insert("ttl".into(), 3600u64.into()).unwrap();
    schema
        .insert(
            "required".into(),
            vec![Value::Text("meta".into()), Value::Text("$createdAt".into())].into(),
        )
        .unwrap();
    let mut fixture = Fixture::on(platform, schema, version, dash_to_credits!(1));
    let (_, ambiguous) = fixture.creation(named("A", Some("B"))).await;
    fixture.assert_paid(&ambiguous, REPEATED_KEY);
    let (clean, creation) = fixture.creation(named("clean", None)).await;
    assert!(matches!(
        fixture.process(&creation),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    ));
    let tx = fixture.platform.drive.grove.start_transaction();
    // This is the block-end event that run_block_proposal propagates before fee processing.
    fixture
        .platform
        .platform
        .expire_documents(
            &BlockInfo {
                time_ms: 3_600_000,
                ..Default::default()
            },
            &tx,
            version,
        )
        .expect("clean expiry must not fail after refusing ambiguous input");
    let query = DriveDocumentQuery::from_sql_expr(
        "select * from nested",
        &fixture.contract,
        Some(&fixture.platform.config.drive),
        version,
    )
    .unwrap();
    let remaining = fixture
        .platform
        .drive
        .query_documents(
            query,
            None,
            false,
            Some(&tx),
            Some(version.protocol_version),
        )
        .unwrap();
    assert!(
        remaining.documents().is_empty(),
        "expired clean row {} must be deleted",
        clean.id()
    );
    let query = DriveDocumentQuery::from_sql_expr(
        "select * from nested where `meta.name` = 'clean'",
        &fixture.contract,
        Some(&fixture.platform.config.drive),
        version,
    )
    .unwrap();
    assert!(fixture
        .platform
        .drive
        .query_documents(
            query,
            None,
            false,
            Some(&tx),
            Some(version.protocol_version)
        )
        .unwrap()
        .documents()
        .is_empty());
}

#[tokio::test]
async fn should_keep_preexisting_ambiguous_ttl_expiry_failure_visible() {
    let version = PlatformVersion::latest();
    let mut schema = nested_schema(false);
    schema.insert("ttl".into(), 3600u64.into()).unwrap();
    schema
        .insert(
            "required".into(),
            vec![Value::Text("meta".into()), Value::Text("$createdAt".into())].into(),
        )
        .unwrap();
    for repeated in [false, true] {
        let mut fixture = Fixture::new(schema.clone(), version, dash_to_credits!(1));
        let (document, _) = fixture.creation(named("A", repeated.then_some("B"))).await;
        // Seed state through storage to represent data admitted before the acceptance guard.
        fixture
            .platform
            .drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &fixture.contract,
                    document_type: fixture.contract.document_type_for_name("nested").unwrap(),
                },
                false,
                BlockInfo::default(),
                true,
                None,
                version,
                None,
            )
            .unwrap();
        let stored = fixture.stored(None);
        assert_eq!(
            stored[0].get("meta").unwrap(),
            &map(&[("name", if repeated { "B" } else { "A" }.into())])
        );
        assert_eq!(fixture.stored(Some("A")), stored);
        assert!(fixture.stored(Some("B")).is_empty());
        let tx = fixture.platform.drive.grove.start_transaction();
        let result = fixture.platform.platform.expire_documents(
            &BlockInfo {
                time_ms: 3_600_000,
                ..Default::default()
            },
            &tx,
            version,
        );
        if repeated {
            let Err(crate::error::Error::Drive(drive::error::Error::GroveDB(error))) = result
            else {
                panic!("expected failure removing the absent B index slot: {result:?}");
            };
            let drive::grovedb::Error::PathKeyNotFound(message) = error.as_ref() else {
                panic!("expected missing unique index key: {error:?}");
            };
            assert!(
                message.contains("key 0x42 (66 in decimal) not found"),
                "{message}"
            );
            assert!(
                message.ends_with("/0x01/nested/0x6d6574612e6e616d65"),
                "{message}"
            );
        } else {
            result.expect("the same direct-insert harness must expire a clean row");
        }
        let query = DriveDocumentQuery::from_sql_expr(
            "select * from nested",
            &fixture.contract,
            Some(&fixture.platform.config.drive),
            version,
        )
        .unwrap();
        let remaining = fixture
            .platform
            .drive
            .query_documents(
                query,
                None,
                false,
                Some(&tx),
                Some(version.protocol_version),
            )
            .unwrap();
        if repeated {
            assert_eq!(remaining.documents(), &stored);
        } else {
            assert!(remaining.documents().is_empty());
        }
    }
}
