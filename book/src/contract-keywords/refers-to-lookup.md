# Finding a Document by Its Properties (findBy)

`findBy` lets a document reference find its target by the values of the target's properties instead of by its id. The property's value is then one part of a key, the rest comes from the referring document or its writer, and the reference holds if the referenced document type's unique index over exactly those properties finds a document. Reach for it when the natural value is an identity (a member, an author, a recipient) and the rule is "this identity has a document of that kind": with an id reference the writer would have to find the document's id first, and the id would say nothing about who it belongs to.

| | |
|---|---|
| **Where** | Inside a `permanentDocument` or `deletableDocument` [`refersTo`](refers-to.md) declaration: on an identifier property, on the `items` of a typed array of identifiers, as an operand of an [expression](refers-to-expressions.md), or in [`ownerRefersTo` and `creatorRefersTo`](owner-refers-to.md). Also on a string or byte array property, whose value `findBy` reads as is (see [A string or byte array key](#a-string-or-byte-array-key)) or through a [function](#commit-and-reveal). |
| **Value** | An object of 1 to 10 entries `{ "<referenced property>": <source> }`. Each key is a property of `documentType`, by its name there (system ones such as `$ownerId` included). Each source is `"."` (the reference's own value), `"$ownerId"` (the writer), a property path of the referring document type, or at most one [function](#commit-and-reveal). The keys must be exactly the properties of one unique index of `documentType`, in any order. Beside a function the `refersTo` may also declare `consume` and `constraints`, holding `minimumAgeBlocks` and [`chooseFirst`](#choosing-the-first-commitment-choosefirst). With [`inList`](refers-to-list-element.md), `findBy` is `{ "$id": <property> }` instead. |
| **Since** | protocol version 14 |
| **On update** | Fixed, like the rest of `refersTo`: adding, removing or changing `findBy` is refused (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `ReferencedEntityNotFoundError` (40120) when no document is found; at registration `ReferencedDocumentLookupInvalidError` (40137) for a document type of another contract, `InvalidContractStructure` (10231) for one of the same contract. A function adds `DocumentReferencePreimageInvalidError` (10423) and `ReferencedDocumentRequirementNotMetError` (40142), and `chooseFirst` `ReferencedDocumentPrecededError` (40149). |

## Example

The moderation charters contract's `joinRequest` type, which is immutable and can never be deleted, has this unique index: one join request per proposal and owner.

```json
"indices": [
  {
    "name": "bySubmittedCharter",
    "properties": [{ "submittedCharterId": "asc" }, { "$ownerId": "asc" }],
    "unique": true
  }
]
```

The same contract's `electedCharter` lists its team in `members`, each of whom must have asked to join:

```json
"members": {
  "type": "array", "minItems": 0, "maxItems": 15, "uniqueItems": true,
  "items": {
    "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
    "contentMediaType": "application/x.dash.dpp.identifier",
    "distinctFrom": "$ownerId",
    "refersTo": {
      "type": "permanentDocument",
      "documentType": "joinRequest",
      "findBy": { "submittedCharterId": "submittedCharterId", "$ownerId": "." }
    }
  },
  "position": 2
}
```

Every member must be the owner of a `joinRequest` whose `submittedCharterId` equals this elected charter's `submittedCharterId`. `findBy` names `submittedCharterId` and `$ownerId`, exactly the properties of `bySubmittedCharter`, so that index finds the join request. For each element the key is `submittedCharterId` read from the elected charter and `$ownerId` filled with the element itself (`"."`). The same form works on a single identifier property, where `"."` is the property's value.

Read as SQL, a reference is a query that must return a row. `documentType` is the `FROM`, `findBy` the part of the `WHERE` that finds the row through a unique index, [`where`](refers-to.md#where) the rest of the `WHERE`, checked on the row found, `constraints` further conditions on it (`minimumAgeBlocks` an age, `chooseFirst` a precedence) and `consume` a delete of the row. The declaration above reads, for each element:

```sql
SELECT * FROM joinRequest
WHERE submittedCharterId = :submittedCharterId  -- findBy, from the elected charter
  AND $ownerId = :element                       -- findBy, "."
```

## How it works

### Which index finds the document

`findBy` does not name an index. The index is the unique index of the referenced document type whose properties are exactly the properties `findBy` names, in any order. It must not bucket its first property with `timeRange` or `integerRange`, and the referenced type must not be `indexOnly`. A type's indexes can never be added, removed or changed by a contract update, so the same `findBy` always resolves to the same index.

Registration refuses a `findBy` that names no such index, listing the unique indexes the type has. Into the `joinRequest` type above, `{ "submittedCharterId": "submittedCharterId" }` leaves out `$ownerId`:

```text
"joinRequest" has no unique index over exactly (submittedCharterId): findBy must name every
property of one of its unique indexes and nothing else (bySubmittedCharter (submittedCharterId, $ownerId))
```

A `findBy` naming exactly the properties of an index that is not unique is refused too, since it could find several documents. `joinRequest` also has a `byOwner` index on `$ownerId` alone, which is not unique, so `{ "$ownerId": "." }` is refused:

```text
index "byOwner" of "joinRequest" over ($ownerId) is not unique: findBy must find at most one document
```

### Assembling the key

When the referring document is created or replaced, a key is assembled for each value (each element of a typed array), one part per entry of `findBy`:

- `"."`: the value being checked, the property's value (a [string or byte array](#a-string-or-byte-array-key) one included) or the element.
- `"$ownerId"`: the referring document's owner, the writer.
- a property path (`"submittedCharterId"`, `"meta.charterId"`): the referring document's value at that path.
- a [function](#commit-and-reveal): the hash of the params it reads.

The index is queried for at most one document, billed as a document fetch. If it finds one, the reference holds, and any [`where`](refers-to.md#where) entries beside `findBy` are checked against that document (`ReferencedDocumentPropertyMismatchError`, 40127). If it finds none, the write is refused with `ReferencedEntityNotFoundError` (40120) naming the property, or the element by its list path (`members[1]`); the error's target reads "found by" and the properties `findBy` names.

### Permanent and deletable references

A `permanentDocument` found by `findBy` never dangles. Its referenced documents are never deleted, and registration makes sure the key of every one of them is fixed once written (see [below](#rules-at-registration)), so the document a key found stays there. A replace checks it again only:

- when the property itself changed (for a typed array, only the elements the stored list did not hold);
- when a property `findBy` reads changed, and then every value, every element included;
- when the referring value of a `where` entry changed, or on every replace for a `where` entry whose value is `"$ownerId"`.

A key part read from `"$ownerId"` never triggers a check: only a type whose documents keep their writer may read it.

A `deletableDocument` found by `findBy` promises less. Once the document a key found is deleted, a new document filed later under the same key makes the reference hold again, with different content, where an id reference to a deleted document stays dead. So it means "a document with this key exists now", which is what a membership gate needs. Every replace checks it again, whether or not the replace touched it, and no `immutable` property may hold one. The moderation charters contract's `resignationRequest` uses one to accept a writer who has an `addedModerator` document for the charter at the time of writing, as the alternative to being an elected member (see [Writer and Creator References](owner-refers-to.md)).

`findBy` can also reach into another contract. This `recipientId` must be an identity that has a DashPay profile when the document is written:

```json
"recipientId": {
  "type": "identifier",
  "refersTo": {
    "type": "deletableDocument",
    "contractId": "Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7",
    "documentType": "profile",
    "findBy": { "$ownerId": "." }
  },
  "position": 0
}
```

DashPay's `profile` type has a unique index on `$ownerId` alone, which finds the profile, and profiles can be deleted, so the reference is a `deletableDocument` one.

### A string or byte array key

The value carrying the reference need not be an identifier. A string or byte array property may carry a `refersTo` found by `findBy` too, its value read as is: `"."` fills an index property holding the same kind of value, a string a string and a byte array a byte array. A `post` naming its author by handle, where `handle` is unique on its `name`:

```json
"authorHandle": {
  "type": "string",
  "maxLength": 63,
  "refersTo": {
    "type": "permanentDocument",
    "documentType": "handle",
    "findBy": { "name": "." }
  },
  "position": 0
}
```

The property keeps its type, and the reference is judged as an identifier's is: on a create, and on a replace by the rules above (when the value changes, when a property `findBy` reads changes, and on every replace for a `deletableDocument`). The key is the value byte for byte, so `"Alice"` does not find the handle `alice`: store a normalized value where the referenced index holds one. The document found must hold exactly that value: the index stores the empty string under the key of `"\0"` and an empty byte array under the key of no value, and neither stands in for the other. A value longer than an index key may be (255 bytes) finds nothing, without a read: the referring property may allow longer values than the referenced one, and no document of the referenced type holds one. The value is no id, so a refusal (`ReferencedEntityNotFoundError`, 40120) names it by its path, with an all-zero entity id. A [function](#commit-and-reveal) may read such a property instead, by its path among its params.

### What findBy cannot do

The value of a reference found by `findBy` is a key part, not a document id. So such a reference cannot be the join property of a chained query or of a composite join by id, and a `preallocated` index is never bound through one (see [Index-Only Types](index-only.md)). It counts as one reference per value against the [reference budget](refers-to.md#the-reference-budget), like any other.

## Commit and reveal

One `findBy` entry may hold a function: `"<referenced property>": { "function": "sys.hash.sha256d", "params": [...] }` says the referenced document's property holds the SHA-256 of the SHA-256 of the params' bytes, joined in order. The platform fills that property of the key with the hash to find the document, like any other entry. The document it finds is a commitment made earlier, one that stored that hash, so the document being created may exist only while a commitment to values it carries exists. This is how a name registration that preorders a salted hash works, written as a declaration on the salt:

```json
"preorderSalt": {
  "type": "bytes", "size": 32, "position": 4,
  "refersTo": {
    "type": "deletableDocument",
    "documentType": "preorder",
    "findBy": {
      "saltedDomainHash": {
        "function": "sys.hash.sha256d",
        "params": ["preorderSalt", "normalizedLabel", { "const": "." }, "parentDomainName"]
      }
    },
    "where": { "$ownerId": "$ownerId" },
    "constraints": { "minimumAgeBlocks": 1 },
    "consume": true
  }
}
```

The salt must reveal a `preorder` whose `saltedDomainHash` is the sha256d of the salt, the normalized label, a dot and the parent. `findBy` names `saltedDomainHash` alone, the whole of the `preorder` type's unique `saltedHash` index. The `where` entry makes it the writer's own preorder, `minimumAgeBlocks: 1` one created in an earlier block, and `consume: true` deletes it with the create. As SQL:

```sql
SELECT * FROM preorder
WHERE saltedDomainHash =                                     -- findBy
      sha256d(:preorderSalt || :normalizedLabel || '.' || :parentDomainName)
  AND $ownerId = :writer                                     -- where
  AND $createdAtBlockHeight <= :createHeight - 1             -- minimumAgeBlocks
-- then the create deletes the row found                     -- consume
```

- **Params.** A property path reads the document being created, the property carrying the reference included; `{ "const": text }` is fixed text of 1 to 64 bytes; `"."` is the value carrying the reference where it has no path (each element of a typed array, the writer, the creator); `"$ownerId"` is the writer's id; 1 to 16 params. Strings count as their UTF-8, byte arrays as their bytes, identifiers as their 32 bytes. Two values of no fixed length must be separated by a one-byte `const`, and a value holding that byte is refused, so the joined bytes split back one way only.
- **One function.** `findBy` holds at most one function, and the property it fills must be a byte array of exactly 32 bytes.
- **Carrier.** A string or byte array property may carry a `refersTo` this way, or with its value read as is (see [A string or byte array key](#a-string-or-byte-array-key)), and keeps its type. The value carrying the reference fills the key exactly once, as a `"."` entry or a param; a param names a property by its path, never as `"."`. In `ownerRefersTo` and `creatorRefersTo` the value may be left out beside a function.
- **Judged once.** A function's key is checked when the document is created, never on a replace, so every stored value it reads must be fixed once written, the carrier included (listed under `immutable` on a mutable type, and required when it is a `deletableDocument` reference by id, which a replace may otherwise clear once its document is deleted). The `where` entries are judged with it, so each referring property they name must be fixed once written too, or transient. On a type whose documents can be replaced it may not be an operand of an `anyOf`, which would then hold on every replace. Params may be transient: they are read from the create. The hash is billed as the double SHA-256 it is, by the blocks it hashes, beside the document fetch.

`constraints` (holding `minimumAgeBlocks` and [`chooseFirst`](#choosing-the-first-commitment-choosefirst)) and `consume` sit on the `refersTo`, beside `findBy`: they describe the document `findBy` finds when the referring document is created, so the reference must be judged on the create alone. A function in `findBy` makes it so, and so does a referring type whose documents are never replaced (`documentsMutable: false`): a replace is the one write that checks a reference again. Any other `findBy` declaring either is refused at registration (10231), and so is either beside [`inList`](refers-to-list-element.md), which checks that a list holds the value rather than finding a document by a key.

- **`constraints.minimumAgeBlocks`**: the commitment's `$createdAtBlockHeight` is at least that many blocks below the create's height. The commitment type must list `$createdAtBlockHeight` in `required`.
- **`consume: true`**: the create deletes the commitment, its storage refunded to its owner. Only on a `deletableDocument` reference with the `where` entry `"$ownerId": "$ownerId"`, into a type of the same contract whose owner may delete its documents (`canBeDeleted: true`) or whose documents only a consume deletes (`canBeDeleted: "onlyWhenConsumed"`, see [Deletion](deletion.md#canbedeleted)), that declares no delete token cost or delete action fee and no [`deleteConstraints`](deletion.md#deleteconstraints) (no delete transition charges or judges them), and that requires no stricter signature security level than the revealing type. A contract-bound key signing the create must be allowed to act on the consumed type too (`ContractBoundedKeyOutOfBoundsError`, 20014).

A create missing a param, or with a value holding its separator, is refused before any read (`DocumentReferencePreimageInvalidError`, 10423). No commitment is `ReferencedEntityNotFoundError` (40120), another identity's `ReferencedDocumentPropertyMismatchError` (40127), and one too young `ReferencedDocumentRequirementNotMetError` (40142). The [Documents chapter](../data-model/documents.md#commit-and-reveal-a-findby-function) has the full rules.

### Choosing the first commitment (chooseFirst)

Once a create reveals a commitment, anyone reading the pending create learns the values it reveals. Where blocks are full and fees order the mempool, another identity could then commit to the same values, pay more to land its commitment, and reveal in the next block, before the first committer's reveal lands. The `chooseFirst` constraint orders the commitments to the same values and lets only the first valid one be revealed: a create is refused while a valid commitment of another identity comes before the one it found. Ordered by creation height, the commitment that lands in the earliest block wins.

It sits in the `constraints` of a `deletableDocument` `refersTo` whose `findBy` holds exactly `"$ownerId": "$ownerId"` and a function of the revealed values, into a type whose documents expire (`ttl`). A name registration whose preorders carry two hashes reads:

```json
"preorder": {
  "type": "object",
  "documentsMutable": false,
  "canBeDeleted": true,
  "ttl": 86400,
  "properties": {
    "ownerNameHash": { "type": "bytes", "size": 32, "position": 0 },
    "nameHash": { "type": "bytes", "size": 32, "position": 1 }
  },
  "indices": [
    {
      "name": "ownerAndName",
      "properties": [{ "$ownerId": "asc" }, { "nameHash": "asc" }],
      "unique": true
    },
    { "name": "nameHashByHeight", "properties": [{ "nameHash": "asc" }, { "$createdAtBlockHeight": "asc" }] }
  ],
  "required": ["$createdAt", "$createdAtBlockHeight", "ownerNameHash", "nameHash"],
  "additionalProperties": false
}
```

and the `domain`, which can be transferred, declares on its creator:

```json
"creatorRefersTo": {
  "type": "deletableDocument",
  "documentType": "preorder",
  "findBy": {
    "$ownerId": "$ownerId",
    "nameHash": {
      "function": "sys.hash.sha256d",
      "params": ["normalizedLabel", { "const": "." }, "normalizedParentDomainName"]
    }
  },
  "constraints": {
    "minimumAgeBlocks": 1,
    "chooseFirst": {
      "among": "nameHash",
      "validWhen": {
        "ownerNameHash": {
          "function": "sys.hash.sha256d",
          "params": ["$ownerId", "normalizedLabel", { "const": "." }, "normalizedParentDomainName"]
        }
      },
      "by": ["$createdAtBlockHeight"]
    }
  }
}
```

`findBy` finds the writer's own preorder of the name, from an earlier block, by the hash of the name, which every preorder of the name holds, whoever made it. `among` names that property: the preorders holding the same value there are the group the found one is chosen among. `validWhen` says when a preorder of the group counts: it holds, at `ownerNameHash`, the hash of its own owner's id and the name, which only an identity knowing the name can compute (`"$ownerId"` there stands for each preorder's owner). `by` orders the group. When the domain is created, the platform checks the found preorder and then reads:

```sql
-- the found preorder must be valid itself:
--   :found.ownerNameHash = sha256d(:writer || :normalizedLabel || '.' || :normalizedParentDomainName)
SELECT * FROM preorder AS c
WHERE c.nameHash = :found.nameHash                                                   -- among
  AND c.$createdAtBlockHeight < :found.$createdAtBlockHeight                         -- before it in `by`
ORDER BY c.$createdAtBlockHeight                                                     -- by
LIMIT 1000                                                       -- max_choose_first_candidates
-- refused by the first row where:
--   NOT expired(c)                                              -- within its ttl
--   AND c.ownerNameHash = sha256d(c.$ownerId || :normalizedLabel || '.' || :normalizedParentDomainName)   -- validWhen
```

- **Valid, not copied.** Anyone can copy a pending preorder's `nameHash` without knowing the name. A preorder counts only when it satisfies `validWhen` with its own owner, which takes knowing the name: a copy names another identity in its hash than the one that owns it. Copies never refuse a create, and the found preorder must be valid too, or a copy landing first could be revealed by whoever learns the name from the honest reveal.
- **The order.** `by` lists the properties of the referenced type the commitments are ordered by, first entry first, each a name (ascending) or `{ "<name>": "asc" }` or `{ "<name>": "desc" }`: `[{ "$createdAtBlockHeight": "desc" }]` would choose the newest. The type needs an index over exactly `among` followed by those properties, in that order, through which they are read. A last `$id` breaks ties instead: it stands outside that index, which must then not be unique, since a non-unique index keeps its entries by id, in the direction of its last property (ascending under `among` alone). The scan reads them that way, so a last `$id` runs in the direction of the property before it, ascending when it stands alone: `[{ "$createdAtBlockHeight": "desc" }, { "$id": "desc" }]`, not `{ "$id": "asc" }` there. A commitment counts only when it comes strictly before the found one; two equal on every `by` property precede neither other, and the first create then wins through the name's unique index. Whether a tie-breaker such as `$id` can be steered by whoever commits (a document id hashes entropy the creator picks) is the contract's choice to make.
- **Ties.** Ordered by `$createdAtBlockHeight` alone, a competitor from the found preorder's own block does not count.
- **Expiry.** A competitor past its type's `ttl` does not count, though the cleanup may not have deleted it yet. A found preorder past its `ttl` is no commitment either: the reference finds no document.
- **Another identity's.** `findBy` finds the found preorder through the unique index over `$ownerId` and `nameHash`, which holds one preorder of a name per identity, and the writer's is the one found, so every competitor read is another identity's.
- **Consumed.** With `consume` (beside the `where` entry `"$ownerId": "$ownerId"`), a reveal deletes the preorder it found, which then precedes nobody, so a later reveal of the same name by another identity, a contested join, is judged as if it had never existed.
- **The bound.** The competitors are read in one query, in `by` order and only as far as the found preorder's value of the first ordered property, at most `max_choose_first_candidates` of `SystemLimits` (1,000 at protocol version 14). When none of them refuses, the create stands: refusing past the bound would let anyone block a name with copies. Each identity holds one preorder of a name, so pushing an older valid preorder past the bound takes 1,000 other identities, each paying for a preorder of the name ordered before the hidden one. Ordered by `$createdAtBlockHeight` ascending, as here, those land no later than the hidden one's block, while that one is still pending: a copy made once it is in state comes after it. Another `by` loses that timing: ordered newest first, copies posted after a valid preorder lands come before it and can fill the bound, and a property its creator picks can place a later copy ahead of it. Copies and expired preorders make a create cost more, never refuse it.
- **Billing.** The `findBy` hash is billed as the double SHA-256 it is, the found preorder's `validWhen` hash too, the query as the document fetch it is, and `validWhen` computed for each competitor as a double SHA-256. The query and the competitors' hashes together bill at most the scan's budget: `choose_first_credits_per_candidate` of the fee schedule (100,000 credits at protocol version 14) times the candidate limit, 100,000,000 credits (0.001 DASH). Check tx does not validate a reveal's state, so it holds the signer to that whole budget, and to the `findBy` and `validWhen` hashes priced by the reveal's own values (a transient param counts, though nothing stores it), on top of the reveal's estimated fee: a reveal admitted to the mempool can always pay for its `chooseFirst` check in a block, where it is charged only what the check costs. Like any fee check, this holds for the balance the signer had when admitted.
- **What it does not hide.** The `findBy` hash carries no salt: every preorder of a name holds the same one, so a preorder of a name easy to guess discloses it, from the moment it is broadcast. Guessed from a preorder already in a block, the name comes too late: a commitment to it lands in a later block, after the original, which precedes it. Guessed from a preorder still pending, it does not: the guesser can commit to the name and outbid the original, landing in the same block (where neither counts and the name's unique index decides between the reveals) or in an earlier one, whose commitment then comes first. `chooseFirst` settles the race to reveal; for a name easy to guess, the race to commit stays open.

The found preorder must be within its `ttl` and valid, or the reference finds no document (`ReferencedEntityNotFoundError`, 40120). A valid competitor refuses the create with `ReferencedDocumentPrecededError` (40149), which names the found preorder, the competitor and `among`. A create missing a param of `validWhen` is refused before any read (`DocumentReferencePreimageInvalidError`, 10423).

## Rules at registration

**On the referring side**, checked for every contract by the parser (`InvalidContractStructure`, 10231):

- `findBy` is only allowed on `permanentDocument` and `deletableDocument` references.
- The reference's own value is read exactly once, as a `"."` entry or as a param of a [function](#commit-and-reveal). Without it every value would find the same document. Only `ownerRefersTo` or `creatorRefersTo` may leave it out, beside a function.
- Every other source is `"$ownerId"`, a property path or a function. No other `$` name is accepted, and a path may not name the reference property itself: write `"."` for that.
- A property an entry reads must exist on the referring type, be required (and so must every object around it), not be `transient` or inside a transient object, and hold a single value, not an object or an array. `findBy` never runs with a missing key part, and a reader can assemble the same key from the stored document. A function's params follow [their own rules](#commit-and-reveal): they may be transient or optional, since they are read from the create alone.
- A `"$ownerId"` source needs a referring type whose documents can be neither transferred nor traded, unless the reference is judged on the create alone (a [function](#commit-and-reveal) in `findBy`, or a referring type whose documents are never replaced): a transfer or a purchase would move the writer part of the key without a write, but such a reference is never checked again.
- A `deletableDocument` found by `findBy` may not sit under an `immutable` property (a string or byte array one included), alone or as an operand of an expression: every replace checks it again, so once its document is deleted the property would have to change. One whose key a [function](#commit-and-reveal) computes is the exception: it is checked on the create alone, and its carrier must be fixed once written.
- `{ "$id": ... }` is allowed only with [`inList`](refers-to-list-element.md), as the only entry.
- [`chooseFirst`](#choosing-the-first-commitment-choosefirst), in `constraints`, sits only on a `deletableDocument` reference whose `findBy` holds exactly two entries, `"$ownerId": "$ownerId"` and a function filling `among` that reads neither `"$ownerId"` nor `"."`, with no `where` entry but `"$ownerId": "$ownerId"` and no `where` condition (`const`, `notEqual`, `in` or an ordering), which a competitor would never be asked to meet. It holds `among`, `validWhen` (one entry, a function reading `"$ownerId"` and no `"."`, whose params follow a function's rules and read exactly the properties the `findBy` function reads, into a property `findBy` does not name: a property only one of them read, such as a salt, would let a commitment holding the same value have committed to other values) and `by`, 1 to 10 properties each listed once, `$id` only last, in the direction of the property before it (ascending alone), and never `among`. A document type declares at most one, never on the elements of a typed array, as an operand of an expression or beside `inList`, and its creates may offer no gas sponsorship (`gasFeesPaidBy` other than `DocumentOwner`): a sponsored create is judged, state included, before anyone has paid for it.

**On the referenced side**, refused with `InvalidContractStructure` (10231) for a document type of the declaring contract and with `ReferencedDocumentLookupInvalidError` (40137) for one of another contract:

- The keys are exactly the properties of one unique index of the type, by their names on the referenced side (system properties such as `$ownerId` included), in any order, and nothing else (see [Which index finds the document](#which-index-finds-the-document)). That index does not bucket its first property by a `timeRange` or an `integerRange`, the referenced type is not `indexOnly`, and no index property is `transient`.
- Each source holds the same type of value as the property it fills. `"$ownerId"` is an identifier, and so is `"."`, but on a string or byte array property, where it is that property's value.
- The key stays with the document it found. Every schema property of the index must be fixed once written: the referenced type is immutable (`documentsMutable: false`), or the property's top-level property is listed under `immutable` and is no optional `deletableDocument` reference by id, which a replace may clear once its document is deleted. `$ownerId` may be a key part only where the referenced documents can be neither transferred nor traded. `$updatedAt` and its block height forms may be one only where they can be neither replaced, transferred nor traded, and `$transferredAt` and its forms only where they can be neither transferred nor traded. `$id`, `$creatorId`, `$createdAt` and its forms never change.
- For a [`chooseFirst`](#choosing-the-first-commitment-choosefirst), the property `validWhen` fills is a byte array of exactly 32 bytes, fixed once written and not `transient`, of a type whose documents expire (`ttl`), so a commitment never revealed stops preceding others. Every `by` property is recorded on every document (listed in `required` with every object around it, `$ownerId` aside), fixed once written and not `transient`. The type has an index over exactly `among` followed by the `by` properties, in that order (a last `$id` aside, which then needs that index not to be unique), and no unique index holding `among`, or the property `validWhen` fills, without `$ownerId` (a copy of either hash, taken from a pending commitment and landing first under another identity, would otherwise keep the commitment out). The unique index `findBy` finds through, over `$ownerId` and `among`, holds one commitment to a value per identity.

**The referenced type** must exist (`ReferencedDocumentTypeNotFoundError`, 40121). A `permanentDocument` found by `findBy` in a type whose documents can disappear is refused with `ReferencedDocumentTypeDeletableError` (40122), and a `deletableDocument` found by `findBy` in one whose documents cannot with `ReferencedDocumentTypeNotDeletableError` (40131).

Index definitions and the flags these rules read cannot change on a contract update, so a `findBy` that registered keeps resolving, through the same index.

The keyword `lookup`, which named the index and mapped its properties under `keys`, was replaced by `findBy` before protocol version 14 shipped. A declaration still carrying it is refused on every parse, with a message saying to map every property of the unique index to its source and leave the index name out.

## See also

- [References (refersTo)](refers-to.md) for the targets, [`where`](refers-to.md#where) and the replace rules.
- [Expressions](refers-to-expressions.md), where a reference found by `findBy` may be an operand, and [Writer and Creator References](owner-refers-to.md), where `"."` is the writer or the creator.
- [Found by a unique index](../data-model/documents.md#found-by-a-unique-index-findby) in the Documents chapter, with the internals.
- [Indexes (indices)](indexes.md), [Time-Range Indexes](time-range.md), [Mutability](mutability.md) and [transient](transient.md) for the keywords the rules read.
