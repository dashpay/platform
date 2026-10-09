//! End-to-end coverage for the property type shorthands (protocol version
//! 14): a contract writing `"type": "identifier"` and `"type": "bytes"` is
//! registered by a signed contract create and stored as sent; documents are
//! created against the contract read back from state, refused when a value
//! has the wrong length, and queried by the value of each shorthand, with and
//! without a proof. A signed contract update rewriting both properties in
//! their long form is no change, and the documents keep answering.

use super::*;

mod property_type_shorthand_tests {
    use super::super::reference_test_setup::{
        assert_successful, create_document, process_and_commit,
    };
    use super::*;
    use crate::platform_types::platform_state::PlatformState;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use ciborium::value::Value as CborValue;
    use dapi_grpc::platform::v0::get_documents_request::{
        GetDocumentsRequestV0, Version as RequestVersion,
    };
    use dapi_grpc::platform::v0::get_documents_response::{
        get_documents_response_v0, GetDocumentsResponseV0, Version as ResponseVersion,
    };
    use dapi_grpc::platform::v0::{GetDocumentsRequest, GetDocumentsResponse};
    use dpp::consensus::basic::BasicError;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::data_contract::document_type::DocumentPropertyType;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::Document;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::prelude::DataContract;
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use drive::query::{InternalClauses, WhereClause, WhereOperator};
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    /// A `payment` type keyed by who it pays (an identifier) and the
    /// transaction that paid it (32 bytes), each read by an index, the
    /// transaction hash a unique one; written with the shorthands
    /// (`shorthand`) or in full.
    fn payment_schema(shorthand: bool) -> Value {
        let (recipient, tx_hash) = if shorthand {
            (
                platform_value!({ "type": "identifier", "position": 1 }),
                platform_value!({ "type": "bytes", "size": 32, "position": 2 }),
            )
        } else {
            (
                platform_value!({
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 1
                }),
                platform_value!({
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "position": 2
                }),
            )
        };
        platform_value!({
            "type": "object",
            "properties": {
                "amount": { "type": "integer", "minimum": 1, "maximum": 1000000, "position": 0 },
                "recipientId": recipient,
                "txHash": tx_hash
            },
            "indices": [
                { "name": "byRecipient", "properties": [{ "recipientId": "asc" }] },
                { "name": "byTxHash", "properties": [{ "txHash": "asc" }], "unique": true }
            ],
            "required": ["amount", "recipientId", "txHash"],
            "additionalProperties": false
        })
    }

    fn payment_contract_value(contract_id: Identifier, owner_id: Identifier) -> Value {
        platform_value!({
            "$formatVersion": "1",
            "id": contract_id,
            "ownerId": owner_id,
            "version": 1,
            "documentSchemas": { "payment": payment_schema(true) }
        })
    }

    struct PaymentFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        identity: Identity,
        owner: Identifier,
        /// The contract as the create sent it
        sent: DataContract,
        /// The contract read back from state
        stored: DataContract,
        next_nonce: u64,
        rng: StdRng,
    }

    impl PaymentFixture {
        async fn new() -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let (identity, signer, key) = setup_identity(&mut platform, 977, dash_to_credits!(1));

            let identity_nonce = 1;
            let contract_id =
                DataContract::generate_data_contract_id_v0(identity.id(), identity_nonce);
            let sent = DataContract::from_value(
                payment_contract_value(contract_id, identity.id()),
                true,
                platform_version,
            )
            .expect("the shorthand contract parses");
            let create = DataContractCreateTransition::new_from_data_contract(
                sent.clone(),
                identity_nonce,
                &identity.clone().into_partial_identity_info(),
                key.id(),
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the contract create");
            let platform_state = platform.state.load();
            assert_successful(
                &process_and_commit(&platform, &platform_state, &create, platform_version),
                "a contract written with the shorthands registers",
            );

            let stored = Self::fetch_contract(&platform, contract_id);

            Self {
                platform,
                signer,
                key,
                owner: identity.id(),
                identity,
                sent,
                stored,
                // The create set the owner's nonce for the contract to the
                // identity nonce it used
                next_nonce: identity_nonce + 1,
                rng: StdRng::seed_from_u64(978),
            }
        }

        /// The contract as Drive holds it, parsed as a contract read from state is.
        fn fetch_contract(
            platform: &TempPlatform<MockCoreRPCLike>,
            contract_id: Identifier,
        ) -> DataContract {
            platform
                .drive
                .fetch_contract(
                    contract_id.to_buffer(),
                    None,
                    None,
                    None,
                    PlatformVersion::latest(),
                )
                .unwrap()
                .expect("the contract is stored")
                .unwrap()
                .contract
                .clone()
        }

        /// Sends a signed update rewriting `recipientId` and `txHash` in their
        /// long form, then reads the contract back from state.
        async fn update_to_the_long_form(&mut self) {
            let platform_version = PlatformVersion::latest();
            let mut updated = self.sent.clone();
            updated.set_version(2);
            updated
                .set_document_schema(
                    "payment",
                    payment_schema(false),
                    true,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("the long form registers");
            let nonce = self.next_nonce;
            self.next_nonce += 1;
            let update = DataContractUpdateTransition::new_from_data_contract(
                updated,
                &self.identity.clone().into_partial_identity_info(),
                self.key.id(),
                nonce,
                0,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the contract update");
            let platform_state = self.platform.state.load();
            assert_successful(
                &process_and_commit(&self.platform, &platform_state, &update, platform_version),
                "rewriting the shorthands in full is no change",
            );
            self.stored = Self::fetch_contract(&self.platform, self.stored.id());
        }

        async fn create(&mut self, recipient: Value, tx_hash: Value) -> (Document, bool) {
            let platform_version = PlatformVersion::latest();
            let platform_state: Arc<PlatformState> = self.platform.state.load_full();
            let nonce = self.next_nonce;
            self.next_nonce += 1;
            let (document, result) = create_document(
                &self.platform,
                &platform_state,
                &self.stored,
                "payment",
                &[
                    ("amount", Value::U64(250)),
                    ("recipientId", recipient),
                    ("txHash", tx_hash),
                ],
                self.owner,
                &self.key,
                nonce,
                &self.signer,
                &mut self.rng,
                platform_version,
            )
            .await;
            match result.execution_results().as_slice() {
                [StateTransitionExecutionResult::SuccessfulExecution { .. }] => (document, true),
                [StateTransitionExecutionResult::PaidConsensusError { error, .. }] => {
                    assert!(
                        matches!(
                            error,
                            ConsensusError::BasicError(BasicError::JsonSchemaError(_))
                        ),
                        "expected a JSON schema error, got {error:?}"
                    );
                    (document, false)
                }
                other => panic!("unexpected processing result {other:?}"),
            }
        }

        /// The payments whose `property` equals `value`, from the documents
        /// query, answered with a proof (`prove`) or without one.
        fn query(&self, property: &str, value: Value, prove: bool) -> Vec<Document> {
            let platform_version = PlatformVersion::latest();
            let clause = WhereClause {
                field: property.to_string(),
                operator: WhereOperator::Equal,
                value,
            };
            let mut where_serialized = Vec::new();
            let where_cbor: CborValue = Value::Array(vec![clause.clone().into()])
                .try_into()
                .expect("the where clause converts to CBOR");
            ciborium::ser::into_writer(&where_cbor, &mut where_serialized)
                .expect("the where clause serializes");

            let platform_state = self.platform.state.load();
            let response = self
                .platform
                .platform
                .query_documents(
                    GetDocumentsRequest {
                        version: Some(RequestVersion::V0(GetDocumentsRequestV0 {
                            data_contract_id: self.stored.id().to_vec(),
                            document_type: "payment".to_string(),
                            r#where: where_serialized,
                            order_by: vec![],
                            limit: 10,
                            prove,
                            start: None,
                        })),
                    },
                    &platform_state,
                    platform_version,
                )
                .expect("the query runs");
            assert!(response.errors.is_empty(), "{:?}", response.errors);
            let Some(GetDocumentsResponse {
                version: Some(ResponseVersion::V0(GetDocumentsResponseV0 { result, .. })),
            }) = response.data
            else {
                panic!("expected a v0 response");
            };
            let document_type = self
                .stored
                .document_type_for_name("payment")
                .expect("the payment type");
            match result {
                Some(get_documents_response_v0::Result::Documents(documents)) => documents
                    .documents
                    .iter()
                    .map(|bytes| {
                        Document::from_bytes(bytes, document_type, platform_version)
                            .expect("a stored payment deserializes")
                    })
                    .collect(),
                Some(get_documents_response_v0::Result::Proof(proof)) => {
                    let query = DriveDocumentQuery {
                        contract: &self.stored,
                        document_type,
                        internal_clauses: InternalClauses {
                            equal_clauses: BTreeMap::from([(property.to_string(), clause)]),
                            ..Default::default()
                        },
                        offset: None,
                        limit: Some(10),
                        order_by: Default::default(),
                        start_at: None,
                        start_at_included: false,
                        block_time_ms: None,
                        resolved_time_ranges: vec![],
                        sub_queries: vec![],
                    };
                    query
                        .verify_proof(&proof.grovedb_proof, platform_version)
                        .expect("the proof verifies")
                        .1
                }
                None => panic!("expected documents or a proof"),
            }
        }
    }

    fn ids(documents: &[Document]) -> Vec<Identifier> {
        let mut ids: Vec<_> = documents.iter().map(|document| document.id()).collect();
        ids.sort();
        ids
    }

    #[tokio::test]
    async fn should_register_create_and_query_documents_of_a_contract_written_with_the_shorthands()
    {
        let mut fixture = PaymentFixture::new().await;

        // Stored as sent: the schema the create carried, shorthands and all,
        // and the bytes it serialized to
        assert_eq!(
            fixture.stored.document_schemas(),
            fixture.sent.document_schemas()
        );
        let stored_schema = fixture
            .stored
            .document_schemas()
            .get("payment")
            .copied()
            .cloned()
            .expect("the payment schema");
        assert_eq!(
            stored_schema
                .get_value_at_path("properties.recipientId")
                .expect("recipientId")
                .get_optional_str("type")
                .expect("a type"),
            Some("identifier")
        );
        assert_eq!(
            stored_schema
                .get_value_at_path("properties.txHash")
                .expect("txHash")
                .get_optional_integer::<u16>("size")
                .expect("a size"),
            Some(32)
        );
        let payment_type = fixture
            .stored
            .document_type_for_name("payment")
            .expect("the payment type");
        assert_eq!(
            payment_type
                .flattened_properties()
                .get("recipientId")
                .map(|property| &property.property_type),
            Some(&DocumentPropertyType::Identifier)
        );
        assert!(payment_type.binary_paths().contains("txHash"));

        let alice = Identifier::new([0xA1; 32]);
        let bob = Identifier::new([0xB0; 32]);
        let (first, created) = fixture
            .create(
                Value::Identifier(alice.to_buffer()),
                Value::Bytes(vec![1; 32]),
            )
            .await;
        assert!(created, "a payment to alice is created");
        let (second, created) = fixture
            .create(
                Value::Identifier(alice.to_buffer()),
                Value::Bytes(vec![2; 32]),
            )
            .await;
        assert!(created, "a second payment to alice is created");
        let (third, created) = fixture
            .create(
                Value::Identifier(bob.to_buffer()),
                Value::Bytes(vec![3; 32]),
            )
            .await;
        assert!(created, "a payment to bob is created");

        // The validator compiled from the long form refuses a value of the
        // wrong length for either shorthand
        let (_, created) = fixture
            .create(Value::Bytes(vec![0xA1; 31]), Value::Bytes(vec![4; 32]))
            .await;
        assert!(!created, "a 31-byte identifier is refused");
        let (_, created) = fixture
            .create(
                Value::Identifier(bob.to_buffer()),
                Value::Bytes(vec![5; 33]),
            )
            .await;
        assert!(!created, "a 33-byte transaction hash is refused");

        for prove in [false, true] {
            assert_eq!(
                ids(&fixture.query("recipientId", Value::Identifier(alice.to_buffer()), prove)),
                ids(&[first.clone(), second.clone()]),
                "payments to alice, prove {prove}"
            );
            assert_eq!(
                ids(&fixture.query("recipientId", Value::Identifier(bob.to_buffer()), prove)),
                ids(&[third.clone()]),
                "payments to bob, prove {prove}"
            );
            assert_eq!(
                ids(&fixture.query("txHash", Value::Bytes(vec![2; 32]), prove)),
                ids(&[second.clone()]),
                "the payment by its transaction hash, prove {prove}"
            );
        }

        // An update spelling both properties in full changes nothing: it is
        // accepted, the contract now stores the long form, and the documents
        // written under the shorthands answer as before
        fixture.update_to_the_long_form().await;
        let stored_schema = fixture
            .stored
            .document_schemas()
            .get("payment")
            .copied()
            .cloned()
            .expect("the payment schema");
        assert_eq!(
            stored_schema
                .get_value_at_path("properties.txHash")
                .expect("txHash")
                .get_optional_str("type")
                .expect("a type"),
            Some("array")
        );
        for prove in [false, true] {
            assert_eq!(
                ids(&fixture.query("recipientId", Value::Identifier(alice.to_buffer()), prove)),
                ids(&[first.clone(), second.clone()]),
                "payments to alice after the update, prove {prove}"
            );
            assert_eq!(
                ids(&fixture.query("txHash", Value::Bytes(vec![3; 32]), prove)),
                ids(&[third.clone()]),
                "the payment by its transaction hash after the update, prove {prove}"
            );
        }
    }
}
