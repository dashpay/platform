# Features from Protocol Version 14

Protocol version 14 adds query methods, request fields and document query features that nodes
of the previous release do not serve. Until a network activates protocol 14, it runs both
releases: 4.1 nodes and 5.0 nodes, all at protocol 13. A 4.1 node answers a method it does not
know with `UNIMPLEMENTED`, and decodes a request with the 4.1 protobuf types, dropping the
fields it does not know and answering the request that is left. Either answer gets an honest
node banned: the SDK retries `UNIMPLEMENTED` elsewhere and bans the node, and it cannot verify
the answer to a different question. Some document features are refused by every node at 13,
and one fails there in a way that bans the node.

So the SDK does not send such a request until a verified answer shows the network runs
protocol 14. A request that needs it is refused locally, with a dedicated error, and no node is
banned. [Protocol Versions in the SDK](protocol-versions.md) describes how the SDK decides, how
long a probe of the network's version holds, and the errors.

## What needs protocol version 14

| Request | Needs 14 when |
|---|---|
| `getIdentityKeysRemainingBudgets`, `getDataContractsLatestVersions`, `getDataContractsByRange`, `getContractGroupInfo`, `getContractGroupMembers`, `getContractGroupsForContract`, `getContractModerationStatus`, `getContractModerationEntries`, `getContractDocumentRemovals`, `getContractTeamActions`, `getContractTeamActionSigners`, `getContractModerationActionCounts`, `getContractFeePots` | always |
| `getDocuments` | the query uses any of: a composite query (sub-queries), a chained query, `contains` or `containsAny`, a time-range or integer-range selection, an aggregate ordered by the selected aggregate (ranked), a `having` clause, several `in` clauses in a documents query, or a start cursor on a document type whose documents are summed (`documentsSummable`) |
| `getShieldedPoolState`, `getShieldedNotesCount`, `getShieldedAnchors`, `getMostRecentShieldedAnchor`, `getShieldedEncryptedNotes`, `getShieldedNullifiers` | a token's pool is asked for (`token_id`); requests for the credit pool are the ones 4.1 SDKs send, and are always sent |

The non-unique key lookup's request version 1 is selected only once protocol 14 is verified,
and is otherwise sent as version 0, which every node serves.

How each request declares what it needs:

- A method or request field declares it through `TransportRequest::needs_protocol_version` in
  `rs-dapi-client`. A test in `rs-dapi-client/tests/request_inventory.rs` compares every request
  field, oneof arm, enum value and method with what 4.1 nodes understand, and fails on any
  addition without a verdict.
- A document query declares it through `DocumentQuery::required_protocol_version` (and
  `ChainedDocumentQuery::required_protocol_version`) in `dash-platform-queries`, from the
  features it uses. Each feature reads a version table slot: a slot that is `None` before the
  protocol version that adds a field older nodes drop (`document_sub_queries`,
  `chained_document_query`), or the slot the server switches on for a route
  (`document_type_schema`, `compute_aggregate_mode_and_check_limit`, `where_clause_grouping`,
  `start_at_document`). The refusal names the feature.

## The refusal

| Surface | Error |
|---|---|
| Rust (`dash-sdk`) | `Error::ProtocolVersionNotActive { method, required, feature, network, pinned, cause }` |
| JS (`wasm-sdk`, `evo-sdk`) | `WasmSdkError` with kind `ProtocolVersionNotActive` (`error.name === 'ProtocolVersionNotActive'`) |
| FFI (Swift, Kotlin) | `DashSDKErrorCode::ProtocolError`, with a message that starts with `<method> needs Platform protocol version`; `NetworkError` when asking for the network's version failed for a reason that can pass |

Another node cannot help, so the SDK never retries the request elsewhere and bans no node for
it. An app that offers a protocol 14 feature can catch the error and hide the feature until the
network activates protocol 14; the same call succeeds once it has. When asking for the
network's version failed for a reason that can pass (nodes or a quorum source not answering),
the network may serve the request already: JS reports the error as retriable (`isRetriable`),
and FFI as a network error, so the app can try again later instead of hiding the feature. A
failure the SDK's own setup causes, no context provider or one configured so that it cannot give
quorum keys (a non-trusted EvoSDK), is not retriable: Rust apps can tell the two apart with
`Error::is_transient_protocol_version_refusal`.

Right after an activation, a node that has not caught up can still answer the SDK's question
for the network's version at 13. The refusal it causes lasts until that node's epoch can have
ended, which it already has or is about to; an app that hides a feature on a refusal should
look again soon.

## Adding a request

A method or request field that only a newer release serves declares that release's first
protocol version in `impl_transport_request_grpc!` (`needs_protocol_version: ...`), and gets a
line in the inventory test's `ADDITIONS`. A feature a later release adds declares that release's
protocol version, not 14.

A `getDocuments` field or operator a later release adds gets an `OptionalFeatureVersion` slot in
the version tables instead, `None` in every table before the protocol version that adds it
(backfilled into the shipped tables), and `DocumentQuery::required_protocol_version` reads it.
A new route the server opens is read from the slot the server switches on. Its inventory line
is `NoGate`, since the wire request declares nothing.

A caller that executes a request directly (`DapiRequestExecutor::execute`) skips the SDK's check
and owns it.
