# propertyConstraints

`propertyConstraints` holds named rules that every created or replaced document of a type must meet. JSON Schema bounds one property at a time; these rules relate properties to each other: a deposit that covers price times quantity, percentages that add up to 100, a closed order that carries its closing time, a second party who is not the owner. Each rule is a small tree of comparisons, arithmetic and logic that consensus evaluates against the document. A rule can also read a total of other documents, how many there are or what an integer property adds up to, from the count and sum trees their indexes keep (see [Totals of other documents](#totals-of-other-documents)).

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

- **Create and replace.** The rules run after the JSON schema validation of the document's properties (and after [maxBytes](max-bytes.md)), so every value a rule reads has passed its property's schema. A replace is judged on the whole new document, not only on what changed.
- **Name order, first failure.** Rules are checked in the order of their names, and the first rule the document breaks refuses the transition with `DocumentPropertyConstraintViolatedError` (10422). The error names the document type, the rule, and why it failed (below).
- **Transfer and purchase.** These change only the owner and the transfer's time and heights. Rules that read `$ownerId`, `$transferredAt…` or a total that depends on the owner are judged again, against the stored document with its new owner and transfer values; other rules are not, since nothing they read changed. A transfer or purchase that would break such a rule is refused with 10422.
- **Price updates** change only the update's time and heights, so the rules that read `$updatedAt…` are judged again the same way; other rules are not.
- **Immutable properties.** The same grammar is the condition of an [`immutable`](mutability.md#immutable) entry, which freezes a property while it holds. Only such a condition may read the stored document, through `$old.<path>`; a rule judges creates too, which have none.
- **Deletes** are not judged, with one exception: a delete of an [index-only](index-only.md) document carries the row's values, which are validated like a create's, rules included. The delete carries neither the owner nor any time or height, which is why an index-only type may not have a rule reading `$ownerId` or a system time or height.
- **State and fees.** A rule reads the document, its owner and its times and heights, and a `countOf` or `sumOf` reads a total from state. Each such total is a state read billed with the write; nothing else a rule does adds a fee, and it changes nothing stored. The limits below bound its cost. SDKs that validate a document before sending it apply the same rules, except those reading a total, which they cannot read.

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
| `startsWith`, `endsWith` | `[text, affix]` | The first string starts, or ends, with the second, byte for byte with no case folding. Each side is a string constant, a string property or an `ifAbsent` string default, at least one a property and never the same one twice. A string property left out without a default takes no string, and the condition does not hold for it |
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
| system time or height | `"$createdAt"`, `"$updatedAtBlockHeight"` | A time or height the document records (see [Times and heights](#times-and-heights)) |
| `countOf`, `sumOf` | `{ "countOf": ["listing", { "$ownerId": "$ownerId" }] }` | A total of documents of a type of the same contract, read from state (see [Totals of other documents](#totals-of-other-documents)) |

Where `maxLength`, `maxBytes` and `maxItems` bound one property by a fixed number, a size can be compared with another property or bounded only under a condition: `{ "lessThanOrEqual": [{ "count": "tags" }, "maxTags"] }` holds a list to its own limit. A size never breaks a rule by itself: a property left out or null has size 0, and so would a value of another type, which the schema validation refuses first.

Two more forms appear only in string and identifier comparisons, never inside arithmetic:

| Form | Meaning |
|---|---|
| `{ "const": "closed" }` | A string constant, or, compared with an identifier property or `$ownerId`, a base58 identifier |
| `{ "ifAbsent": ["status", "open"] }` | A string property, read as the given string when the document leaves it out |

A bare JSON string is always a path and a bare JSON number always a value, so a constant string needs `{ "const": ... }`. The values an `in` lists are literals and need no wrapper. A path is a property name, or names joined by dots for a nested property (`"rewardSplit.leader"`); the only `$` names a rule accepts are `$ownerId` and the times and heights below.

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

SDK pre-checks run before the block exists: they use the device clock for the times a write records, and do not judge a rule reading a block height, which is unknown until the block.

## Totals of other documents

`countOf` and `sumOf` read a total from state: how many documents of a type of the same contract match a filter, or what one of their integer properties adds up to. The total is the one a count or sum tree keeps ([Count Trees](../drive/document-count-trees.md), [Sum Trees](../drive/document-sum-trees.md)), so reading it costs about the same however many documents match.

| Form | Value | The counted type needs |
|---|---|---|
| `{ "countOf": ["listing"] }` | How many `listing` documents there are | `documentsCountable` |
| `{ "countOf": ["listing", { "$ownerId": "$ownerId" }] }` | How many of them match the filter | A countable index whose properties are exactly the filter's keys |
| `{ "sumOf": ["pledge", "amount"] }` | The total `amount` over every `pledge` | `documentsSummable: "amount"` |
| `{ "sumOf": ["pledge", "amount", { "campaignId": "campaignRef" }] }` | The total over those matching the filter | An index with `summable: "amount"` whose properties are exactly the filter's keys |

A filter maps each key, a property of the counted type or `$ownerId`, to the value it must take, read from the document being written: one of its properties (`"campaignRef"`), `$ownerId`, an integer, or a `{ "const": ... }` string or base58 identifier. The counted type may be the rule's own.

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

- **As it will be after the write.** The total is the stored one with the write applied. When the counted type is the rule's own, a create adds the document, a replace swaps its stored version for the new one, and a transfer or purchase moves it to its new owner. So `atMostTenListings`, declared on `listing`, keeps every owner at ten or fewer, and a replace of one of ten is allowed.
- **Judged when the rule's own type is written.** A rule is never judged on writes of the type it counts. On its own type it holds for good, since every write that could raise the total is judged; a type with a contested index cannot total its own documents, since a document a contest awards is stored without any rule judged. On another type it is only checked when its own type is written, and can go stale later: deleting a `profile` does not undo a `post` that needed one. Deletes are not judged, so a lower bound can be broken by deleting documents.
- **Transfers, purchases and price updates.** A total that depends on the owner (a filter value of `$ownerId`, or a `$ownerId` key on the rule's own type) is read again for a transfer or purchase, the document counted toward its new owner. A rule a price update judges, one reading `$updatedAt…`, reads its totals too.
- **Billed.** Each total is a state read billed with the write. A total two rules read alike is read once.
- **Every earlier write counts.** A document batch carries one transition, and each state transition of a block is applied before the next is validated, so a total includes every write before it.
- **SDK pre-checks** cannot read state, so they do not judge a rule reading a total.

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
- a comparison, `subtract`, `divide`, `modulo` and `power` take exactly two operands; `add` and `multiply` two or more; `anyOf` and `allOf` two or more conditions, no two alike; an `in` two or more distinct values, all integers or all strings;
- no `anyOf` or `allOf` holds its own kind directly, and no `not` holds a `not` or a `notIn`;
- a path matches `$ownerId`, one of the nine [times and heights](#times-and-heights), or dotted names of 1 to 64 letters, digits or underscores, so `$revision` and other system properties are refused;
- a `countOf` lists a type name and optionally a filter, and a `sumOf` a type name, a property and optionally a filter; a filter has one or more keys, each `$ownerId` or a dotted path, and each value is a path, `$ownerId`, an integer or a `{ "const": ... }` string.

The parser then checks the rules against the document type (`InvalidContractStructure`, 10231):

- every path an integer expression reads names an integer or boolean property; every path `length` or `byteLength` measures names a string property, and every path `count` counts an array or byte array property; every path a `contains` looks in names a typed array property whose elements are integers, strings or identifiers, of the kind of the value looked for (a string constant among them in the elements' `enum` when they declare one); every path compared with a string, or tested by `startsWith` or `endsWith`, names a string property, and a constant tested against one with an `enum` starts or ends one of its values; every path compared with an identifier names an identifier property; every path `present` or `absent` tests names a property of any type, an object included;
- no rule reads a property that is `transient` or inside a transient object, since a stored document could never be held to it;
- every comparison and `in` reads at least one property: a comparison of constants would hold for every document or for none;
- strings and identifiers are compared only with `equal`, `notEqual` and `in`; a string is never compared with an identifier; a property is never compared with itself;
- string constants and `ifAbsent` defaults are in the property's `enum` when it has one; identifier constants are base58 identifiers of 32 bytes;
- no literal divisor is 0 and no literal exponent is negative;
- every time or height a rule reads is one the type lists in `required`, and takes no `ifAbsent` default;
- `present` and `absent` do not name `$ownerId` or a time or height, and an index-only type has no rule reading any of them;
- no `anyOf` or `allOf` lists two conditions that parse alike, such as `1` and `1.0`, or two `in` conditions listing the same values in another order, and no `ifThen` or `ifThenElse` holds two alike conditions;
- no condition or operand nests more than 64 levels deep;
- once every document type of the contract is parsed, every `countOf` and `sumOf` counts a type of the contract that is not index-only, and not its own type when that has a contested index, with a tree that keeps the total as set out in [Totals of other documents](#totals-of-other-documents). A unique, contested, ranked, time-range, integer-range or index-only-terminal index keeps no such total, nor does one with more properties than the filter has keys;
- every key of a filter is `$ownerId` or an integer, string or identifier property of the counted type, and its value is of the same kind; a string constant is in the key's `enum` when it has one, and an identifier constant is base58;
- every property a filter value reads is listed in `required`, with every object around it, so a write always has the value; an index-only type has no rule reading a total.

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
| An integer, a path, an `ifAbsent`, a size (`length`, `byteLength`, `count`) or a time or height | 1 |
| `add`, `multiply`, `subtract`, `divide`, `modulo`, `power`, `min`, `max`, `abs` | 1, plus their operands |
| `countOf`, `sumOf` | 1, plus 1 per filter key |

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

**A flag in arithmetic.** A boolean reads as 1 or 0, so a waived fee must be 0:

```json
"propertyConstraints": {
  "waivedMeansFree": { "equal": [{ "multiply": ["waiveFee", "fee"] }, 0] }
}
```

## See also

- [Property Constraints](../data-model/documents.md#property-constraints-propertyconstraints), the deep dive
- [distinctFrom](distinct-from.md), a single-keyword way to keep two identifiers apart
- [Property Schemas](property-schemas.md), for the one-property bounds JSON Schema gives
- [transient](transient.md), [Index-Only Types](index-only.md)
- [Contract Keywords overview](../contract-keywords.md), for the limits and the conventions of these tables
