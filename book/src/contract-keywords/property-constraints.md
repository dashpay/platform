# propertyConstraints

`propertyConstraints` holds named rules that every created or replaced document of a type must meet. JSON Schema bounds one property at a time; these rules relate properties to each other: a deposit that covers price times quantity, percentages that add up to 100, a closed order that carries its closing time, a second party who is not the owner, a payment address whose first byte names a known type. Each rule is a small tree of comparisons, arithmetic and logic that consensus evaluates against the document. A rule can also read a total of other documents, how many there are or what an integer property adds up to, from the count and sum trees their indexes keep (see [Totals of other documents](#totals-of-other-documents)), a value of the document one of its references points at, which the reference's check reads anyway (see [Values of a referenced document](#values-of-a-referenced-document)), and the stored version a write replaces, through `$old.` (see [The stored version](#the-stored-version-old)).

| | |
|---|---|
| **Where** | Document type |
| **Value** | An object of rules, at least one. Each key is the rule's name (1 to 64 letters, digits or underscores); each value is a condition (see [Conditions](#conditions)) |
| **Default** | Absent: no rules |
| **Since** | protocol version 14 |
| **On update** | Fixed: adding, removing or changing a rule is refused (`IncompatibleDocumentTypeSchemaError`, 10246). Stored documents were judged against the rules as they were |
| **Errors** | `DocumentPropertyConstraintViolatedError` (10422) on a document; at registration `JsonSchemaError` (10101) or `InvalidContractStructure` (10231) |

## Example

```json
"order": {
  "type": "object",
  "properties": {
    "price": { "type": "integer", "minimum": 0, "maximum": 1000000000, "position": 0 },
    "fee": { "type": "integer", "minimum": 0, "maximum": 1000000, "position": 1 },
    "quantity": { "type": "integer", "minimum": 1, "maximum": 10000, "position": 2 },
    "deposit": { "type": "integer", "minimum": 0, "position": 3 },
    "status": { "type": "string", "enum": ["open", "pending", "closed"], "position": 4 },
    "closedAt": { "type": "integer", "minimum": 0, "position": 5 }
  },
  "required": ["price", "quantity", "deposit", "status"],
  "propertyConstraints": {
    "depositCoversOrder": {
      "lessThanOrEqual": [
        { "multiply": [{ "add": ["price", "fee"] }, "quantity"] },
        "deposit"
      ]
    },
    "feeWaivedOrAtLeastTen": {
      "anyOf": [{ "equal": ["fee", 0] }, { "greaterThanOrEqual": ["fee", 10] }]
    },
    "closedNeedsClosedAt": {
      "anyOf": [{ "notEqual": ["status", { "const": "closed" }] }, { "present": "closedAt" }]
    }
  },
  "additionalProperties": false
}
```

`depositCoversOrder` reads `(price + fee) * quantity <= deposit`. `feeWaivedOrAtLeastTen` reads `fee == 0 || fee >= 10`; an order that leaves `fee` out passes, since a missing integer reads as 0. `closedNeedsClosedAt` says an order whose status is `closed` carries a `closedAt`.

## How it works

- **Create and replace.** The rules run after the JSON schema validation of the document's properties (and after [maxBytes](max-bytes.md)), so every value a rule reads has passed its property's schema. A replace is judged on the whole new document, not only on what changed, with the document it replaces under `$old.`; a create has none.
- **Name order, first failure.** Rules are checked in the order of their names, and the first rule the document breaks refuses the transition with `DocumentPropertyConstraintViolatedError` (10422). The error names the document type, the rule, and why it failed (below).
- **Transfer and purchase.** These change only the owner, the transfer's time and heights, and `$revision`. Rules that read `$ownerId`, `$transferredAt…`, `$revision` or a total that depends on the owner, of the document or of the stored version under `$old.`, are judged again, against the stored document with its new owner and transfer values, `$old.` reading it as it was before; other rules are not, since nothing they read changed. A transfer or purchase that would break such a rule is refused with 10422. A rule reading a referenced document is never among them (see [Values of a referenced document](#values-of-a-referenced-document)).
- **Price updates** change only the update's time and heights and `$revision`, so the rules that read `$updatedAt…` or `$revision`, of either version, are judged again the same way; other rules are not.
- **Moderator field changes.** A moderator's change of the fields the type keeps for its moderators (see [moderatorAbilities.changeFields](moderator-abilities.md#changefields)) is judged as a replace is, with the document it changes under `$old.`, but only against the rules that read a field it changes, of either version, `$revision`, which it moves, or a total, and never against a rule reading a referenced document, which registration keeps from reading anything the change can break. It keeps the document's owner, times and heights. The other rules read only what the owner last wrote, and gave their verdict then.
- **Moderator restores.** The rules of the restored type that read a total are judged on the retained document, with its original owner, times, heights and revision, and `$old.` reading the retained document itself, except a rule reading a referenced document: a restore checks no reference, and registration keeps such a rule from reading any total on a type whose moderators' deletions keep records. Totals include the restored document as an insertion into the live trees; its removal record is not counted. Another write while it was deleted may make a cap, lower bound or cross-type prerequisite fail. The other rules read only what the document held when it was last written, and gave their verdict then: a type's rules never change once it is registered. Uniqueness is checked first. A refusal in a block charges the moderator and consumes its nonce, while the document stays deleted and its removal record stays unrestored. Mempool admission refuses the same invalid restore without persisting fees or a nonce change. Restore does not revalidate the full property schema or judge `deleteConstraints`.
- **Immutable properties.** The same grammar is the condition of an [`immutable`](mutability.md#immutable) entry, which freezes a property while it holds. It reads the stored document through `$old.` as a rule does.
- **Deletes** are not judged, with one exception: a delete of an [index-only](index-only.md) document carries the row's values, which are validated like a create's, rules included. The delete carries neither the owner nor any time, height or revision, which is why an index-only type may not have a rule reading `$ownerId`, a system time or height, `$revision` or `$old.`. What may be deleted is the job of [`deleteConstraints`](deletion.md#deleteconstraints), rules in this grammar the stored document must meet for its owner to delete it, which read it by their plain paths.
- **State and fees.** A rule reads the document, its owner, its times, heights and revision, and the stored version the write replaces, which the write has already fetched; a `countOf` or `sumOf` reads a total from state. Each such total is a state read billed with the write; nothing else a rule does adds a fee, and it changes nothing stored. A value of a referenced document comes from the read the reference's check makes. The limits below bound its cost. SDKs that validate a document before sending it apply the same rules, except those reading a total or a referenced document, which they cannot read, and those reading the stored version when they are not given it (see [The stored version](#the-stored-version-old)).

Why a rule fails, as the error reports it:

| Reason | When |
|---|---|
| does not hold | The rule evaluates without a fault and comes out false |
| overflow | A value it reads, or a result it computes on the way, does not fit a 128-bit signed integer |
| division by zero | A `divide` or `modulo` whose divisor evaluates to 0 |
| negative exponent | A `power` whose exponent evaluates to a negative number |
| not an integer | A value it reads for an integer property is a float with no fractional part, such as `5.0`, which the schema's `integer` type admits but an integer property cannot store |

## Conditions

A rule is a condition: a JSON object with exactly one key.

| Condition | Form | Holds when |
|---|---|---|
| `equal`, `notEqual` | `[left, right]` | The two sides are equal, or differ. The sides are two integer expressions, or a string property and a string constant or another string property, or an identifier property and an identifier constant, another identifier property or `$ownerId` |
| `lessThan`, `lessThanOrEqual`, `greaterThan`, `greaterThanOrEqual` | `[left, right]` | The left integer expression compares with the right one this way. Integers only |
| `in` | `[expression, [v1, v2, ...]]` | The expression takes one of the listed values: two or more, no two alike, all integers or all strings. With strings, the expression is a string property, or an identifier property or `$ownerId` with the strings as base58 identifiers |
| `notIn` | `[expression, [v1, v2, ...]]` | The expression takes none of the listed values: an `in` negated, listed the same way, in as many nodes. A string or identifier property the document leaves out takes none |
| `startsWith`, `endsWith` | `[text, affix]` | The first string starts, or ends, with the second, byte for byte with no case folding. Each side is a string constant, a string property or an `ifAbsent` string default, at least one a property and never the same one twice. A string property left out without a default takes no string, and the condition does not hold for it. Beside a byte array property it tests byte arrays instead (see [Byte arrays](#byte-arrays)) |
| `contains` | `["path", value]` | The typed array property at the path holds an element equal to the value: an integer expression among integers; a string constant, a string property or an `ifAbsent` string default among strings; an identifier constant, an identifier property or `$ownerId` among identifiers. An array the document leaves out holds nothing, and a string or identifier property it leaves out is among no elements |
| `present` | `"path"` | The document holds the property, with a value other than null and, for an object, with at least one member present |
| `absent` | `"path"` | The document leaves the property out, sets it to null, or gives an object no member that is present |
| `anyOf` | `[c1, c2, ...]` | At least one of two or more conditions holds |
| `allOf` | `[c1, c2, ...]` | Every one of two or more conditions holds |
| `not` | `condition` | Its one condition does not hold |
| `ifThen` | `[if, then]` | If the first condition holds, the second must. The second is evaluated only when the first holds, and a fault in either breaks the rule. The two may not be alike |
| `ifThenElse` | `[if, then, else]` | If the first condition holds, the second must; if not, the third must. Only the branch the first selects is evaluated. No two of the three may be alike |

Conditions nest: `{ "not": { "allOf": [{ "equal": ["price", 0] }, { "greaterThan": ["quantity", 10] }] } }` refuses a free order of more than 10. An `anyOf` or `allOf` may not list the same condition twice, nor hold one of its own kind directly (it says what one flat list says), and a `not` may not hold a `not` or a `notIn` directly.

An `in` says what an `anyOf` of `equal` comparisons says, in far fewer nodes: `{ "in": ["fee", [0, 10, 25, 50]] }` is 6 nodes where the `anyOf` is 13.

`startsWith` and `endsWith` test a string's ends: `{ "startsWith": ["url", { "const": "https://" }] }` holds a link to https, `{ "endsWith": ["url", { "const": ".dash" }] }` to a domain, and `{ "startsWith": ["path", "parentPath"] }` holds a reply's path under its parent's. A constant tested against a property that declares an `enum` must start or end one of its values.

A `contains` looks the other way round, for one value among an array's elements:

- `{ "not": { "contains": ["labels", { "const": "used" }] } }` refuses a `"used"` label;
- `{ "contains": ["participants", "$ownerId"] }` holds the owner to the participants, and since it reads `$ownerId`, a transfer or purchase to someone else is refused;
- `{ "contains": ["tiers", "quantity"] }` holds the quantity to one of the tiers the document lists.

The kind of the array's elements decides what the value is: a `{ "const": "sale" }` is a string among strings and a base58 identifier among identifiers.

## Expressions

An integer expression is one of:

| Expression | Form | Value |
|---|---|---|
| integer | `100` | Itself. A number written `100.0` reads as 100 |
| path | `"price"`, `"meta.total"` | The value of an integer or boolean property of the document type, 1 for true and 0 for false. A property the document leaves out, or sets to null, reads as 0 |
| `ifAbsent` | `{ "ifAbsent": ["quantity", 1] }` | The property's value, or the given integer when the document leaves it out |
| `add`, `multiply` | `{ "add": [a, b, ...] }` | The sum or product of two or more operands |
| `subtract` | `{ "subtract": [a, b] }` | `a - b` |
| `divide` | `{ "divide": [a, b] }` | The Euclidean quotient of `a` by `b` |
| `modulo` | `{ "modulo": [a, b] }` | The Euclidean remainder of `a` by `b`, never negative |
| `power` | `{ "power": [a, b] }` | `a` to the power `b` |
| `min`, `max` | `{ "max": [a, b, ...] }` | The least or greatest of two or more operands, every one evaluated |
| `abs` | `{ "abs": a }` | The absolute value of its one operand |
| `length`, `byteLength` | `{ "length": "title" }` | The characters (as `maxLength` counts them) or UTF-8 bytes (as `maxBytes` counts them) of a string property, 0 when the document leaves it out |
| `count` | `{ "count": "tags" }` | The items of an array property, or the bytes of a byte array property, 0 when the document leaves it out |
| `countPresent` | `{ "countPresent": ["email", "phone"] }` | How many of two or more properties, no two alike, the document holds, each as `present` tests it (see [How many of a group](#how-many-of-a-group)) |
| `byteAt` | `{ "byteAt": ["address", 0] }` | The byte, 0 to 255, at an index (counted from 0) of a byte array property, 0 when the array does not hold it or the document leaves it out (see [Byte arrays](#byte-arrays)) |
| system time or height | `"$createdAt"`, `"$updatedAtBlockHeight"` | A time or height the document records (see [Times and heights](#times-and-heights)) |
| `countOf`, `sumOf` | `{ "countOf": ["listing", { "$ownerId": "$ownerId" }] }` | A total of documents of a type of the same contract, read from state (see [Totals of other documents](#totals-of-other-documents)) |
| `referenced` | `{ "referenced": ["pollId", "endsAt"] }` | A value of the document a reference points at, read as a path is (see [Values of a referenced document](#values-of-a-referenced-document)) |

Where `maxLength`, `maxBytes` and `maxItems` bound one property by a fixed number, a size can be compared with another property or bounded only under a condition: `{ "lessThanOrEqual": [{ "count": "tags" }, "maxTags"] }` holds a list to its own limit. A size never breaks a rule by itself: a property left out or null has size 0, and so would a value of another type, which the schema validation refuses first.

Two more forms appear only in string and identifier comparisons, never inside arithmetic:

| Form | Meaning |
|---|---|
| `{ "const": "closed" }` | A string constant, or, compared with an identifier property or `$ownerId`, a base58 identifier, or, tested against a byte array property, hex digits |
| `{ "ifAbsent": ["status", "open"] }` | A string property, read as the given string when the document leaves it out |

A bare JSON string is always a path and a bare JSON number always a value, so a constant string needs `{ "const": ... }`. The values an `in` lists are literals and need no wrapper. A path is a property name, or names joined by dots for a nested property (`"rewardSplit.leader"`); the only `$` names a rule accepts are `$ownerId`, the times and heights below, `$revision`, a path or any of those after `$old.`, which reads the stored version (see [The stored version](#the-stored-version-old)), and, as the value a [total's filter](#totals-of-other-documents) matches by, `$id`. A `{ "referenced": [reference, field] }` stands wherever a path does.

A `number` property (a float) cannot be read by a rule, which keeps every result exact.

## Strings

A string property is compared for equality only, never ordered and never used in arithmetic:

- `{ "equal": ["status", { "const": "closed" }] }` or `notEqual`, with the constant on either side;
- `{ "notEqual": ["fromCurrency", "toCurrency"] }`, two bare paths that both name string properties, which compares their strings;
- `{ "in": ["status", ["open", "pending"]] }`, whose values are two or more distinct strings.

A string property the document leaves out equals no constant and no other string property, not even one also left out. So `notEqual` holds for it, and `equal` and `in` do not. `{ "ifAbsent": ["status", "open"] }` gives it a default instead: it may stand wherever the bare path stands, and the property then reads as that string when it is left out. `present` and `absent` test it directly.

When the property declares an `enum`, every constant compared with it, and every `ifAbsent` default given to it, must be one of the enum's values. A misspelled constant is refused at registration instead of making the rule quietly never hold.

## Identifiers and `$ownerId`

An identifier property compares in the same three ways: `{ "equal": ["paymentToken", { "const": "<base58>" }] }` or `notEqual`, `{ "notEqual": ["buyerId", "sellerId"] }`, and `{ "in": ["paymentToken", ["<base58>", "<base58>"]] }`. Constants are base58 identifiers of 32 bytes, checked at registration and compared by their bytes, whatever form the document gives the identifier in. An identifier property the document leaves out equals no identifier, not even another one left out. Identifiers take no `ifAbsent` default and are never ordered.

`$ownerId`, the document's owner, is an identifier operand too:

- `{ "equal": ["authorId", "$ownerId"] }` holds the `authorId` property to the owner.
- `{ "in": ["$ownerId", ["<base58>", "<base58>"]] }` lets only the listed identities own a document of the type.

It is not a property: `present`, `absent` and integer expressions refuse it, and comparing it with itself is refused. On create and replace it is the writer. A transfer or purchase is judged with the new owner, as described in [How it works](#how-it-works). An [index-only](index-only.md) type may not declare a rule that reads it.

## Byte arrays

A byte array property is read a byte at a time, or tested for the bytes it starts or ends with:

- `{ "byteAt": ["address", 0] }` is an integer expression: the byte at index 0, a number from 0 to 255. It goes wherever an integer goes: `{ "in": [{ "byteAt": ["address", 0] }, [0, 1]] }` holds the first byte to 0 or 1, `{ "lessThan": [{ "byteAt": ["flags", 1] }, 128] }` keeps the top bit of the second byte clear.
- `{ "startsWith": ["payload", { "const": "cafe" }] }` holds a payload whose first two bytes are `0xca` and `0xfe`; `endsWith` looks at the last bytes. The constant is written in hex, two digits a byte, in either case and without a `0x`. Either side may be a byte array property instead, as in `{ "startsWith": ["key", "parentKey"] }`, never the same one twice.

What a document that leaves the array out, or holds fewer bytes, reads:

| | Reads |
|---|---|
| `byteAt` of an array left out or set to null | 0, as an integer property left out does |
| `byteAt` past the last byte the array holds | 0 |
| `startsWith` or `endsWith` of an array left out or set to null | Holds for no constant and no other array, as a string left out does. `not` turns it into a hold |
| `startsWith` or `endsWith` of an array shorter than the constant | Does not hold |

Since a byte left out reads as 0, a rule admitting 0 holds for an array left out: `{ "in": [{ "byteAt": ["address", 0] }, [0, 1]] }` lets a document without an address through. When 0 is not admitted, or a short array must be told from a 0 byte, test the array first: `{ "ifThen": [{ "present": "address" }, { "equal": [{ "byteAt": ["address", 0] }, 1] }] }`, or compare `{ "count": "address" }` with the length it needs.

The index is a literal integer from 0 to 65535, never an expression, and a byte array takes no `ifAbsent` default. `byteAt`, `startsWith` and `endsWith` read only a `byteArray` property, stored (not `transient`, nor inside a transient object). An identifier property is compared whole, with `equal`, `notEqual` or `in` (see [Identifiers](#identifiers-and-ownerid)), and a byte array never is: `equal`, `notEqual`, `in` and `contains` refuse one. A `startsWith` with a constant as long as a fixed-size array says what an `equal` would.

When the property declares `maxItems`, an index at or past it, or a constant longer than it, is refused at registration, since the byte or the bytes are never there.

A rule reading bytes is judged on create and replace like any other; a path through `$old.` reads the stored array's bytes (`{ "byteAt": ["$old.address", 0] }`), and a [`deleteConstraints`](deletion.md#deleteconstraints) rule the stored document's by the plain path. Reading bytes does not make a transfer, a purchase or a price update judge a rule, since none of them changes a byte array.

## Times and heights

A rule can read when the document was created, last updated and last transferred, as an integer:

| | block time (ms) | Platform block height | Core block height |
|---|---|---|---|
| creation | `$createdAt` | `$createdAtBlockHeight` | `$createdAtCoreBlockHeight` |
| last update: a create, a replace or a price update | `$updatedAt` | `$updatedAtBlockHeight` | `$updatedAtCoreBlockHeight` |
| last transfer: a create, a transfer or a purchase | `$transferredAt` | `$transferredAtBlockHeight` | `$transferredAtCoreBlockHeight` |

- `{ "lessThanOrEqual": [{ "subtract": ["endsAt", "$createdAt"] }, 604800000] }` keeps a listing to a week from its creation.
- `{ "lessThanOrEqual": ["$updatedAt", "endsAt"] }` refuses a replace or a price update after the listing ends.
- `{ "lessThanOrEqual": ["$transferredAt", "endsAt"] }` refuses a transfer or a purchase after it ends.

A rule may read one only when the document type records it by listing it in `required`, so every stored document holds it. None takes an `ifAbsent` default, `present` and `absent` refuse them, and an [index-only](index-only.md) type reads none. Each write is judged with the values the stored document ends up with: a create with its block's time and heights for all three events; a replace with the stored creation and transfer values and its block's as the update; a price update with its block's as the update; a transfer or a purchase with its block's as the transfer.

SDK pre-checks run before the block exists: they use the device clock for the times a write records, and do not judge a rule reading a block height the write records, which is unknown until the block: on a create every height read without `$old.`, on a replace the update's. A stored height read through `$old.` is judged: it reads 0 on a create, and the stored document's height on a replace the JavaScript check is given the stored document for.

`$revision` is an integer operand too, on a type that keeps a revision: one whose documents are mutable, transferable, sold at a price their owner sets, or keep fields for their moderators. It is 1 on a create and one more for each later write: a replace, a transfer, a purchase, a price update or a moderator's change of fields; a moderator's restore keeps it. `{ "lessThanOrEqual": ["$revision", 6] }` allows five writes after the create. It counts every write, not only the owner's edits, so a transfer and a price update are judged against a rule reading it. To count edits alone, keep a counter (see [The stored version](#the-stored-version-old)).

## The stored version (`$old.`)

A path under `$old.` reads the stored version of the document a write replaces instead of the version it writes. A rule can then compare the two. On an `order` type that is mutable and transferable, requires `edits`, `$updatedAt` and `$transferredAt`, and whose `dispute` type keeps a countable index by `orderId`:

```json
"propertyConstraints": {
  "counterNeverDrops": {
    "greaterThanOrEqual": ["counter", { "ifAbsent": ["$old.counter", 0] }]
  },
  "closedStaysClosed": {
    "ifThen": [
      { "equal": ["$old.status", { "const": "closed" }] },
      { "equal": ["status", { "const": "closed" }] }
    ]
  },
  "editsAnHourApart": {
    "greaterThanOrEqual": [{ "subtract": ["$updatedAt", "$old.$updatedAt"] }, 3600000]
  },
  "noResaleWithinADay": {
    "ifThen": [
      { "notEqual": ["$ownerId", "$old.$ownerId"] },
      { "greaterThanOrEqual": [{ "subtract": ["$transferredAt", "$old.$transferredAt"] }, 86400000] }
    ]
  },
  "statusChangesOnlyWithoutDisputes": {
    "ifThen": [
      { "notEqual": ["status", "$old.status"] },
      { "equal": [{ "countOf": ["dispute", { "orderId": "$id" }] }, 0] }
    ]
  },
  "editsCounted": {
    "equal": ["edits", { "add": [{ "ifAbsent": ["$old.edits", -1] }, 1] }]
  }
}
```

`counterNeverDrops` lets a counter rise or stay, from any value at or above 0 on a create. `closedStaysClosed` keeps a closed order closed. `editsAnHourApart` spaces a document's replaces, and its price updates, an hour apart. `noResaleWithinADay` refuses a transfer or a purchase less than a day after the last one. `statusChangesOnlyWithoutDisputes` lets the status change only while no dispute points at the document. `editsCounted` starts `edits` at 0 and moves it by one on each replace, a count of edits that, unlike `$revision`, transfers and price updates leave alone. Unlike an [`immutable`](mutability.md#immutable) condition, which refuses a replace with 40128 naming the property, each of these refuses the write with 10422 naming the rule, and can read a total.

What `$old.` reads on each write, and which rules the write is judged against:

| Write | `$old.<property>` | `$old.$ownerId`, a stored time, height or `$old.$revision` | Rules judged |
|---|---|---|---|
| Create | Nothing: it reads as a property the document leaves out does (0 or its `ifAbsent` value in an integer expression, absent to `present`, equal to no string or identifier) | No owner, which equals no identifier; 0 | Every rule |
| Replace | The stored document's | The stored document's: its owner (the writer), the previous update's time and heights, the previous revision | Every rule |
| Transfer, purchase | The stored document's, which are the document's | The previous owner, the previous transfer's time and heights, the previous revision | Those reading the owner, a transfer time or height, `$revision`, or a total depending on the owner, of either version |
| Price update | The stored document's, which are the document's | The previous update's time and heights, the previous revision | Those reading an update time or height or `$revision`, of either version |
| Moderator field change | The stored document's | The stored document's | Those reading a field it changes, of either version, `$revision`, or a total |
| Moderator restore | The restored document's own | The restored document's own | Those reading a total |

A write that changes no property is not judged against a rule that compares properties alone: it would compare each with itself, and a rule demanding a change, such as `{ "greaterThan": ["counter", "$old.counter"] }`, would refuse every transfer.

- `$old.` goes in front of any path a rule reads: in an integer expression, an `ifAbsent`, a size, a `byteAt` or a `countPresent`, in `present` and `absent`, in a comparison of strings, identifiers or byte arrays, in an `in`, and in a `contains`, on either side. The path after it is checked as any path is. `$old.$ownerId` compares as `$ownerId` does, and a stored time, height or revision is an integer operand, read only on a type that records it.
- A create reads every `$old.` value as absent, so a rule a create must pass guards it: `{ "ifAbsent": ["$old.counter", 0] }`, `{ "absent": "$old.status" }` in an `anyOf`, or `$old.$revision` reading 0. `{ "lessThanOrEqual": ["price", "$old.price"] }` alone refuses every create with a price above 0.
- A rule demanding a change is judged on every replace: `{ "greaterThan": ["counter", "$old.counter"] }` refuses a replace that leaves the counter alone.
- A moderator's change of fields keeps `$updatedAt`, so `editsAnHourApart` is not judged against it.
- SDK pre-checks read no state. Each rule's descriptor reports `readsStored` when it reads `$old.`. The create pre-check (`dash_sdk_data_contract_check_property_constraints`, under Swift and Kotlin) judges such a rule as a create, with nothing under `$old.`. The JavaScript `checkDocumentPropertyConstraints(document, storedDocument)` takes the stored document for a replace, `null` for a create, and with `undefined` does not judge such a rule.

## Totals of other documents

`countOf` and `sumOf` read a total from state: how many documents of a type of the same contract match a filter, or what one of their integer properties adds up to. The total is the one a count or sum tree keeps ([Count Trees](../drive/document-count-trees.md), [Sum Trees](../drive/document-sum-trees.md)), so reading it costs about the same however many documents match.

| Form | Value | The counted type needs |
|---|---|---|
| `{ "countOf": ["listing"] }` | How many `listing` documents there are | `documentsCountable` |
| `{ "countOf": ["listing", { "$ownerId": "$ownerId" }] }` | How many of them match the filter | A countable index whose properties are exactly the filter's keys |
| `{ "sumOf": ["pledge", "amount"] }` | The total `amount` over every `pledge` | `documentsSummable: "amount"` |
| `{ "sumOf": ["pledge", "amount", { "campaignId": "campaignRef" }] }` | The total over those matching the filter | An index with `summable: "amount"` whose properties are exactly the filter's keys |

A filter maps each key, a property of the counted type or `$ownerId`, to the value it must take, read from the document being written: one of its properties (`"campaignRef"`), `$ownerId`, `$id` (its own id, for an identifier key), an integer, or a `{ "const": ... }` string or base58 identifier. The counted type may be the rule's own. `$id` counts the documents pointing at this one: `{ "countOf": ["vote", { "pollId": "$id" }] }` is how many votes name the poll being written.

```json
"propertyConstraints": {
  "atMostTenListings": {
    "lessThanOrEqual": [{ "countOf": ["listing", { "$ownerId": "$ownerId" }] }, 10]
  },
  "pledgesWithinGoal": {
    "lessThanOrEqual": [{ "sumOf": ["pledge", "amount", { "campaignId": "campaignRef" }] }, "goal"]
  }
}
```

- **As it will be after the write.** The total is the stored one with the write applied. When the counted type is the rule's own, a create or moderator restore adds the document, a replace swaps its stored version for the new one, and a transfer or purchase moves it to its new owner. So `atMostTenListings`, declared on `listing`, keeps every owner at ten or fewer, and a replace of one of ten is allowed.
- **Judged when the rule's own type is written.** A rule is never judged on writes of the type it counts. On its own type it holds for good, since every write that could raise the total is judged; a type with a contested index cannot total its own documents, since a document a contest awards is stored without any rule judged. On another type it is only checked when its own type is written, and can go stale later: deleting a `profile` does not undo a `post` that needed one. Deletes are not judged, so a lower bound can be broken by deleting documents, unless the counted type's [`deleteConstraints`](deletion.md#deleteconstraints) hold it.
- **Transfers, purchases and price updates.** A total that depends on the owner (a filter value of `$ownerId`, or a `$ownerId` key on the rule's own type) is read again for a transfer or purchase, the document counted toward its new owner. A rule a transfer, purchase or price update judges for another read (`$transferredAt…`, `$updatedAt…` or `$revision`, of either version) reads its totals too.
- **Billed.** Each total is a state read billed with the write. A total two rules read alike is read once.
- **Every earlier write counts.** A document batch carries one transition, and each state transition of a block is applied before the next is validated, so a total includes every write before it.
- **SDK pre-checks** cannot read state, so they do not judge a rule reading a total.

## Values of a referenced document

`{ "referenced": [reference, field] }` reads a value of the document a reference of the type points at: `reference` names a property of the type carrying a [`refersTo`](refers-to.md), by its dotted path when it sits inside an object (`"meta.pollId"`), and `field` a value of the document it finds. It stands wherever a path stands, and reads that value as a path reads a property of the type: as an integer operand, a side of a string or identifier comparison, the operand of an `in`, the path of an `ifAbsent`, `present` or `absent`, a size, a `byteAt`, a `startsWith` or `endsWith`, or either side of a `contains`. It is one node.

```json
"propertyConstraints": {
  "atLeastReserve": {
    "greaterThanOrEqual": ["amount", { "referenced": ["auctionId", "reservePrice"] }]
  },
  "deliveredOrder": {
    "equal": [{ "referenced": ["orderId", "status"] }, { "const": "delivered" }]
  },
  "memberWrites": { "contains": [{ "referenced": ["groupId", "members"] }, "$ownerId"] }
}
```

A bid meets its auction's reserve price, a review names an order that was delivered, and only a member of a group writes to it.

- **The field** is a property of the referenced type by its dotted path, its `$id`, `$ownerId` or `$creatorId` (compared as identifiers, `$creatorId` on a type that records it), or a time or height it lists in `required` (`"$createdAt"`, read as an integer).
- **Absence** reads as a property of the type left out does: an integer reads 0 (or an `ifAbsent` default), a string or an identifier equals nothing, `present` does not hold. A reference the document leaves out leaves every value read through it absent. A `where` [condition](refers-to.md#conditions) reads absence its own way, so the same check written there and here can treat a value left out differently.
- **Which comparison.** The field's type decides, as a property of the type's does. A rule names another type's property, maybe another contract's, so beside a property of the type or `$ownerId` it compares as that property does, and beside a string constant, in an `in` listing strings, or as an array `contains` looks a string up in, the field is read as what it holds: a string is a string, an identifier is written base58, and a byte array hex, the only one of the three with a start and an end a `startsWith` or `endsWith` tests. Two fields of referenced documents compared by `equal` or `notEqual` compare by value whatever they hold, one left out equalling nothing (wrap an integer in `ifAbsent` to read it as 0 instead); an ordering compares two integers. Registration checks the field holds what the rule reads it as, that each constant spells a value of its type, and that two fields compared hold the same kind of value. The referenced document's own `$id`, `$ownerId` and `$creatorId` are identifiers.

**Which references.** The reference holds one document: a `permanentDocument` or `deletableDocument` reference found by id or by a [`findBy`](refers-to-lookup.md) without a function, or a `moderatedDocument` one, into this contract or another. Not the elements of a typed array, an `anyOf` or `allOf`, an `inList`, or a `findBy` function, which is judged on the create alone. Through a `moderatedDocument` reference a rule reads only `$id`, `$ownerId` and the fields the referenced type keeps under [`deleteKeepsFields`](deletion.md#moderatorabilitiesdeletekeepsfields): once a moderator removes the document its [removal record](../data-model/contract-moderation.md#deleting-documents) is all there is to read, and a rule is judged on every replace.

**When it is judged.** A rule reading a referenced document is judged once the type's references hold, with the documents their checks read, after the type's other rules; a missing target refuses the write with `ReferencedEntityNotFoundError` (40120) before it. A broken rule is `DocumentPropertyConstraintViolatedError` (10422), as any rule. Every replace judges every rule, so a reference a rule reads through is [checked again on every replace](refers-to.md#on-replace), its `where` included, and the rule reads the document as it is at the replace: a `permanentDocument` target replaced since is read with its new values. Each such check costs the read the reference's first check made, billed the same way. A kept `moderatedDocument` reference whose document a moderator removed is read from the removal record.

Transfers, purchases, price updates, a moderator's change of fields and a moderator's restore check no reference, and do not judge such a rule. Registration keeps that sound by refusing a rule reading a referenced document that one of those writes could break. On a type whose documents can be transferred or traded, it may not read what a transfer or purchase changes: `$ownerId`, `$transferredAt` and its heights or `$revision`, of either version, or a total depending on the owner. On a type whose prices can be set, it may not read what a price update changes: `$updatedAt` and its heights or `$revision`, of either version. On a type with fields its [moderators change](moderator-abilities.md#changefields), it may not read anything their change can break: one of those fields or `$revision`, of either version, or any `countOf` or `sumOf`, since a total may have moved since the document was last written. On a type whose moderators' deletions keep records, so that a restore can bring a document back, it may read no `countOf` or `sumOf` at all.

**Time against a referenced value.** A create records its block for every time and height; a replace keeps the stored creation and transfer and records its block as the update. So `$createdAt` below a referenced `endsAt` holds the create to the deadline and lets a later replace through, while `$updatedAt` holds every write to it: a vote that may be changed only while its poll is open reads `$updatedAt`. On a type whose documents are never replaced the two are the same.

**SDK pre-checks** cannot read the referenced document, so they do not judge such a rule, and list what it reads in `readsReferenced`.

## How many of a group

`countPresent` counts the properties of a group that the document holds, each as `present` tests it, so a rule compares the count with a number. A seller is reached by exactly one of three contacts:

```json
"propertyConstraints": {
  "oneContact": { "equal": [{ "countPresent": ["email", "phone", "handle"] }, 1] }
}
```

The comparisons and `in` give every bound:

| Want | Rule |
|---|---|
| exactly one | `{ "equal": [{ "countPresent": [...] }, 1] }` |
| at most one | `{ "lessThanOrEqual": [{ "countPresent": [...] }, 1] }` |
| at least two | `{ "greaterThanOrEqual": [{ "countPresent": [...] }, 2] }` |
| one or two | `{ "in": [{ "countPresent": [...] }, [1, 2]] }` |
| none or all three | `{ "in": [{ "countPresent": ["a", "b", "c"] }, [0, 3]] }` |

The count runs from 0 to the size of the group, so an `in` lists a range in a few values. `oneContact` is 6 nodes (the comparison, `countPresent`, three paths and `1`); the same rule written with `present` alone needs an `anyOf` and a `not` of every pair, 17 nodes for three properties and more than 32 for five. Inside `ifThen` the bound applies only when another condition holds: `{ "ifThen": [{ "equal": ["kind", { "const": "shop" }] }, { "greaterThanOrEqual": [{ "countPresent": ["email", "phone", "handle"] }, 1] }] }` asks a shop for at least one contact.

## Evaluation order and short-circuiting

Conditions are checked in declared order and no further than the outcome needs. A comparison evaluates its left side, then its right. `anyOf` stops at the first condition that holds, `allOf` at the first that fails. Operands are evaluated left to right.

A fault (an overflow, a division by zero, a negative exponent, a value that is not an integer) in a condition that is evaluated breaks the rule, whatever the other conditions would say, and `not` does not turn a fault into a pass. String comparisons, `present` and `absent` never fault. So an earlier condition can guard a later one:

```json
{ "anyOf": [{ "equal": ["b", 0] }, { "equal": [{ "divide": ["a", "b"] }, 2] }] }
```

holds for a `b` of 0 without dividing by it. The same two conditions the other way round divide by zero and break the rule.

## Arithmetic

- Integers are exact over 128-bit signed integers. Every intermediate result must fit, and one that does not breaks the rule instead of wrapping. `add` and `multiply` fold their operands from the left, so an overflow on the way is a fault even when a later operand would bring the total back in range.
- `divide` and `modulo` are Euclidean: the remainder is never negative, and the quotient is the one that goes with it. `-7` divided by `2` is `-4`, remainder `1`. For operands that are not negative this is ordinary integer division.
- A divisor that evaluates to 0 breaks the rule. A negative exponent breaks it too, since it has no integer result. `0` to the power `0` is `1`.
- There are no floats.

## Rules at registration

The meta-schema checks the shape (`JsonSchemaError`, 10101):

- the keyword is an object of one or more rules, named with 1 to 64 letters, digits or underscores;
- every condition and every operator object has exactly one key;
- a comparison, `subtract`, `divide`, `modulo` and `power` take exactly two operands; `add` and `multiply` two or more; `anyOf` and `allOf` two or more conditions, no two alike; an `in` two or more distinct values, all integers or all strings; a `countPresent` two or more distinct paths;
- no `anyOf` or `allOf` holds its own kind directly, and no `not` holds a `not` or a `notIn`;
- a path matches `$ownerId`, `$revision`, one of the nine [times and heights](#times-and-heights), or dotted names of 1 to 64 letters, digits or underscores, any of them optionally after `$old.`, so `$id` and other system properties are refused; a `referenced` lists a property name, then a dotted path, `$id`, `$ownerId`, `$creatorId` or a time or height;
- a `byteAt` lists a path and an integer index from 0 to 65535;
- a `countOf` lists a type name and optionally a filter, and a `sumOf` a type name, a property and optionally a filter; a filter has one or more keys, each `$ownerId` or a dotted path, and each value is a path, `$ownerId`, `$id`, an integer or a `{ "const": ... }` string.

The parser then checks the rules against the document type (`InvalidContractStructure`, 10231):

- every path an integer expression reads names an integer or boolean property; every path `length` or `byteLength` measures names a string property, and every path `count` counts an array or byte array property; every path `byteAt` reads, and every path a `startsWith` or `endsWith` of byte arrays tests, names a byte array property, within its `maxItems` when it declares one: an index below it, a constant no longer than it; every path a `contains` looks in names a typed array property whose elements are integers, strings or identifiers, of the kind of the value looked for (a string constant among them in the elements' `enum` when they declare one); every path compared with a string, or tested by `startsWith` or `endsWith`, names a string property, and a constant tested against one with an `enum` starts or ends one of its values; every path compared with an identifier names an identifier property; every path `present`, `absent` or `countPresent` tests names a property of any type, an object included;
- no rule reads a property that is `transient` or inside a transient object, since a stored document could never be held to it;
- every comparison and `in` reads at least one property: a comparison of constants would hold for every document or for none;
- strings and identifiers are compared only with `equal`, `notEqual` and `in`; a string is never compared with an identifier; a byte array is tested only by `startsWith` and `endsWith`, against hex constants (an even number of hex digits) and other byte arrays, never a string or an identifier, and takes no default; a property is never compared with itself;
- string constants and `ifAbsent` defaults are in the property's `enum` when it has one; identifier constants are base58 identifiers of 32 bytes;
- no literal divisor is 0 and no literal exponent is negative;
- every time or height a rule reads, of either version, is one the type lists in `required`, and takes no `ifAbsent` default, nor does `$revision`;
- `present`, `absent` and `countPresent` do not name `$ownerId`, a time, a height or `$revision`, of either version, and an index-only type has no rule reading any of them or anything under `$old.`;
- a rule reads `$revision`, or anything under `$old.`, only on a type that keeps a revision: its documents are mutable, transferable, sold at a price their owner sets, or keep fields for their moderators. On any other a create is the only write a rule judges, with no stored version and the first revision;
- no `countOf` or `sumOf` filter takes a value under `$old.`;
- no `anyOf` or `allOf` lists two conditions that parse alike, such as `1` and `1.0`, or two `in` conditions listing the same values in another order, and no `ifThen` or `ifThenElse` holds two alike conditions;
- no condition or operand nests more than 64 levels deep;
- once every document type of the contract is parsed, every `countOf` and `sumOf` counts a type of the contract that is not index-only, and not its own type when that has a contested index, with a tree that keeps the total as set out in [Totals of other documents](#totals-of-other-documents). A unique, contested, ranked, time-range, integer-range or index-only-terminal index keeps no such total, nor does one with more properties than the filter has keys;
- every key of a filter is `$ownerId` or an integer, string or identifier property of the counted type, and its value is of the same kind (`$ownerId` and `$id` are identifiers); a string constant is in the key's `enum` when it has one, and an identifier constant is base58;
- every property a filter value reads is listed in `required`, with every object around it, so a write always has the value; an index-only type has no rule reading a total;
- a `referenced` reads through a property of the type holding one reference to one document, as set out in [Values of a referenced document](#values-of-a-referenced-document), only in `propertyConstraints` (not `deleteConstraints`, an `immutable` condition or `retractedWhen`, whose writes check no reference), not on an index-only type, and not beside what a write checking no reference changes: what a transfer or purchase of the type changes (`$ownerId`, `$transferredAt` and its heights or `$revision`, of either version, or a total depending on the owner), what a price update of the type changes (`$updatedAt` and its heights or `$revision`, of either version), on a type keeping fields for its moderators one of those fields or `$revision`, of either version, or any total, and, on a type whose moderators' deletions keep records, any total;
- once every document type of the contract is parsed, what a rule reads of a referenced type of the same contract is a field that type gives it: one it has, stored, of the kind the rule reads, a time or height it records, a `$creatorId` it records, a constant its `enum` lists, and through a `moderatedDocument` reference a field its removal record keeps. A referenced type of another contract is checked at registration against the stored contract, refused with `ReferencedDocumentRuleReadInvalidError` (40148).

Three limits come from the protocol version 14 `SystemLimits`, and a rule over one is refused the same way:

- at most 16 rules per document type (`max_property_constraints`);
- at most 32 nodes per rule (`max_property_constraint_nodes`);
- at most 4 distinct `countOf` and `sumOf` totals read by one document type's rules, a total read twice counting once (`max_property_constraint_aggregates`).

A rule within 32 nodes is never deep enough to reach the 64-level bound. Nodes are counted like this:

| Part of a rule | Nodes |
|---|---|
| A comparison of integers | 1, plus its two sides |
| An `equal` or `notEqual` of strings or identifiers, a `startsWith` or an `endsWith` | 3: the condition and its two sides |
| An `in` over integers | 1, plus its expression, plus 1 per value |
| An `in` over strings or identifiers | 2, plus 1 per value |
| `contains` | 2, plus the value it looks for |
| `present`, `absent` | 1 |
| `anyOf`, `allOf` | 1, plus their conditions |
| `not` | 1, plus its condition |
| `ifThen`, `ifThenElse` | 1, plus their conditions |
| `notIn` | as the `in` it negates |
| An integer, a path, an `ifAbsent`, a size (`length`, `byteLength`, `count`), a `byteAt` or a time or height | 1 |
| `countPresent` | 1, plus 1 per path |
| `add`, `multiply`, `subtract`, `divide`, `modulo`, `power`, `min`, `max`, `abs` | 1, plus their operands |
| `countOf`, `sumOf` | 1, plus 1 per filter key |
| `referenced` | 1, as a path |

`depositCoversOrder` above is 7 nodes (the comparison, `multiply`, `add` and four paths), and `closedNeedsClosedAt` is 5. An `in` fits up to 30 values in 32 nodes.

## Worked examples

**Percentages that add up.** The moderation charters system contract requires a proposal's reward split to be whole. The paths name members of the `rewardSplit` object, each an integer from 0 to 100:

```json
"propertyConstraints": {
  "rewardSplitIsWhole": {
    "equal": [
      { "add": ["rewardSplit.leader", "rewardSplit.equal", "rewardSplit.actions"] },
      100
    ]
  }
}
```

Six nodes: the comparison, `add`, three paths and `100`.

**One of two, or both or neither.** A contact card must give an email or a phone; a shipping block gives a street and a city together or not at all:

```json
"propertyConstraints": {
  "reachable": { "anyOf": [{ "present": "email" }, { "present": "phone" }] },
  "addressComplete": {
    "anyOf": [
      { "allOf": [{ "present": "street" }, { "present": "city" }] },
      { "allOf": [{ "absent": "street" }, { "absent": "city" }] }
    ]
  }
}
```

`present` and `absent` work on properties of any type, strings and objects included. On an integer they are also the only way to tell "not given" from "given as 0", since a missing integer reads as 0 in an expression. An object with no member present, `{}` or `{ "inner": {} }`, counts as absent: a stored document does not keep it, and a transfer, purchase or price update is judged on the stored document, so a create or replace is judged the same way.

**A time window.** An event ends after it starts, and lasts at most a week (`startsAt` and `endsAt` are required integer timestamps in milliseconds):

```json
"propertyConstraints": {
  "endsAfterStart": { "lessThan": ["startsAt", "endsAt"] },
  "atMostAWeek": { "lessThanOrEqual": [{ "subtract": ["endsAt", "startsAt"] }, 604800000] }
}
```

**A status workflow.** On a `ticket` type, `status` is optional and means `open` when it is left out. A closed ticket names who closed it; an open or pending one does not:

```json
"propertyConstraints": {
  "closedNamesCloser": {
    "anyOf": [{ "notEqual": ["status", { "const": "closed" }] }, { "present": "closedBy" }]
  },
  "activeHasNoCloser": {
    "anyOf": [
      { "not": { "in": [{ "ifAbsent": ["status", "open"] }, ["open", "pending"]] } },
      { "absent": "closedBy" }
    ]
  }
}
```

Without the `ifAbsent`, a ticket with no status would equal none of the listed strings, the `in` would not hold, and `activeHasNoCloser` would let it carry a `closedBy`. If `status` declares an `enum`, `"closed"`, `"open"` and `"pending"` must all be in it.

**Who may own a badge.** On a transferable `badge` type, only two identities may ever hold one:

```json
"propertyConstraints": {
  "knownHolder": {
    "in": [
      "$ownerId",
      ["HJtU46rVEkKJgevQhiVt2YhdHtDS3xtGzzJqit8en5mb", "GfRNCXeyuB3th33a6nkJKQJecKMdRzuBiPsgvSCyUTvK"]
    ]
  }
}
```

A create by anyone else is refused, and so is a transfer or sale of a badge to anyone else: the rule reads `$ownerId`, so it is judged again with the new owner.

**A guarded division.** The average unit price of a batch is at most 100, and a batch may be empty:

```json
"propertyConstraints": {
  "unitPriceCapped": {
    "anyOf": [
      { "equal": ["quantity", 0] },
      { "lessThanOrEqual": [{ "divide": ["total", "quantity"] }, 100] }
    ]
  }
}
```

The `equal` comes first, so an empty batch never reaches the division. Written the other way round, an empty batch breaks the rule with a division by zero.

**The type of a payment address.** A DIP-33 address in storage form is a type byte, `0x00` for P2PKH or `0x01` for P2SH, then a 20-byte HASH160. A profile may leave its address out:

```json
"profile": {
  "type": "object",
  "properties": {
    "corePaymentAddress": { "type": "bytes", "size": 21, "position": 0 }
  },
  "additionalProperties": false,
  "propertyConstraints": {
    "knownAddressType": { "in": [{ "byteAt": ["corePaymentAddress", 0] }, [0, 1]] }
  }
}
```

Four nodes: the `in`, the `byteAt` and two values. An address starting with `0x02` is refused with 10422, and a profile without one is accepted, since its byte 0 reads as 0. The same rule with `startsWith` takes 8 nodes, an `anyOf` of `absent` and two prefixes:

```json
{
  "anyOf": [
    { "absent": "corePaymentAddress" },
    { "startsWith": ["corePaymentAddress", { "const": "00" }] },
    { "startsWith": ["corePaymentAddress", { "const": "01" }] }
  ]
}
```

`startsWith` is the one to reach for when a run of bytes is checked: `{ "startsWith": ["content", { "const": "1220" }] }` holds a multihash of a SHA-256 digest in 3 nodes, where two `byteAt` comparisons in an `allOf` take 7.

**A vote on a poll.** A single-choice ballot names its poll and an option by index. The poll holds two to three options, may close at `endsAt`, and may take several choices from one voter (`multiChoice`):

```json
"vote": {
  "type": "object",
  "documentsMutable": false,
  "properties": {
    "pollId": {
      "type": "identifier",
      "refersTo": { "type": "deletableDocument", "documentType": "poll" },
      "position": 0
    },
    "choice": { "type": "integer", "minimum": 0, "maximum": 9, "position": 1 }
  },
  "required": ["pollId", "choice", "$createdAt"],
  "propertyConstraints": {
    "castBeforeTheEnd": {
      "lessThan": [
        "$createdAt",
        { "ifAbsent": [{ "referenced": ["pollId", "endsAt"] }, 9007199254740991] }
      ]
    },
    "singleChoicePoll": { "equal": [{ "referenced": ["pollId", "multiChoice"] }, 0] },
    "choiceIsAnOption": {
      "lessThan": [
        "choice",
        { "countPresent": [
          { "referenced": ["pollId", "option0"] },
          { "referenced": ["pollId", "option1"] },
          { "referenced": ["pollId", "option2"] }
        ] }
      ]
    }
  },
  "additionalProperties": false
}
```

A vote lands before the poll closes (one without `endsAt` stays open), only on a poll taking one choice (`multiChoice` left out reads as 0), and for an option the poll holds, its options filled from the first. The three rules read the poll the reference check fetched, so they add no read. A vote is never replaced, so `$createdAt` and `$updatedAt` are the same here.

**A flag in arithmetic.** A boolean reads as 1 or 0, so a waived fee must be 0:

```json
"propertyConstraints": {
  "waivedMeansFree": { "equal": [{ "multiply": ["waiveFee", "fee"] }, 0] }
}
```

## See also

- [Property Constraints](../data-model/documents.md#property-constraints-propertyconstraints), the deep dive
- [deleteConstraints](deletion.md#deleteconstraints), rules in this grammar judged on the owner's delete
- [References (refersTo)](refers-to.md), whose `where` checks the referenced document with an equality, a literal, an inequality, a list or an ordering, each when the reference is checked
- [distinctFrom](distinct-from.md), a single-keyword way to keep two identifiers apart
- [Property Schemas](property-schemas.md), for the one-property bounds JSON Schema gives
- [transient](transient.md), [Index-Only Types](index-only.md)
- [Contract Keywords overview](../contract-keywords.md), for the limits and the conventions of these tables
