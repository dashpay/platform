# subscribeToStateTransitions

## Client API

Streams committed, successfully executed state transitions that match any of the request's
filters, block by block, from a chosen height onward.

**Request Parameters**:
- `filters`: 1 to 16 filters; a transition is delivered when it matches **any** of them, and
  every constraint within one filter must hold. One of:
  - `documents`: document transitions (in batch transitions) on one data contract
    - `dataContractId` (required)
    - `documentTypeName`: unset for every document type (actions may then carry no clauses)
    - `actions`: unset for every document transition, including an erase (the purge of a
      deleted document's retained revisions), which no action names; otherwise up to 12
      alternatives, none of which delivers an erase, each with:
      - `action`: `CREATE`, `REPLACE`, `DELETE` (also indexOnly deletes), `TRANSFER`,
        `UPDATE_PRICE`, `PURCHASE`
      - `newDocumentWhere`: `CREATE`/`REPLACE` clauses on the document data, typed like
        `getDocuments` V1 where clauses (up to 16)
      - `originalDocumentWhere`: clauses on the document before the transition; only `$id`
        (`EQUAL`/`IN`), except `DELETE` of an indexOnly document type, which carries its values
        (up to 16)
      - `ownerIds`: `TRANSFER` recipient or `PURCHASE` buyer must be one of these (up to 64)
      - `price`: `UPDATE_PRICE` constraint on the new price
    - `batchOwnerId`: the identity that signs the batch
  - `addresses`: up to 256 platform addresses (21 bytes: type byte then 20-byte hash) and a
    `role`: `ANY` (default), `SENDER` (an address input the transition spends) or `RECIPIENT`
    (an address it names as an output or change output; it is credited what execution leaves
    for it, which can be nothing: a shield from an asset lock names where its surplus goes and
    may leave none)
- `fromBlockHeight`: first height to scan (inclusive); unset to start after the current tip.
  At most 50,000 blocks behind or 1,000 ahead of the node's tip. A node asked for a height
  above the block after its tip scans from that block: a checkpoint says every height up to it
  was scanned, and the node does not say that of a height it has not reached. So a client
  that resumes on a node a few blocks behind misses nothing; it can see matches again.

**Response Parameters**: a stream of one of
- `stateTransition`: `blockHeight`, `blockTimeMs`, `protocolVersion`,
  `stateTransitionHash` (SHA-256 of the bytes, the Tenderdash hash), `stateTransition` (bytes),
  `matchedFilters` (indexes into the request), `matchedBatchPositions` (for batches)
- `checkpoint`: `blockHeight`

The first message is a checkpoint at the height before the first scanned block: at
`fromBlockHeight - 1`, or lower when the node starts at its tip. A checkpoint
follows every block with a match and repeats every 10 seconds otherwise. A checkpoint at `h`
means every match from the first scanned block through `h` was sent: resume with
`fromBlockHeight = h + 1`. Any end of the stream, including one with status `OK` (for example
at the gateway's 600-second limit), means the same: resume from the highest checkpoint.

The height to resume from only moves up: it is the greater of the one the client holds
(the height it first asked to start at included) and a received checkpoint plus one. A stream
resumed on a node that starts at its tip opens with a checkpoint below the one the client
holds, and repeats matches the client may have. Resuming from that lower checkpoint could ask
for a height nodes no longer replay. The Rust SDK does this: the checkpoints it gives an app
never go down.

**Example Usage** (Rust SDK):
```rust
use dash_sdk::dash_platform_queries::subscriptions::{
    DocumentAction, DocumentActionMatch, DocumentFilter, StateTransitionFilter,
};
use dash_sdk::platform::subscriptions::SubscriptionEvent;

let filter = StateTransitionFilter::Documents(
    DocumentFilter::new(contract_id)
        .with_document_type("contactRequest")
        .with_action(DocumentActionMatch::new(DocumentAction::Create).with_new_document_where(
            WhereClause { field: "toUserId".into(), operator: WhereOperator::Equal, value: my_id.into() },
        )),
);
let mut subscription = sdk.subscribe_to_state_transitions(vec![filter], None, None).await?;
loop {
    match subscription.next().await? {
        SubscriptionEvent::StateTransition(event) => handle(event),
        SubscriptionEvent::Checkpoint { block_height } => save_cursor(block_height + 1),
    }
}
```

## What matching can and cannot see

Filters are evaluated against what a transition itself names, so:
- of a document before a replace, delete, transfer, price update or purchase only the `$id` is
  known, so a query's result set cannot be maintained from transitions alone: track the ids of
  matching documents and subscribe to them by `$id`;
- shielded senders and recipients are not public;
- a transition that failed is never delivered, even one that was charged for. Such a transition
  can still change the balance of an address it names: the fee taken from an address input, or
  the address credited when an identity cannot be created from the shielded pool. A wallet
  learns of those from `getRecentAddressBalanceChanges` or a proved balance read, not from this
  stream;
- a filter is bound to the contract version current at subscribe time, which later updates
  cannot invalidate from protocol 14 on (on earlier protocol versions an update that widens an
  integer field may need a resubscribe).

## Trust

The stream is the node's assertion. A client can check that a delivered transition's hash
matches its bytes, but not that the transition was committed or that nothing was left out, and
block heights and times are the node's word. A checkpoint is too. One that names a height the
chain had not reached hides blocks until a stream reaches that height: each stream opened
meanwhile starts at its node's tip, so what is committed between two streams is scanned by
neither, and nothing below the tip of the first of them is. How many blocks that is depends on
how far above the chain the checkpoint was; the 1,000 blocks a node lets a start be above its
tip limit which starts it admits, not this. No honest node sends such a checkpoint; it is one
way a node leaves matches out. Delivery is at least once. Act on a transition by
reading the state it changed with a proved query; catch up after being offline from a proved
read's `metadata.height + 1`.

## Internal Implementation

DAPI (`rs-dapi`, `services/platform_service/subscribe_to_state_transitions`) serves the stream
from the Tenderdash block store, not Drive. Every subscription walks committed heights in order
with its own cursor, up to the highest height whose execution results are saved; history and new
blocks are read the same way, and new-block events only wake the scan, so a missed event cannot
lose a block. A height is never skipped: a transaction the node cannot decode, a protocol version
it does not know or a pruned height ends the stream there (resume on an upgraded or archival
node). Filters are resolved once, against the data contracts Drive holds when the subscription
starts, by `dash_platform_queries::subscriptions`.

Reads and matching are bounded work. A Tenderdash call keeps its place among the 8 in flight
until Tenderdash has answered it, whether or not a stream still waits for it; a stream waits
30 seconds for a read, from when it asks, and then ends. Blocks are decoded and matched on the
blocking pool, and a subscription holds a decoded block only while it has one of two places
to match it. While the node cannot read its chain tip for 30 seconds,
streams that have caught up end and new ones are refused: a checkpoint then would say the
chain is quiet, which the node does not know.

**Limits** (per node): 256 subscriptions, 16 per client address (IPv6 per /48), 8 Tenderdash
RPCs in flight across all subscriptions; a client that does not read for 60 seconds is
dropped with `RESOURCE_EXHAUSTED`. The gateway ends every stream after 600 seconds; resume from the
highest checkpoint.

**Errors**: `INVALID_ARGUMENT` (malformed or undecidable filter), `NOT_FOUND` (unknown data
contract), `OUT_OF_RANGE` (start height behind the replay window: catch up with a proved read
instead), `RESOURCE_EXHAUSTED` (limits), `FAILED_PRECONDITION` (this node no longer keeps a block, has
not reached the start height yet, or cannot decode a block; another node may serve it), and
`UNAVAILABLE` (the node cannot read from Tenderdash: a read failed or took too long, or the chain
tip could not be read). A client moves to another node on both; only `UNAVAILABLE` says the node
itself is failing.
