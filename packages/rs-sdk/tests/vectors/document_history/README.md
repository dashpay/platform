# Signed document-history responses

These synthetic PV15 fixtures come from the ABCI test
`query::document_history::v1::tests::should_round_trip_history_api_through_quorum_and_grove_proof_verification`.
They contain one GroveDB proof of history and lifecycle per response, and
Tenderdash signatures,
using that harness's public test key (`SecretKey::from_be_bytes(&[1; 32])`). They
are offline test data, not responses recorded from a live network.

The harness writes three profile revisions at time 2000. The revision fixture
selects revision 2; the cursor fixture selects revisions 2 and 3 after `(2000, 1)`.
Both include the serialized contract and quorum public key so SDK tests need no
prior contract fetch or protocol-version refresh. Requests and responses use the
generated gRPC version 1 serde representation. The SDK mocks only the transport;
the GroveDB proof and quorum signature are verified normally. Regenerate with the
ABCI test and `UPDATE_DOCUMENT_HISTORY_VECTORS=1`.

The PV14 `pv14_time.json` fixture comes from
`query::document_history::v1::tests::should_verify_signed_legacy_history_from_protocol_14_with_protocol_15_client`.
It stores revisions at times 1000, 2000 and 3000, then selects the latter two
through the released V0 wire request. Its signed producer version selects the
legacy timestamp history layout even when a PV15 client verifies it. Regenerate
it with that ABCI test and `UPDATE_DOCUMENT_HISTORY_VECTORS=1`.
