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
    - `actions`: unset for every action, otherwise alternatives, each with:
      - `action`: `CREATE`, `REPLACE`, `DELETE` (also indexOnly deletes), `TRANSFER`,
        `UPDATE_PRICE`, `PURCHASE`
      - `newDocumentWhere`: `CREATE`/`REPLACE` clauses on the document data, typed like
        `getDocuments` V1 where clauses
      - `originalDocumentWhere`: clauses on the document before the transition; only `$id`
        (`EQUAL`/`IN`), except `DELETE` of an indexOnly document type, which carries its values
      - `ownerIds`: `TRANSFER` recipient or `PURCHASE` buyer must be one of these
      - `price`: `UPDATE_PRICE` constraint on the new price
    - `batchOwnerId`: the identity that signs the batch
  - `addresses`: up to 256 platform addresses (21 bytes: type byte then 20-byte hash) and a `role`
  - `identities`: up to 64 identity ids and a `role`
  - `tokens`: up to 64 `tokenIds` and/or up to 64 `identityIds` (with a `role`)
  - `dataContracts`: up to 64 contracts; matches their creation, updates, moderation and fee claims
- `role`: `ANY` (default), `SENDER` (the transition's owner, or an address input it spends),
  `RECIPIENT` (named as beneficiary or target: credit/document/token transfer recipients,
  created or topped-up identities, explicit mint recipients, freeze targets, moderation targets,
  address outputs and change outputs)
- `fromBlockHeight`: first height to scan (inclusive); unset to start after the current tip.
  At most 50,000 blocks behind or 1,000 ahead of the tip.

**Response Parameters**: a stream of one of
- `stateTransition`: `blockHeight`, `blockTimeMs`, `protocolVersion`, `indexInBlock`,
  `stateTransitionHash` (SHA-256 of the bytes, the Tenderdash hash), `stateTransition` (bytes),
  `matchedFilters` (indexes into the request), `matchedBatchPositions` (for batches)
- `checkpoint`: `blockHeight`

The first message is a checkpoint at the height before the first scanned block. A checkpoint
follows every block with a match and repeats every 10 seconds otherwise. A checkpoint at `h`
means every match from the first scanned block through `h` was sent: resume with
`fromBlockHeight = h + 1`.

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
let mut subscription = sdk.subscribe_to_state_transitions(vec![filter], None).await?;
loop {
    match subscription.next().await? {
        SubscriptionEvent::StateTransition(event) => handle(event),
        SubscriptionEvent::Checkpoint { block_height } => save_cursor(block_height + 1),
    }
}
```

## What matching can and cannot see

Filters are evaluated against what a transition itself names, so:
- amounts are the amounts a transition asks for; fees can reduce what an output is credited;
- of a document before a replace, delete, transfer, price update or purchase only the `$id` is
  known, so a query's result set cannot be maintained from transitions alone: track the ids of
  matching documents and subscribe to them by `$id`;
- a mint without an explicit recipient credits the contract's configured destination, which the
  transition does not name; seller proceeds of a purchase, masternode rewards and other credits
  outside a transition are not seen;
- shielded senders and recipients are not public;
- a token transition that only proposes a group action succeeds before the action executes;
- document clauses are evaluated against the data contract's current version, followed through
  updates seen in the stream, so history from before the latest update is read with the newer
  schema (a property generated from others since then is generated for older documents too).

## Trust

Clients can check every delivered transition: its hash, and that it matches their filters
evaluated against proved data contracts (the Rust SDK does). A data contract update in the
stream is only a signal to a client: it reads the contract again with a proof before matching
against the new version, rather than trusting the contract the node sent. They cannot check completeness:
a node may leave a match out, and block heights and times are its assertion. Act on a
transition by reading the state it changed with a proved query; catch up after being offline
from a proved read's `metadata.height + 1`.

## Internal Implementation

DAPI (`rs-dapi`, `services/platform_service/subscribe_to_state_transitions`) serves the stream
from the Tenderdash block store, not Drive. Every subscription walks committed heights in order
with its own cursor; history and new blocks are read the same way, and new-block events only
wake the scan, so a missed event cannot lose a block. A height is never skipped: a block whose
results are not saved yet is retried, and a transaction the node cannot decode or a protocol
version it does not know ends the stream at that height (resume on an upgraded node).
Filters are evaluated by `dash_platform_queries::subscriptions`, the same code the SDK uses.

**Limits** (per node): 1,024 subscriptions, 16 per client address (IPv6 per /64), 8 subscriptions
catching up on history at once; a client that does not read for 60 seconds is dropped with
`RESOURCE_EXHAUSTED`. The gateway ends every stream after 600 seconds; resume from the last
checkpoint.

**Errors**: `INVALID_ARGUMENT` (malformed or undecidable filter), `NOT_FOUND` (unknown data
contract), `OUT_OF_RANGE` (start height outside the window, or a block this node does not keep),
`RESOURCE_EXHAUSTED` (limits), `FAILED_PRECONDITION` (the node cannot decode a block), and
`UNAVAILABLE` (Tenderdash unreachable).
