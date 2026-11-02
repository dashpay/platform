//! A data contract update whose document type schema names `properties` twice,
//! run through block execution. From protocol version 15 registration refuses a
//! key a document type schema repeats, at its root or below it.

use super::*;
use dpp::consensus::basic::BasicError;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::data_contract::errors::DataContractError;
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::document::{Document, DocumentV0};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::platform_value;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use simple_signer::signer::SimpleSigner;

const TYPE_NAME: &str = "t";

fn integer_properties() -> Value {
    platform_value!({
        "a": {"type": "integer", "minimum": 0, "maximum": 100, "position": 0},
        "b": {"type": "string", "maxLength": 20, "position": 1},
    })
}

fn string_properties() -> Value {
    platform_value!({
        "a": {"type": "string", "maxLength": 10, "position": 0},
        "b": {"type": "string", "maxLength": 20, "position": 1},
    })
}

fn schema(properties: Vec<Value>) -> Value {
    let mut entries = vec![
        ("type".into(), "object".into()),
        ("documentsMutable".into(), true.into()),
    ];
    for properties in properties {
        entries.push(("properties".into(), properties));
    }
    entries.push((
        "indices".into(),
        platform_value!([{"name": "byB", "properties": [{"b": "asc"}]}]),
    ));
    entries.push(("additionalProperties".into(), false.into()));
    Value::Map(entries)
}

struct Fixture {
    platform: TempPlatform<MockCoreRPCLike>,
    contract: DataContract,
    identity: Identity,
    signer: SimpleSigner,
    key: IdentityPublicKey,
    nonce: u64,
    version: &'static PlatformVersion,
}

impl Fixture {
    fn new(version: &'static PlatformVersion) -> Self {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(version.protocol_version)
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let (identity, signer, key) = setup_identity(&mut platform, 977, dash_to_credits!(1.0));
        let mut contract =
            get_data_contract_fixture(Some(identity.id()), 0, version.protocol_version)
                .data_contract_owned();
        contract
            .set_document_schema(
                TYPE_NAME,
                schema(vec![integer_properties()]),
                true,
                &mut Vec::new(),
                version,
            )
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
            identity,
            signer,
            key,
            nonce: 1,
            version,
        }
    }

    /// The contract as Drive has it now.
    fn current_contract(&self) -> DataContract {
        self.platform
            .drive
            .get_contract_with_fetch_info(self.contract.id().to_buffer(), false, None, self.version)
            .expect("contract fetch")
            .expect("contract exists")
            .contract
            .clone()
    }

    async fn creation(&mut self, properties: Value) -> StateTransition {
        let document_type = self.contract.document_type_for_name(TYPE_NAME).unwrap();
        let entropy = [self.nonce as u8; 32];
        let mut document = Document::V0(DocumentV0 {
            owner_id: self.identity.id(),
            properties: properties.into_btree_string_map().unwrap(),
            revision: Some(1),
            ..Default::default()
        });
        document
            .set_id_for_creation(document_type, &entropy, self.nonce, self.version)
            .unwrap();
        let transition = BatchTransition::new_document_creation_transition_from_document(
            document,
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
        transition
    }

    /// An update to version 2 naming `properties` twice: first with `a` a
    /// string, then as stored.
    async fn update_repeating_properties(&mut self) -> StateTransition {
        let mut contract = self.contract.clone();
        contract.set_version(2);
        // Placed unvalidated: the transition carries the schema as written, and
        // the platform validates it
        contract
            .set_document_schema(
                TYPE_NAME,
                schema(vec![string_properties(), integer_properties()]),
                false,
                &mut Vec::new(),
                self.version,
            )
            .expect("schema placed");
        let transition = DataContractUpdateTransition::new_from_data_contract(
            contract,
            &self.identity.clone().into_partial_identity_info(),
            self.key.id(),
            self.nonce,
            0,
            &self.signer,
            self.version,
            None,
        )
        .await
        .expect("update transition");
        self.nonce += 1;
        transition
    }

    /// Executes `transition` as a validator executes a received block, and
    /// commits.
    fn process(&self, transition: &StateTransition) -> StateTransitionExecutionResult {
        let raw = transition.serialize_to_bytes().unwrap();
        assert_eq!(
            &StateTransition::deserialize_from_bytes_untrusted_exact_in_version(&raw, self.version)
                .unwrap(),
            transition,
            "the signed wire input keeps both `properties` entries"
        );
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
            .expect("block execution returns per-transition results");
        self.platform
            .drive
            .grove
            .commit_transaction(tx)
            .unwrap()
            .unwrap();
        result.execution_results()[0].clone()
    }

    /// The `a` of every stored document of the type.
    fn stored_values_of_a(&self) -> Vec<Option<Value>> {
        let contract = self.current_contract();
        let query = DriveDocumentQuery::from_sql_expr(
            "select * from t",
            &contract,
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
            .expect("expected the documents to read")
            .documents()
            .iter()
            .map(|document| document.properties().get("a").cloned())
            .collect()
    }
}

#[tokio::test]
async fn should_refuse_a_contract_update_repeating_properties() {
    let mut fixture = Fixture::new(PlatformVersion::latest());

    for a in [5_u64, 0] {
        let create = fixture
            .creation(platform_value!({"a": a, "b": format!("b{a}")}))
            .await;
        assert_matches!(
            fixture.process(&create),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }
    let before = fixture.stored_values_of_a();

    let update = fixture.update_repeating_properties().await;
    assert_matches!(
        fixture.process(&update),
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::InvalidContractStructure(ref message)
            )),
            ..
        } if message.contains("declares `properties` more than once")
    );

    assert_eq!(fixture.current_contract().version(), 1);
    assert_eq!(
        fixture.stored_values_of_a(),
        before,
        "the stored documents read as they were written"
    );
}
