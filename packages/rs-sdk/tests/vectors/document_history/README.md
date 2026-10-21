# Signed document-history responses

These synthetic PV15 fixtures come from the ABCI test
`query::document_history::v0::tests::should_round_trip_history_api_through_quorum_and_grove_proof_verification`.
They contain real GroveDB history and lifecycle proofs and Tenderdash signatures,
using that harness's public test key (`SecretKey::from_be_bytes(&[1; 32])`). They
are offline test data, not responses recorded from a live network.

The harness writes three profile revisions at time 2000. The revision fixture
selects revision 2; the cursor fixture selects revisions 2 and 3 after `(2000, 1)`.
Both include the serialized contract and quorum public key so SDK tests need no
prior contract fetch or protocol-version refresh. Requests and responses use the
generated gRPC serde representation. The SDK mocks only the transport; the
GroveDB proofs and quorum signature are verified normally.
