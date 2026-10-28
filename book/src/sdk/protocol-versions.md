# Protocol Versions in the SDK

A Dash Platform network runs one protocol version at a time, and moves to the next only when
an epoch starts. Its nodes do not all run the same release: during an upgrade, nodes of the
previous release and of the new one answer side by side, and a node that has fallen behind
still answers at the version it last saw. The SDK has to send requests every node it may reach
serves, and read each answer with the rules of the version that produced it. This chapter
describes how it keeps track of versions and what it does with them.

## The versions the SDK keeps

| Version | What it is | Where it comes from |
|---|---|---|
| Stored | The version whose rules the SDK builds requests and transitions with (`Sdk::version`) | Seeded, initial or pinned at build time; raised by verified answers |
| Verified | The newest version a verified answer was signed at (`ProtocolVersionInfo::verified_version`) | Verified answers only; none before the first (`None`, and 0 in the network basis) |
| Network basis | The version the SDK may rely on both the network and itself serving (`QuerySettings::network_protocol_basis`) | The lower of the two above |

The stored version starts at one of three values:

- **Seeded.** `SdkBuilder` picks a per-network default (`min_protocol_version`), a version
  every node of that network serves; it may be older than the version the network runs.
- **Initial.** `SdkBuilder::with_initial_version` starts it elsewhere, for example at a version
  an app persisted from an earlier session. The SDK can tell the app each time the version
  rises (`SdkBuilder::with_protocol_version_observer`).
- **Pinned.** `SdkBuilder::with_version` fixes it: no answer moves it.

An unpinned stored version only ever rises. It rises to the version a verified answer was
signed at, when this build knows that version. An unpinned SDK on a network proven to run a
version newer than this build knows speaks the newest version it knows.

None of the three values says what the network runs. A seed may be older than the network, a
persisted version may come from another network, and a pin may be ahead of it. Only the
verified version is evidence, which is why the network basis takes the lower of the two.

## Learning from verified answers

Every proved answer carries metadata: the block height and time, and the protocol version
that signed it. The quorum signature covers these fields together with the proof. Once the
proof and the signature verify, the SDK checks that the answer is fresh: its time within the
time tolerance of the local clock, and its height not too far below the highest height seen.
Only then does it learn from it, raising the verified version and the stored one.

Nothing is learned from metadata that was not verified: an unproved answer, an answer whose
proof fails, or metadata a caller hands to `Sdk::verify_response_metadata`.
`Sdk::refresh_protocol_version` asks the network for a proved answer to learn from on purpose;
on an SDK that does not request proofs it does nothing, and only the probe described below
learns for it.

## Request and response versions

Most requests are versioned envelopes, `V0`, `V1` and so on, and so are their responses. A
version table (`drive_abci.query.<method>`) lists which envelopes the nodes of each protocol
version accept. The SDK encodes a request in the envelope its stored version selects, so a
request built at a version the network does not run yet may not be understood.

Where a newer envelope is refused by older nodes, the SDK picks it only from the network basis:
it sends the older envelope until a verified answer shows the network accepts the newer one.
Document requests are the exception: every network the SDK is seeded for serves their `V1`
envelope (protocol 12 and later), so the SDK sends `V1` from its stored version and judges each
document feature separately, as the next section describes.

A request whose answer has a new shape gets a new envelope version rather than a new field in
an existing one. A node that does not know the new version refuses it, instead of answering
in the old shape.

## Requests only newer nodes serve

A protocol upgrade can add a method, a request field or a query feature. Nodes below the
version that adds it do not serve it:

- a method they do not know is answered `UNIMPLEMENTED`;
- a field they do not know is dropped, and the node proves the request without it, which is
  another question;
- a feature they do not route is refused, or served as another question.

Several of these answers get an honest node banned, and none answers the request. So before
sending a request the SDK asks what it needs (`Query::protocol_requirement`):

- for most requests, what the request declares (`TransportRequest::needs_protocol_version`);
- for a document query, what its features need (`DocumentQuery::required_protocol_version`),
  read from the version tables: a feature slot for a field older nodes drop, or the slot the
  server itself switches on for a route.

The SDK then compares the requirement with the network basis:

| The SDK | When |
|---|---|
| Sends | the network basis is at least the requirement, or the SDK is a mock, which has no nodes to ban |
| Refuses | it is pinned below the requirement, or has no context provider to verify an answer with, or a recent probe proved the network below it (see below) |
| Probes, then decides again | otherwise |

The probe asks for the genesis epoch, with a proof whatever the SDK's proof setting, under the
caller's request settings. Every node serves it at every version, and its verified answer
raises the verified version like any other. An SDK that does not request proofs still asks for
this one, and bans no node for its answers: its context provider may not be able to give
quorum keys, and then the probe fails on every node, which says nothing about the nodes.

Calls that need a probe while one runs, from the SDK or its clones, wait for its answer and
decide again from it, so one probe serves them all. When it proves too little to decide, for
example because it failed, each of them asks under its own request settings at once.

A refused request is not sent and bans no node. [Features from Protocol Version
14](protocol-version-gate.md) lists the requests and features this applies to.

### How long a probe holds

The protocol version changes only when an epoch starts, and epochs follow a fixed grid: epoch
`n` runs from `n` epoch lengths after the genesis block to `n + 1`, and starts with its first
block. When a probe shows the network below the requirement, the SDK also proves the current
epoch: it asks for the epoch the answer names as current and the epoch after it, and the proof
shows that the first has started and the second has not, and when the first started.

The genesis epoch's start, which the probe proves, and the current epoch's start bound the
epoch length from above, so they tell when the current epoch has ended at the latest. Until
then the SDK refuses requests that need more than the version it saw, without asking again.
It measures the time on the local clock from when the answer arrived, and never past that
bound by block time, so neither a local clock behind block time nor an older answer lengthens
the refusal. An SDK and its clones share
this. Any verified answer that shows the network at the requirement, or at a newer version than
the probe saw, lifts the refusal at once, whichever request it answered.

The bound is late by about how long after its start on the grid the current epoch's first
block came, which is seconds while blocks are made, and longer after a halt across the epoch's
start. Then a request may be refused after the network has moved to a newer version, until the
next verified answer or the end of the hold.

Nothing is held when the epoch cannot be proven: when the answer comes from the genesis epoch,
or when the epoch query fails or proves another epoch current. The next request that needs more
asks again.

### At the activation

When the network moves to a new version, a node that has not caught up still answers at the
old one. An answer to a request that needs the new version, signed below it, is treated as
coming from a stale node (`StaleNodeError::BehindProtocolUpgrade`): the request moves to
another node, and this one is banned.

A probe can reach such a node too. Its answer, fresh and signed at the old version, can decide
that call: unless the epoch query that follows reaches an up-to-date node, the request is
refused as if the network had not moved. The epoch that node proves
has already ended or is about to, so the next request asks again, usually of another node.

## Document answers

Document queries (documents, counts, sums, averages and their split forms, ranked, having,
composite and chained queries) are verified at the version that signed them. An answer signed at a
newer version than the SDK's is verified with that version's rules when this build knows it,
and with the newest rules it knows otherwise.

An answer signed at an older version than the SDK's stored version is not verified. What the
SDK does with it depends on what verified answers have shown:

- **The network is proven at the SDK's version.** The node is behind its network. The answer
  fails as a proof error: the request moves to another node, and this one is banned.
- **Nothing has shown it yet.** The SDK may be pinned, seeded or persisted ahead of its
  network, where every node answers this way. The request moves to another node, and none is
  banned (`StaleNodeError::ProtocolVersion`). A request whose every attempt ends this way fails
  with `Error::Config`. An answer outside the height or time tolerance still fails as stale, and
  its node is banned.

Before a document query is sent, the result type checks that it can be verified from an answer
to it (`FromProof::check_request`), at the version the request is encoded with. A query no node
serves, or none of whose answers can be verified as that type, is refused without being sent.

## Errors

| Situation | Rust (`dash-sdk`) | JS (`wasm-sdk`, `evo-sdk`) | FFI (Swift, Kotlin) | Retried elsewhere / node banned |
|---|---|---|---|---|
| The network is not proven to serve what the request needs | `Error::ProtocolVersionNotActive { method, required, feature, network, pinned, cause }` | kind `ProtocolVersionNotActive` | `ProtocolError`, or `NetworkError` when learning the network's version failed for a reason that can pass | no / no |
| No node can answer the request as the requested result type: refused before sending, or a fresh verified answer cannot be returned as that type (several documents for a single-document fetch, a total that overflows) | `Error::InvalidRequest` | kind `InvalidArgument` | `InvalidParameter` | no / no |
| An answer to a request that needs a version, signed below it | `StaleNodeError::BehindProtocolUpgrade` | kind `StaleNode` | `NetworkError` | yes / yes |
| A document answer signed below the SDK's version, before the network is proven at it | `StaleNodeError::ProtocolVersion` per attempt; the request fails with `Error::Config` once no attempt is left | kind `Config` | `ProtocolError` | yes / no |
| A document answer signed below the SDK's version, once the network is proven at it | `Error::Proof` | kind `Proof` | `ProtocolError` (its message names both versions) | yes / yes |

`ProtocolVersionNotActive` names the method and, for a document query, the feature that needs
the version, for example `get_documents needs Platform protocol version 14 for time-range
selections`. `network` is the network basis, `pinned` says no answer can lift the refusal, and
`cause` holds the error when learning the network's version failed. A refusal is transient only
when that failure can pass (`Error::is_transient_protocol_version_refusal`; JS `isRetriable`):
then the network may serve the request already. An app that offers a newer feature can catch
the refusal and hide the feature until the network serves it.
