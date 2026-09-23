# Results, Receipts and Proofs

A response to a contract call or a contract query can carry several things in
one message: the bytes or error text the guest returned, a stored receipt,
response metadata, and a GroveDB proof. Each of these means something
different, and a valid proof in the response does not make the text next to
it proven. This chapter fixes the vocabulary so that the proof verifier, the
SDKs and the tests all mean the same thing when they say "proved", and so a
client never advertises a contract result as a proof of execution merely
because the same response included a valid state proof.

It also states the receipt policy the smart-contract work implements against
(owner decision Q32 in the plan tracked by
[dashpay/platform#4626](https://github.com/dashpay/platform/issues/4626)),
and lists the requirements the implementing tasks tick. Nothing in this
chapter changes wire formats or versions; it names what already exists and
what the receipt and client tasks must keep true.

## Three result classes

A client holds one of three classes of result after a contract call or a
query. They differ in what produced the data, what binds it, and what the
client may conclude.

| Class | Produced by | Bound to | A client may conclude | A client may not conclude |
|---|---|---|---|---|
| Unproven execution-result text | The node that served the request. The guest's returned bytes, structured error, or a node's status text. | Nothing beyond the node's word. | The node reported this. | That the network agreed, that the result is in state, or that execution happened as described. |
| Stored receipt | The network, when it settled an accepted contract call whose contract retains receipts. | The receipt's key under the contract at the signed current root, once proven. | The current state contains this outcome record under this key, with the bounded outcome, actual fees and failure class it recorded. | That execution was re-run and checked, or what the state looked like at the height the call executed. |
| Current-state proof | Drive's proof generation, verified client side. | The trusted Platform state root plus the complete path and query. | The proved values are exactly what the current state holds at the signed root, including absence where the query proves it. | Anything about a different contract, version, query or root. |

The proofs-and-clients draft proposes the type names `ExecutionOutcome`,
`StoredReceipt` and `VerifiedStateResult` for these three classes. Those are
proposed names, not code: the receipt codec task (R10-03) and the SDK results
task (R10-06) allocate the real types, and may rename them. What this chapter
fixes is that the three stay three distinct types. A converter must not fold
a receipt into a proved-state result, and JSON must not collapse the tagged
value into an untagged one.

## How this extends the existing guarantee tag

Platform already distinguishes two guarantees a verified state-transition
proof can give. The verifier returns a
[`StateTransitionProofOutcome`](https://github.com/dashpay/platform/blob/v5.0-dev/packages/rs-dpp/src/state_transition/proof_result.rs)
with one of two tags:

- **`ExecutionProved`**: the verified values could only exist if this
  specific transition was applied. Document creates, identity creates and
  contract registrations fall here because the proved value carries the
  transition's own identifier, entropy or nonce.
- **`AffectedState`**: the proof authenticates the keys the transition
  affects, as of the proof's block, but cannot bind them to this
  transition's execution. Balance top-ups, credit transfers, address funds
  movements, shields and no-history token operations fall here. The result
  is a height-pinned snapshot, and the SDK says so in the type.

The Rust SDK exposes the two as two API pairs on `BroadcastStateTransition`
(see [Put Operations](put-operations.md)). The strict pair
(`wait_for_response`, `broadcast_and_wait`) accepts only `ExecutionProved`
and returns `Error::ExecutionNotProved` otherwise. The affected-state pair
(`wait_for_affected_state`, `broadcast_and_wait_for_affected_state`) accepts
both tags because `ExecutionProved` is the strictly stronger guarantee. The
JavaScript SDK mirrors the pairs as `waitForResponse` and
`waitForAffectedState`. For reads that need no proof at all, `FetchUnproved`
in the Rust SDK and the `FromUnproved` trait in the proof verifier decode a
response without verifying it; their output is unproven execution-result
text in the vocabulary above, whatever its Rust type.

Contract calls sit alongside this tag rather than replacing it. The following
mapping is the working interpretation for the SDK results task (R10-06),
which owns the final one:

- A wait on a contract call whose contract retains receipts returns the
  receipt as an `ExecutionProved` result when the proved receipt key binds
  the specific outer invocation. Provisionally the key binds the caller's
  identity and nonce; the receipt codec task (R10-03, register entry A18)
  decides the key.
- A wait on a contract call whose contract disabled receipts can at best
  return `AffectedState` over the keys the call declared it would touch. A
  caller of the strict API receives `ExecutionNotProved` and must choose the
  affected-state API deliberately.
- The invariant the final mapping must keep: no path returns
  `ExecutionProved` without a proof that binds this invocation. A proof of
  the affected keys plus the node's text about the outcome is still
  `AffectedState` plus unproven text, never execution evidence.

## What a receipt proves, and what it does not

A stored receipt is a record. Proving it means proving inclusion of that
record under its key at the signed current root, and the verifier binds the
contract, the key and the exact query the same way it binds every other
current-state proof. From a verified receipt a client learns:

- the network accepted the outer invocation and settled it;
- the bounded outcome the receipt recorded, including bounded nested
  outcomes for calls the invocation made;
- the actual fees charged and, on paid failure, the failure class.

A receipt does not prove:

- that the execution was re-run and checked by the verifier. Verifying a
  receipt never loads the runtime; the verifier trusts the network's
  settlement the same way it trusts any other committed value;
- what the state looked like at the height the call executed. Platform
  offers no past-height proof service in 5.0, and a receipt in current state
  is not one;
- anything the receipt does not record. A receipt bounded at its declared
  size (provisionally 64 KiB from the allocation register) can truncate
  nested detail; the truncation is part of the record, not evidence of
  more.

One receipt is written per outer invocation. Internal speculative retries
and node faults write none: only an accepted result charges fees, consumes
the top-level nonce or creates a receipt.

## Receipt policy (Q32, confirmed)

The owner confirmed the receipt policy in the plan's review. It is binding on
every task that stores, removes, queries or displays receipts:

1. **Receipt storage is enabled by default.** A contract that declares
   nothing about receipts gets them.
2. **A contract may disable receipts.** The declaration is part of the
   contract's manifest, captured with its owner before execution so the
   policy in force at execution is the one recorded.
3. **Retention is contract-defined.** The contract declares how long its
   receipts stay.
4. **No declared expiry means retain until authorized removal.** Neither
   the node, the SDK nor any cleanup job invents an automatic expiry.
   Indefinite retention creates no expiry entry.
5. **Canonical code retention is independent, in both directions.**
   Disabling receipts, removing receipts under retention, and the bounded
   cleanup after a logical wipe never delete code or manifests. Permanent
   code never implies permanent receipts.

"Authorized removal" means one of two things: a removal the contract's own
retention rules authorize, or the bounded cleanup that follows a logical
wipe of the contract. Those are the only removal paths. The exact transition
shape for a retention update is the receipt codec task's allocation (A18).

## Absence is not failure

A receipt query that returns nothing has several possible causes, and a
client must not collapse them into "the call failed":

- the contract disabled receipts;
- the receipt was removed under the contract's declared retention;
- the call was never accepted: it was rejected at check time, or it was
  never included in a block;
- the query ran before the block that included the call;
- the query named the wrong contract or the wrong key.

The response must therefore carry, or the client must fetch, the effective
receipt policy and the contract's status next to an absence, so a user can
read "receipts disabled" or "not yet included" instead of "failed". A paid
failure is an accepted outcome: when receipts are on, it has a receipt like
any success, and its absence means one of the reasons above, not that the
failure happened.

## Outcome classes a client must keep apart

| Outcome | Fee | Nonce | Receipt | Effects | What a proof can establish |
|---|---|---|---|---|---|
| Accepted success | Actual cost charged | Consumed | Written when enabled | Kept | Inclusion of the receipt or of the affected keys at the current root |
| Accepted paid failure | Protocol-specified fee charged | Consumed | Written when enabled | Rolled back | Inclusion of the paid-failure receipt at the current root |
| Rejected transition | None | Not consumed | None | None | Nothing; the consensus error is the node's report |
| Transport error | Unknown | Unknown | Unknown | Unknown | Nothing; retry or query |
| Invalid or wrong-root proof | Unknown | Unknown | Unknown | Unknown | Nothing; the response must be discarded |

The first two rows are outcomes the network settled. The last three are not
outcomes at all: a rejected transition never entered state, a transport
error says nothing about the call, and an invalid proof says nothing about
anything. A client that shows "call failed" for a transport error, or shows
a paid failure as if it were a rejection, has merged rows that the network
keeps apart.

## What each layer returns today

The baseline the contract-call extension builds on:

| API | Layer | Class returned |
|---|---|---|
| `fetch`, `fetch_many` | Rust SDK | Current-state proof (verified before the value is returned) |
| `fetch_with_metadata_and_proof` | Rust SDK | Current-state proof plus the raw proof and metadata |
| `fetch_unproved` | Rust SDK | Unproven execution-result text |
| `wait_for_response`, `broadcast_and_wait` | Rust SDK | Current-state proof tagged `ExecutionProved` |
| `wait_for_affected_state`, `broadcast_and_wait_for_affected_state` | Rust SDK | Current-state proof tagged `ExecutionProved` or `AffectedState` |
| `FromProof` | Proof verifier | Current-state proof |
| `FromUnproved` | Proof verifier | Unproven execution-result text |
| `waitForResponse`, `broadcastAndWait` | JavaScript SDK | Current-state proof tagged `ExecutionProved` |
| `waitForAffectedState`, `broadcastAndWaitForAffectedState` | JavaScript SDK | Current-state proof tagged `ExecutionProved` or `AffectedState` |
| `fetchUnproved` | JavaScript SDK | Unproven execution-result text |

No row returns a stored receipt yet. The contract-call work adds receipt
queries and receipt-bearing wait results without moving any existing row to
a different class.

## Requirements for the implementing tasks

Each item names the task that ticks it.

- **ABI capability declaration (R08-04, register entry A07).** The
  contract manifest carries a receipt policy declaration with an `enabled`
  flag defaulting to true and a retention declaration whose absence means
  indefinite retention. The declaration describes a capability; it grants
  nothing and never changes how a call is authorized.
- **Receipt codec (R10-03, register entry A18).** The stored receipt is
  bounded (provisionally 64 KiB), versioned, and records the policy in force
  at execution so a later policy change does not rewrite history. The key
  binds the outer invocation.
- **Retention updates (R10-03 with R12-09).** A retention update changes
  how long receipts stay from that point on. It can never delete code, and
  it never re-credits settled fees.
- **Client result types (R10-06, R13-03, R13-11).** The three classes are
  three distinct types in Rust, in the WASM bindings and in the Swift and
  Kotlin mirrors. Converters preserve the optional receipt and policy
  fields, and JSON never collapses a tagged result into an untagged one.
- **No implied services (R10-06).** No client method implies a network
  preflight or a historical proof service. Local fixture simulation is
  allowed and is labelled as such.
- **Tests (R09-08, R10-07).** Vectors cover receipts off, removed under
  retention, retained, tampered, and presented with a wrong root, plus the
  absence case of every receipt query. Cross-language converters reject the
  same malformed vectors and yield the same class for the same input.
