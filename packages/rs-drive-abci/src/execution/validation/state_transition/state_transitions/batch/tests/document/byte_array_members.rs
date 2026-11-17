//! A document create whose byte array, or identifier, holds a list of integers that are not
//! bytes (`Value::U64`, as a client converting a JavaScript `Uint8Array` writes them). The
//! schema check reads the document's JSON form, where such a list passes; the document
//! serializer reads only a list of bytes. From protocol version 15 document validation
//! refuses it with a `JsonSchemaError` (`byteArray`) naming the member, which check_tx
//! returns; protocol version 14 passes it on to the serializer, which fails with an internal
//! error. In a block the create is refused unpaid at 15 and fails internally at 14: either
//! way a proposer drops it and validators reject a block holding it.

use super::*;
use crate::error::Error;
use crate::execution::check_tx::CheckTxLevel::FirstTimeCheck;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::consensus::basic::BasicError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::document::{Document, DocumentV0};
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::platform_value;
use dpp::prelude::DataContract;
use dpp::state_transition::StateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use simple_signer::signer::SimpleSigner;

/// `JsonSchemaError`'s consensus code.
const JSON_SCHEMA_ERROR: u32 = 10101;

/// The entropy of the `blob` a fixture creates.
const ENTROPY: [u8; 32] = [3; 32];

/// A `blob` type with a plain byte array `data`, an identifier `target` and a transient byte
/// array `pad`, which a write carries and no document stores.
fn blob_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "data": { "type": "array", "byteArray": true, "maxItems": 64, "position": 0 },
            "target": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 1,
            },
            "pad": { "type": "array", "byteArray": true, "maxItems": 64, "position": 2 },
        },
        "transient": ["pad"],
        "additionalProperties": false,
    })
}

struct Fixture {
    platform: TempPlatform<MockCoreRPCLike>,
    contract: DataContract,
    owner: Identifier,
    signer: SimpleSigner,
    key: IdentityPublicKey,
    version: &'static PlatformVersion,
}

impl Fixture {
    fn new(version: &'static PlatformVersion) -> Self {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(version.protocol_version)
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let mut contract =
            get_data_contract_fixture(Some(identity.id()), 0, version.protocol_version)
                .data_contract_owned();
        contract
            .set_document_schema("blob", blob_schema(), true, &mut Vec::new(), version)
            .expect("expected the schema to parse");
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
            .expect("expected the contract to apply");
        Self {
            platform,
            contract,
            owner: identity.id(),
            signer,
            key,
            version,
        }
    }

    /// The `blob` a create holding `properties` makes.
    fn blob(&self, properties: Value) -> Document {
        let mut document = Document::V0(DocumentV0 {
            owner_id: self.owner,
            properties: properties
                .into_btree_string_map()
                .expect("expected a map of properties"),
            revision: Some(1),
            ..Default::default()
        });
        document
            .set_id_for_creation(self.blob_type(), &ENTROPY, 1, self.version)
            .expect("expected an id");
        document
    }

    fn blob_type(&self) -> DocumentTypeRef<'_> {
        self.contract
            .document_type_for_name("blob")
            .expect("expected the document type")
    }

    /// The raw bytes of a signed create of a `blob` holding `properties`.
    async fn creation(&self, properties: Value) -> Vec<u8> {
        let transition = BatchTransition::new_document_creation_transition_from_document(
            self.blob(properties),
            self.blob_type(),
            ENTROPY,
            &self.key,
            1,
            0,
            None,
            &self.signer,
            self.version,
            None,
        )
        .await
        .expect("expected the transition to sign");
        self.raw(transition)
    }

    /// The raw bytes of a signed replace of `blob`, the next write of its owner, holding
    /// `properties`.
    async fn replacement(&self, blob: &Document, properties: Value) -> Vec<u8> {
        let mut replacement = blob.clone();
        replacement
            .increment_revision()
            .expect("expected the revision to increment");
        replacement.set_properties(
            properties
                .into_btree_string_map()
                .expect("expected a map of properties"),
        );
        let transition = BatchTransition::new_document_replacement_transition_from_document(
            replacement,
            self.blob_type(),
            &self.key,
            2,
            0,
            None,
            &self.signer,
            self.version,
            None,
        )
        .await
        .expect("expected the transition to sign");
        self.raw(transition)
    }

    fn raw(&self, transition: StateTransition) -> Vec<u8> {
        let raw = transition
            .serialize_to_bytes()
            .expect("expected the transition to serialize");
        // The members keep their integer type on the wire
        let decoded =
            StateTransition::deserialize_from_bytes_untrusted_exact_in_version(&raw, self.version)
                .expect("expected the transition to decode");
        assert_eq!(decoded, transition);
        raw
    }

    fn check_tx(&self, raw: &[u8]) -> Result<Vec<ConsensusError>, Error> {
        let state = self.platform.state.load();
        let platform_ref = PlatformRef {
            drive: &self.platform.drive,
            state: &state,
            config: &self.platform.config,
            core_rpc: &self.platform.core_rpc,
        };
        self.platform
            .check_tx(raw, FirstTimeCheck, &platform_ref, self.version)
            .map(|result| result.errors)
    }

    /// The result of `raw` in a block, whose writes are dropped.
    fn process(&self, raw: Vec<u8>) -> StateTransitionExecutionResult {
        self.process_keeping(raw, false)
    }

    fn process_keeping(&self, raw: Vec<u8>, keep: bool) -> StateTransitionExecutionResult {
        let tx = self.platform.drive.grove.start_transaction();
        let result = self
            .platform
            .platform
            .process_raw_state_transitions(
                &[raw],
                &self.platform.state.load(),
                &BlockInfo::default(),
                &tx,
                self.version,
                false,
                None,
            )
            .expect("expected the transitions to process");
        if keep {
            self.platform
                .drive
                .grove
                .commit_transaction(tx)
                .unwrap()
                .expect("expected the block to commit");
        }
        result.execution_results()[0].clone()
    }
}

fn u64_members(bytes: &[u8]) -> Value {
    Value::Array(bytes.iter().map(|byte| Value::U64(*byte as u64)).collect())
}

/// The instance path of a single `byteArray` refusal.
fn byte_array_refusal(errors: &[ConsensusError]) -> Option<&str> {
    match errors {
        [ConsensusError::BasicError(BasicError::JsonSchemaError(error))]
            if error.keyword() == "byteArray" =>
        {
            Some(error.instance_path())
        }
        _ => None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn should_refuse_at_check_tx_a_byte_array_of_integers_that_are_not_bytes() {
    let fixture = Fixture::new(PlatformVersion::latest());
    for (properties, path) in [
        (
            platform_value!({ "data": u64_members(&[1, 2, 3]) }),
            "/data/0",
        ),
        (
            platform_value!({ "target": u64_members(&[7; 32]) }),
            "/target/0",
        ),
    ] {
        let raw = fixture.creation(properties.clone()).await;
        let errors = fixture
            .check_tx(&raw)
            .expect("expected check_tx to return a consensus error, not an internal error");
        assert_eq!(byte_array_refusal(&errors), Some(path), "{errors:?}");
        assert_eq!(errors[0].code(), JSON_SCHEMA_ERROR);

        // In a block the refusal is unpaid: nobody is charged, so no proposer gains by
        // adding what check_tx refused, and a block holding it is rejected
        match fixture.process(raw) {
            StateTransitionExecutionResult::UnpaidConsensusError(error) => {
                assert_eq!(byte_array_refusal(&[error]), Some(path))
            }
            other => panic!("expected an unpaid consensus error for {properties:?}, got {other:?}"),
        }
    }
}

/// A replace is held to the same check as a create, and refused unpaid as one is.
#[tokio::test(flavor = "multi_thread")]
async fn should_refuse_unpaid_a_replace_writing_a_byte_array_of_integers_that_are_not_bytes() {
    let fixture = Fixture::new(PlatformVersion::latest());
    let written = platform_value!({ "data": Value::Bytes(vec![1, 2, 3]) });
    let blob = fixture.blob(written.clone());
    assert!(
        matches!(
            fixture.process_keeping(fixture.creation(written).await, true),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        ),
        "expected the create to succeed"
    );

    let raw = fixture
        .replacement(&blob, platform_value!({ "data": u64_members(&[4, 5]) }))
        .await;
    let errors = fixture
        .check_tx(&raw)
        .expect("expected check_tx to return a consensus error, not an internal error");
    assert_eq!(byte_array_refusal(&errors), Some("/data/0"), "{errors:?}");
    match fixture.process(raw) {
        StateTransitionExecutionResult::UnpaidConsensusError(error) => {
            assert_eq!(byte_array_refusal(&[error]), Some("/data/0"))
        }
        other => panic!("expected an unpaid consensus error, got {other:?}"),
    }

    // Written as bytes, the replace is stored
    let raw = fixture
        .replacement(&blob, platform_value!({ "data": Value::Bytes(vec![4, 5]) }))
        .await;
    let errors = fixture.check_tx(&raw).expect("expected check_tx to run");
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        matches!(
            fixture.process(raw),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        ),
        "expected the replace to succeed"
    );
}

/// A transient property is dropped before the document is written, so the serializer never
/// reads it: a create and a replace carrying such a list in one succeed, as at protocol
/// version 14.
#[tokio::test(flavor = "multi_thread")]
async fn should_accept_a_transient_byte_array_of_integers_that_are_not_bytes() {
    for version in [
        PlatformVersion::latest(),
        PlatformVersion::get(14).expect("expected protocol version 14"),
    ] {
        let fixture = Fixture::new(version);
        let written = platform_value!({
            "data": Value::Bytes(vec![1, 2, 3]),
            "pad": u64_members(&[9, 9]),
        });
        let blob = fixture.blob(written.clone());
        let raw = fixture.creation(written).await;
        let errors = fixture.check_tx(&raw).expect("expected check_tx to run");
        assert!(errors.is_empty(), "{errors:?}");
        let execution = fixture.process_keeping(raw, true);
        assert!(
            matches!(
                execution,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            ),
            "expected the create to succeed at {}, got {execution:?}",
            version.protocol_version
        );

        let raw = fixture
            .replacement(
                &blob,
                platform_value!({
                    "data": Value::Bytes(vec![4, 5]),
                    "pad": u64_members(&[8]),
                }),
            )
            .await;
        let errors = fixture.check_tx(&raw).expect("expected check_tx to run");
        assert!(errors.is_empty(), "{errors:?}");
        let execution = fixture.process(raw);
        assert!(
            matches!(
                execution,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            ),
            "expected the replace to succeed at {}, got {execution:?}",
            version.protocol_version
        );
    }
}

/// A member out of byte range is a schema violation, paid as every other one is.
#[tokio::test(flavor = "multi_thread")]
async fn should_keep_charging_for_a_byte_array_member_out_of_byte_range() {
    let fixture = Fixture::new(PlatformVersion::latest());
    let raw = fixture
        .creation(platform_value!({ "data": [Value::U64(300)] }))
        .await;
    let errors = fixture.check_tx(&raw).expect("expected check_tx to run");
    assert!(
        matches!(
            errors.as_slice(),
            [ConsensusError::BasicError(BasicError::JsonSchemaError(error))]
                if error.keyword() == "maximum"
        ),
        "{errors:?}"
    );
    assert!(
        matches!(
            fixture.process(raw),
            StateTransitionExecutionResult::PaidConsensusError { .. }
        ),
        "expected a paid consensus error"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_accept_at_check_tx_a_byte_array_written_as_bytes() {
    let fixture = Fixture::new(PlatformVersion::latest());
    let raw = fixture
        .creation(platform_value!({
            "data": Value::Bytes(vec![1, 2, 3]),
            "target": Value::Identifier([7; 32]),
        }))
        .await;
    let errors = fixture.check_tx(&raw).expect("expected check_tx to run");
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        matches!(
            fixture.process(raw),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        ),
        "expected the create to succeed"
    );
}

/// Protocol version 14 is unchanged: document validation passes the list and serialization
/// fails, an internal error rather than a consensus error.
#[tokio::test(flavor = "multi_thread")]
async fn should_keep_failing_a_byte_array_of_integers_that_are_not_bytes_internally_at_protocol_version_14(
) {
    let fixture = Fixture::new(PlatformVersion::get(14).expect("expected protocol version 14"));
    let raw = fixture
        .creation(platform_value!({ "data": u64_members(&[1, 2, 3]) }))
        .await;
    // check_tx may fail internally; it must not refuse the list by document validation
    if let Ok(errors) = fixture.check_tx(&raw) {
        assert!(
            byte_array_refusal(&errors).is_none()
                && errors.iter().all(|error| error.code() != JSON_SCHEMA_ERROR),
            "protocol version 14 must not refuse it by document validation: {errors:?}"
        );
    }
    assert!(
        matches!(
            fixture.process(raw),
            StateTransitionExecutionResult::InternalError(..)
        ),
        "expected the create to end as an internal error at protocol version 14"
    );
}
