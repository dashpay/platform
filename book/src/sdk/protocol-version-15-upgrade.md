# Updating Apps for Protocol Version 15

Protocol version 15 arrives with Platform 5.1. It changes how some state is stored and proved,
and an app built on a 5.0.x SDK cannot read those parts of a network that runs it.

**Wallets and apps must update to a 5.1 SDK before protocol version 15 activates on the network
they use.** This applies to every SDK: Rust (`dash-sdk`), JavaScript (`@dashevo/evo-sdk`,
`@dashevo/wasm-sdk`), Swift and Kotlin.

A 5.1 SDK reads a network at protocol version 14 and at 15, so an app can ship the update ahead
of the activation and needs no second release when the network switches. The upgrade rehearsal
ran iOS and Android apps built from 5.1 through the switch without a restart.

Nothing can be changed on the network side to keep a 5.0.x client reading these parts: a proof
carries the bytes as they are stored, and only a client that knows the new forms can verify it.

## What stops working for a 5.0.x client

From the first block of protocol version 15:

| What fails | Why | What the app sees |
|---|---|---|
| Fetching the DPNS and DashPay contracts, and every contract created or updated at protocol version 15 (a token configuration update re-stores its contract too) | Contracts are stored compiled, in contract format version 2. The activation block re-stores DPNS and DashPay at their version 3. A proof of a contract carries the stored bytes, and a 5.0.x SDK reads formats 0 and 1 only. | A proof error: `UnexpectedVariant { type_name: "DataContractInSerializationFormat", allowed: 0..=1, found: 2 }` |
| Everything that needs one of those contracts: resolving and registering DPNS names, DashPay profiles and contact requests, and document queries on a contract created or updated at protocol version 15 | The SDK fetches the contract to build and verify the document query. | The same proof error |
| Document history (`getDocumentHistory`) | Retained revisions move to per-type history trees at the activation. Request version 1 reads them, and request version 0, the one 5.0.x sends, is retired. | `unsupported version for query: document_history, currently supporting versions 1 to 1 on platform protocol 15, given 0` |
| Sum queries, and ranked queries ordered by a sum, on a document type that sums a property not declared within ±2^27 | Such a type keeps its sums in trees with a 128-bit total. The activation converts the trees of types registered earlier, so contracts from before the upgrade are affected too. A 5.0.x SDK does not know those GroveDB element types. Other queries whose proof passes through one of these trees can fail the same way. | A proof error: `cannot determine element type in KVValueHash node ... Unknown element type discriminant: 32` |
| Confirming a replace of a document whose type keeps history | The proof of the result follows the new history layout. | The replace is applied, but the SDK cannot verify the result and reports a proof error |

A replace that fails this way has still been executed. An app must not submit it again: read
the document to confirm instead.

### Banned nodes make it worse

A 5.0.x SDK treats an answer it cannot verify as its node's fault: it bans the node and asks
another, which answers the same way. In the rehearsal one DPNS lookup banned every node it
tried, and the app's other requests then failed with `no available addresses to use` for
minutes, including the ones listed below as still working.

Nodes on 5.1 answer the retired document history request with `INVALID_ARGUMENT`, which a
5.0.x SDK neither retries nor bans a node for. The proof errors are decided inside the client,
so only updating the SDK removes those bans.

## What keeps working for a 5.0.x client

As long as its nodes are not banned by one of the failures above:

- identities, their keys and balances;
- documents of contracts stored before protocol version 15 and not updated since, with their
  counts and ranked-by-count queries, and sums of properties declared within ±2^27;
- contested resource vote state and vote polls;
- token balances, supply and info, and group actions;
- epochs and status;
- writes: state transitions are accepted and executed as before. Only the confirmation of a
  replace on a type that keeps history fails, as described above.

## What an app should do

1. Update to a 5.1 SDK and release before the activation. Protocol version 15 is scheduled one
   epoch ahead, once enough evonodes have proposed blocks on 5.1, so the date is known at least
   one epoch before the switch.
2. If the app still supports an older build in the field, show its users an update notice
   before the activation: after it, name lookups and DashPay stop working there.

## The next upgrade

A 5.1 SDK will be the older client when a later protocol version arrives, so it handles that
case differently from 5.0.x:

- Every verified answer tells the SDK which protocol version signed it. Once one names a
  version the build does not know, `Sdk::protocol_version_info()` reports
  `newer_protocol_available` (Swift: `ProtocolVersionInfo`). An app can show an update notice
  from it.
- From then on, an answer the SDK cannot verify that names such a version does not ban its
  node. The request moves to another node, and when every attempt ends that way it fails with
  a configuration error (`Error::Config`, JS kind `Config`) that says the network runs a newer
  protocol version and the SDK has to be updated. Requests the build can still read keep
  working. [Protocol Versions in the SDK](protocol-versions.md#a-network-newer-than-the-sdk)
  has the details.
- A request version that a protocol upgrade retired is refused with `INVALID_ARGUMENT`: the
  request fails at once and no node is banned.
