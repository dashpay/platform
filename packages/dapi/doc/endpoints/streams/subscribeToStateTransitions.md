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
    - `actions`: unset for every action, otherwise up to 12 alternatives, each with:
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
    (an address output or change output it credits)
- `fromBlockHeight`: first height to scan (inclusive); unset to start after the current tip.
  At most 50,000 blocks behind or 1,000 ahead of the tip.

**Response Parameters**: a stream of one of
- `stateTransition`: `blockHeight`, `blockTimeMs`, `protocolVersion`,
  `stateTransitionHash` (SHA-256 of the bytes, the Tenderdash hash), `stateTransition` (bytes),
  `matchedFilters` (indexes into the request), `matchedBatchPositions` (for batches)
- `checkpoint`: `blockHeight`

The first message is a checkpoint at the height before the first scanned block. A checkpoint
follows every block with a match and repeats every 10 seconds otherwise. A checkpoint at `h`
means every match from the first scanned block through `h` was sent: resume with
`fromBlockHeight = h + 1`. Any end of the stream, including one with status `OK` (for example
at the gateway's 600-second limit), means the same: resume from the last checkpoint.

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
- a filter is bound to the contract version current at subscribe time, which later updates
  cannot invalidate from protocol 14 on (on earlier protocol versions an update that widens an
  integer field may need a resubscribe).

## Trust

The stream is the node's assertion. A client can check that a delivered transition's hash
matches its bytes, but not that the transition was committed or that nothing was left out, and
block heights and times are the node's word. Delivery is at least once. Act on a transition by
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

**Limits** (per node): 256 subscriptions, 16 per client address (IPv6 per /48), 8 Tenderdash
RPCs in flight across all subscriptions; a client that does not read for 60 seconds is
dropped with `RESOURCE_EXHAUSTED`. The gateway ends every stream after 600 seconds; resume from the last
checkpoint.

**Errors**: `INVALID_ARGUMENT` (malformed or undecidable filter), `NOT_FOUND` (unknown data
contract), `OUT_OF_RANGE` (start height behind the replay window: catch up with a proved read
instead), `RESOURCE_EXHAUSTED` (limits), `FAILED_PRECONDITION` (this node does not keep or cannot
decode a block; another node may), and `UNAVAILABLE` (Tenderdash unreachable, or the start height
is ahead of this node's tip).
