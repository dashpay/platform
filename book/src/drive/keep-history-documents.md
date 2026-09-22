# Keep-History Documents

> **Status:** implemented and gated at protocol version 15 (meta-schema v4 /
> parser generation 4). The storage layout, the delete and erase paths and
> the fee accounting are pinned against a real grovedb by
> [`lifecycle/tests.rs`](https://github.com/dashpay/platform/blob/v4.3-dev/packages/rs-drive/src/drive/document/lifecycle/tests.rs)
> in rs-drive; the full ABCI pipeline (transitions, validation, proofs,
> refunds) by the `keep_history`, `deletion`, `erase` and
> `lifecycle_contracts` modules under rs-drive-abci's batch tests; and the
> whole story against a running network by
> [`KeepHistoryDocument.spec.js`](https://github.com/dashpay/platform/blob/v4.3-dev/packages/platform-test-suite/test/functional/platform/KeepHistoryDocument.spec.js).

A document type declared with `documentsKeepHistory: true` retains every
revision of every document instead of overwriting the previous one. This
chapter is the reference for how those revisions are stored, how a
keep-history document is deleted and later erased, what the four lifecycle
states mean, and how `getDocumentHistory` reports all of it.

## Storage layout

Every document type owns three reserved single-byte keys under its
document-type tree. Index trees are keyed by property names, which cannot
start below `0x30`, so no index can collide with them.

```text
[DataContractDocuments, contract_id, 1, <doctype>, 0, <document id>]
    → Reference to the current revision      (the primary-key tree)

[DataContractDocuments, contract_id, 1, <doctype>, 1, <document id>]
    → Item(<lifecycle record>)               (the lifecycle tree)

[DataContractDocuments, contract_id, 1, <doctype>, 2, <document id>]
    → ProvableCountTree                      (the history tree)
        [<block time ms ‖ revision>] → Item(<serialized document>)
```

For a type that does not keep history the primary-key tree holds the
serialized document itself and keys `1` and `2` do not exist. For a
keep-history type:

- **The history tree** holds every retained revision of one document, keyed
  by the block time it was written at followed by its revision, both
  big-endian `u64`. Two revisions written in one block therefore stay
  distinct. It is a provable count tree, so how many revisions a document
  retains is a single hash-bound read, and its absence is provable.
- **The primary-key entry** is a reference to the newest revision, so every
  ordinary by-id read and every secondary index resolve through it. For a
  summable type the reference also carries the document's sum contribution.
- **The lifecycle record** exists exactly while the document is deleted or
  erasing. It is what tells a deleted document apart from an id that was
  never used: both are absent from the primary-key tree. The record is
  `DocumentLifecycleRecord` in `rs-dpp` (`packages/rs-dpp/src/document/lifecycle`),
  a versioned enum with the ordinary platform-serialization derives, stored
  with the storage flags of the identity that deleted the document.

The record carries the block time the document was deleted at, the revision
it carried then, how many revisions the history retained at that moment,
and, once an erasure has been authorized, the block time the erasure started
and the newest revision that was retained when it started. The revision
count matters because a history written before protocol 15 can have gaps
(two writes in one block overwrote each other's revision); a by-revision
read is only meaningful when the retained revisions number one through the
deleted revision without a gap, and the record is where that is decided.

The reserved key `1` and the whole lifecycle tree are per type and
unflagged, so no single deleter pays for structure nobody refunds; each
record inside it carries the deleter's flags and is what an erase refunds.

## Contract grammar

Protocol 15's parser generation 4 adds two rules to keep-history types:

| Keyword | Meaning | Constraints |
|---|---|---|
| `canBeDeleted: true` | The owner may delete a document. Previously refused on a keep-history type, since the storage layer could not remove one. | Ordinary keyword, now allowed together with `documentsKeepHistory`. |
| `canBeErased: true` | The retained revisions of a deleted document may be purged. | Requires `documentsKeepHistory` and `canBeDeleted`; defaults to `false`; immutable across contract updates. |

Declaring `canBeErased` on a type that keeps no history, or whose documents
can never be deleted, is a contract-structure error rather than an ignored
default: a type whose declared behaviour and reachable behaviour disagree
is an authoring mistake. The check runs even without full validation,
because the flag governs an irreversible operation.

A keep-history type may **not** carry a contested index. A contested
resource is awarded by block processing, outside transition validation, at
an id derived from the winner rather than from the contested values, and on
a keep-history type that award could land on an id whose retained history
already exists. Lifting this needs changes to the contested machinery
itself; until then the parser refuses the combination.

Types registered under protocol 14 or earlier keep their frozen grammar: a
keep-history type that allows deletion is still refused by the generation 3
parser, so nothing that exists today can enter the lifecycle without a
contract update at protocol 15.

## The four lifecycle states

```text
             delete                erase (first chunk,        erase (terminal chunk)
   Active ──────────► Deleted ──────────────────────► Erasing ──────────────────────► Absent
                         │        by the owner)                 by anyone
                         │
                         └── erase of a history that fits in one chunk goes straight to Absent
```

| State | Primary-key entry | Lifecycle record | History tree |
|---|---|---|---|
| **Active** | reference | none | every revision |
| **Deleted** | none | deleted-at only | every revision retained at deletion |
| **Erasing** | none | deleted-at and erasing-from | shrinking, newest revisions removed first |
| **Absent** | none | none | none; the id is free again |

`DocumentLifecycleState` in `packages/rs-drive/src/drive/document/lifecycle/fetch`
is the one read that classifies a document. It is deliberately not folded
into the by-id fetch every document action performs: only stateful
validation of an action on a keep-history type needs the extra reads, and
they are ordered by how likely they are to settle the question. The
ordinary by-id read answers for every live document; only a miss pays for
the lifecycle record; only a deleted document pays a third read, for the
newest revision it retains, which is where its owner is read from.

## Delete

A delete on a keep-history document removes the primary-key reference and
every index reference, and writes the lifecycle record in their place. The
revisions are untouched, so `getDocumentHistory` keeps serving them.

Two properties follow from the design:

- **Delete never escalates.** It has no code path that removes a revision,
  so a second delete of the same document is a paid consensus error
  (`DocumentNotFoundError`, exactly as for an id that holds nothing) rather
  than a deeper removal.
- **A deleted document keeps its id.** Creating over it is refused in
  transition validation and, independently, in the storage writer:
  appending to a history that still retains revisions would silently merge
  a new document into a deleted one's record, and the writer's check is
  what covers callers that never pass through validation.

Every Drive delete entry point takes the block the delete belongs to and the
deleter's identity, because the record has to name both.

## Erase

An erase purges the retained revisions of a document that has already been
deleted. It is a separate transition kind, `DocumentTransition::Erase`,
appended to the shipped enum the same way every earlier kind was, carrying
nothing beyond the base transition. Its structure validation requires the
type to keep history and to declare `canBeErased`; its state validation
depends on the lifecycle state:

| Lifecycle state | Result |
|---|---|
| Active | refused; deleting is a separate intent with its own permission and cost |
| Deleted | allowed only for the document's owner, read from the newest retained revision |
| Erasing | allowed for **any** identity |
| Absent | `DocumentNotFoundError` |

**Erasure is authorized once.** The owner commits the document to erasure
with the first chunk; the record that commitment leaves in state is then the
evidence that destruction was authorized, so any identity may submit the
remaining chunks and pay for them. An owner who loses their keys, their
funds or their permission cannot strand a half-erased document. For the same
reason an erase carries no token payment: the deletion it follows was
charged when the document was deleted, and a payment on a transition anyone
may submit would let a continuation move tokens.

**Chunks.** One transition removes at most
`max_document_revisions_erased_per_transition` revisions (100 from
`SYSTEM_LIMITS_V5`), newest first, so a partial erasure leaves the oldest
content behind and the retained sequence stays contiguous from one. The
enumeration reads one revision more than it may remove so that it knows,
before emitting anything, whether this chunk is the last one:

- a **non-terminal first chunk** overwrites the record with the erasure it
  authorizes;
- a **terminal chunk** removes the record and the now empty history subtree,
  which GroveDB accepts only because the revision deletes are in the same
  batch and refuses otherwise.

The two never happen together, so one batch never carries two operations on
the record's key.

**Accounting.** Removing a revision credits whoever paid for it, through the
storage flags every revision carries, in balance updates applied after the
transition's fee result is formed. The admission estimate cannot know how
many revisions a document retains, so it prices a full chunk of the type's
largest documents plus both endings; every erase is admitted against the
same worst-case estimate and the actual fee is what it removed. GroveDB's
query surface has no key-only result shape over a range, so the enumeration
reads revision bodies it discards, and the estimate charges for that read
honestly.

## The `getDocumentHistory` query

The query reads a page of a keep-history document's retained revisions
together with the document's lifecycle as of the same block.

**Request.** A contract id, document type name and document id, an optional
`limit` (at most 10, the default), and exactly one selector:

| Selector | Meaning |
|---|---|
| `startAtMs` | inclusive lower bound on block time |
| `startAfter { timeMs, revision }` | exclusive cursor returned with the previous page; the revision is needed because several revisions can share a block time |
| `startAtRevision` | inclusive lower bound on revision; refused on a history with gaps |
| `revision` | exactly one revision; requires limit one |

**Response.** A page of entries, each the block time, revision and
serialized document, and a `Lifecycle` message:

| Field | Meaning |
|---|---|
| `state` | `ACTIVE`, `DELETED`, `ERASING` or `ABSENT` |
| `remaining_revisions` | how many revisions the history still retains |
| `deleted_at_ms` | zero unless the document has been deleted |
| `erasing_started_at_ms` | zero unless an authorized erasure has started |
| `erasing_from_time_ms`, `erasing_from_revision` | the newest revision retained when the erasure started |

The state is derived identically in Drive's fetch and in the proof verifier,
and the verifier checks every claimed field against the proof.

**Proof.** The proved response carries one proof object whose grovedb proof
holds two GroveDB proofs in a small versioned envelope, because they answer
two queries GroveDB cannot merge: the offset-paginated proof over the
document's history tree, and the exact-key proof over the current pointer,
the lifecycle record and the history tree's count. Both commit to the same
root hash, signed once.

## Client surfaces

**Rust SDK.** `DocumentHistory::fetch` with a `DocumentHistoryQuery` from
`dash-platform-queries` reads a page. `DocumentEraseTransitionBuilder` and
`Sdk::document_erase` submit one chunk; the result is named for what its
proof authenticates, which is that the document is absent by id, something
that was already true before the erase ran. That is why the call takes the
affected-state wait rather than the strict one. `Sdk::document_current_lifecycle`
is the separately named read for how much history is left.

**JavaScript.** The evo-sdk `documents` facade gains `history()`,
`historyWithProof()` and `erase()`; the WASM SDK exposes them as
`getDocumentHistory`, `getDocumentHistoryWithProofInfo` and `documentErase`.
Every history selector is an exact `u64`: a `number` is accepted only up to
`Number.MAX_SAFE_INTEGER`, anything larger must be a `bigint`.

```typescript
// Delete, then erase in chunks until the history is gone. The first erase
// is signed by the owner; later ones may be signed by any identity.
await sdk.documents.delete({ document, identityKey: ownerKey, signer });

let lifecycle;
do {
  await sdk.documents.erase({
    document: { id, ownerId, dataContractId, documentTypeName },
    identityKey: ownerKey,
    signer,
  });
  ({ lifecycle } = await sdk.documents.history({
    dataContractId, documentTypeName, documentId: id, startAtMs: 0n, limit: 1,
  }));
} while (lifecycle?.state === 'ERASING');
```

## Versioning

Everything above is selected only from `PLATFORM_V15`. Released tables gain
dormant slots, and protocols 12 through 14 replay unchanged: a keep-history
delete still ends in `InvalidDeletionOfDocumentThatKeepsHistory` at 12 and
13 and in a paid rejection at 14, and an erase transition is refused at
basic-structure validation below 15 by its per-kind bounds slot. A
keep-history type registered under protocol 14 can never become deletable,
so the migrated storage layout and the lifecycle are only ever exercised
together on types updated at 15.

## Rules and guidelines

**Do:**
- Read a keep-history document's state through `fetch_document_lifecycle`
  whenever the difference between deleted and never-existed matters. An
  ordinary by-id read cannot tell them apart.
- Pass the block and the deleter to every Drive delete entry point; the
  record needs both.
- Treat an erase's proof as an observation of the affected state and read
  the lifecycle separately to learn what is left.

**Do not:**
- Declare `canBeErased` without `documentsKeepHistory` and `canBeDeleted`,
  or a contested index on a keep-history type. The parser refuses both.
- Add a token cost to an erase. Any identity may submit a continuation.
- Rely on a by-revision selector over a history written before protocol 15
  without checking `remaining_revisions` against the deleted revision; a
  gapped history refuses it.
