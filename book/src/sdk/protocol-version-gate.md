# Features from Protocol Version 14

Protocol version 14 adds query methods and request fields that nodes of the previous
release do not know. Until a network activates protocol 14, it runs both releases: 4.1
nodes and 5.0 nodes, all at protocol 13. A 4.1 node answers a method it does not know
with `UNIMPLEMENTED`, and decodes a request with the 4.1 protobuf types, dropping the
fields it does not know and answering the request that is left. Either answer gets an
honest node banned: the SDK retries `UNIMPLEMENTED` elsewhere and bans the node, and it
cannot verify the answer to a different question.

So the SDK does not send such a request until it knows every node serves it. A request
that needs it is refused locally, with a dedicated error, and no node is banned.

## What needs protocol version 14

| Request | Needs 14 when |
|---|---|
| `getIdentityKeysRemainingBudgets`, `getDataContractsLatestVersions`, `getDataContractsByRange`, `getContractGroupInfo`, `getContractGroupMembers`, `getContractGroupsForContract`, `getContractModerationStatus`, `getContractModerationEntries`, `getContractDocumentRemovals`, `getContractTeamActions`, `getContractTeamActionSigners`, `getContractModerationActionCounts`, `getContractFeePots` | always |
| `getDocuments` | a composite query (`sub_queries`) or a chained query (`chained`) |
| `getShieldedPoolState`, `getShieldedNotesCount`, `getShieldedAnchors`, `getMostRecentShieldedAnchor`, `getShieldedEncryptedNotes`, `getShieldedNullifiers` | a token's pool is asked for (`token_id`); requests for the credit pool are the ones 4.1 SDKs send, and are always sent |

Other protocol 14 additions are refused alike by every node before protocol 14 (for
example the `contains` and time-range where operators, which nodes at 13 answer with
`INVALID_ARGUMENT`), or are selected by the SDK only once protocol 14 is verified (the
non-unique key lookup's request version 1). Neither bans a node.

Each request declares what it needs through
`TransportRequest::needs_protocol_version` in `rs-dapi-client`. A test in
`rs-dapi-client/tests/request_inventory.rs` compares every request field, oneof arm,
enum value and method with what 4.1 nodes understand, and fails on any addition without
a declaration.

## How the SDK decides

The SDK sends such a request when both hold:

- a response whose proof and quorum signature it verified was signed at protocol 14 or
  later: proof that every node answering now runs 5.0 or newer, since no 4.1 node can
  sign protocol 14; and
- its own protocol version is 14 or later. A version seeded at build time, persisted from
  an earlier session or pinned with `SdkBuilder::with_version` says nothing about the
  network on its own.

This is the same rule that selects the non-unique key lookup's request version 1; both use
`QuerySettings::network_protocol_basis`. An unpinned SDK on a network proven at a version
newer than it knows speaks the newest version it knows, so that the newer network serves
its protocol 14 features.

When the network may have reached protocol 14 since the SDK last heard from it (nothing
verified yet, or the last verified answer was signed below 14), the SDK first asks for the
genesis epoch, with a proof whatever the SDK's proof setting, using the caller's request
settings. One round trip; once an answer signed at 14 has been verified, the SDK never
asks again. It does not ask when no answer can change the outcome:

- an SDK pinned below 14 speaks its pinned version, whatever the network runs;
- an SDK without a context provider cannot verify the answer, and an answer it cannot
  verify would ban the node that sent it.

Mock SDKs do not enforce the rule: they answer what a test registered, and have no nodes
to ban.

## The refusal

| Surface | Error |
|---|---|
| Rust (`dash-sdk`) | `Error::ProtocolVersionNotActive { method, required, network, cause }` |
| JS (`wasm-sdk`, `evo-sdk`) | `WasmSdkError` with kind `ProtocolVersionNotActive` (`error.name === 'ProtocolVersionNotActive'`), `isRetriable` false |
| FFI (Swift, Kotlin) | `DashSDKErrorCode::ProtocolError` |

`method` names the gRPC method (for example `get_contract_fee_pots`), `required` the
version it needs, and `network` the version the SDK could rely on: the lower of its own
version and the newest version a verified answer was signed at, 0 before the first one.
When asking for the network's version failed, `cause` carries that error, and the JS and
FFI messages include it.

The error is not retryable: another node cannot help. An app that offers a protocol 14
feature can catch it and hide the feature until the network activates protocol 14; the
same call succeeds once it has.

## At the activation

When the network activates protocol 14, a node that has not caught up yet still answers
at protocol 13. An answer to one of these requests signed below 14 is treated as coming
from a stale node (`StaleNodeError::ProtocolVersion`): it is discarded, the node is
banned, and the SDK asks another one.

## Adding a request

A method or request field that only a newer release serves declares that release's first
protocol version in `impl_transport_request_grpc!` (`needs_protocol_version: ...`), and
gets a line in the inventory test's `ADDITIONS`. A feature a later release adds declares
that release's protocol version, not 14. A caller that executes a request directly
(`DapiRequestExecutor::execute`) skips the SDK's check and owns it.
