# propertyConstraints

`propertyConstraints` holds named rules that every created or replaced document of a type must meet. JSON Schema bounds one property at a time; these rules relate properties to each other: a deposit that covers price times quantity, percentages that add up to 100, a closed order that carries its closing time, a second party who is not the owner. Each rule is a small tree of comparisons, arithmetic and logic that consensus evaluates against the document, without reading any state.

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
- **Transfer and purchase.** These change only the owner. Rules that read `$ownerId` are judged again, against the stored document and its new owner; other rules are not, since nothing they read changed. A transfer or purchase that would break an owner rule is refused with 10422.
- **Price updates and deletes** are not judged, with one exception: a delete of an [index-only](index-only.md) document carries the row's values, which are validated like a create's, rules included. The delete does not carry the owner, which is why an index-only type may not have a rule reading `$ownerId`.
- **No state, no fee.** A rule reads only the document and its owner. It changes nothing stored and adds no fee; the limits below bound its cost. SDKs that validate a document before sending it apply the same rules.

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
| `present` | `"path"` | The document holds the property, with a value other than null |
| `absent` | `"path"` | The document leaves the property out, or sets it to null |
| `anyOf` | `[c1, c2, ...]` | At least one of two or more conditions holds |
| `allOf` | `[c1, c2, ...]` | Every one of two or more conditions holds |
| `not` | `condition` | Its one condition does not hold |

Conditions nest: `{ "not": { "allOf": [{ "equal": ["price", 0] }, { "greaterThan": ["quantity", 10] }] } }` refuses a free order of more than 10. An `anyOf` or `allOf` may not list the same condition twice, nor hold one of its own kind directly (it says what one flat list says), and a `not` may not hold a `not` directly.

An `in` says what an `anyOf` of `equal` comparisons says, in far fewer nodes: `{ "in": ["fee", [0, 10, 25, 50]] }` is 6 nodes where the `anyOf` is 13.

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

Two more forms appear only in string and identifier comparisons, never inside arithmetic:

| Form | Meaning |
|---|---|
| `{ "const": "closed" }` | A string constant, or, compared with an identifier property or `$ownerId`, a base58 identifier |
| `{ "ifAbsent": ["status", "open"] }` | A string property, read as the given string when the document leaves it out |

A bare JSON string is always a path and a bare JSON number always a value, so a constant string needs `{ "const": ... }`. The values an `in` lists are literals and need no wrapper. A path is a property name, or names joined by dots for a nested property (`"rewardSplit.leader"`); the only `$` name a rule accepts is `$ownerId`.

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
- no `anyOf` or `allOf` holds its own kind directly, and no `not` holds a `not`;
- a path matches `$ownerId` or dotted names of 1 to 64 letters, digits or underscores, so `$createdAt` and other system properties are refused.

The parser then checks the rules against the document type (`InvalidContractStructure`, 10231):

- every path an integer expression reads names an integer or boolean property; every path compared with a string names a string property; every path compared with an identifier names an identifier property; every path `present` or `absent` tests names a property of any type, an object included;
- no rule reads a property that is `transient` or inside a transient object, since a stored document could never be held to it;
- every comparison and `in` reads at least one property: a comparison of constants would hold for every document or for none;
- strings and identifiers are compared only with `equal`, `notEqual` and `in`; a string is never compared with an identifier; a property is never compared with itself;
- string constants and `ifAbsent` defaults are in the property's `enum` when it has one; identifier constants are base58 identifiers of 32 bytes;
- no literal divisor is 0 and no literal exponent is negative;
- `present` and `absent` do not name `$ownerId`, and an index-only type has no rule reading it;
- no `anyOf` or `allOf` lists two conditions that parse alike, such as `1` and `1.0`, or two `in` conditions listing the same values in another order;
- no condition or operand nests more than 64 levels deep.

Two limits come from the protocol version 14 `SystemLimits`, and a rule over one is refused the same way:

- at most 16 rules per document type (`max_property_constraints`);
- at most 32 nodes per rule (`max_property_constraint_nodes`).

A rule within 32 nodes is never deep enough to reach the 64-level bound. Nodes are counted like this:

| Part of a rule | Nodes |
|---|---|
| A comparison of integers | 1, plus its two sides |
| An `equal` or `notEqual` of strings or identifiers | 3: the comparison and its two sides |
| An `in` over integers | 1, plus its expression, plus 1 per value |
| An `in` over strings or identifiers | 2, plus 1 per value |
| `present`, `absent` | 1 |
| `anyOf`, `allOf` | 1, plus their conditions |
| `not` | 1, plus its condition |
| An integer, a path or an `ifAbsent` | 1 |
| `add`, `multiply`, `subtract`, `divide`, `modulo`, `power` | 1, plus their operands |

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

`present` and `absent` work on properties of any type, strings and objects included. On an integer they are also the only way to tell "not given" from "given as 0", since a missing integer reads as 0 in an expression.

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
