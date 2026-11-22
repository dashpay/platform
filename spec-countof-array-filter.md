# Spec: a `countOf` / `sumOf` filter that matches any element of a typed array

Working document. Never commit it (see CLAUDE.md); delete it once the change ships.

## Problem

A `propertyConstraints` rule can count the documents of another type that match
a value of the document being written:

```json
{ "equal": [{ "countOf": ["bannedTag", { "tag": "topic" }] }, 0] }
```

The filter value is one property. A post's hashtags are a typed array (`tags`,
up to 4 strings), and nothing in the rule language reads each element, so "no
tag of this post is banned" cannot be written. References do not help: a
reference requires a document to exist, never that none does.

## Chosen approach

A filter value may be `{ "in": "<typed array property>" }`: the key matches
when it equals **any element** of the list.

```json
{ "equal": [{ "countOf": ["bannedTag", { "banned": 1, "tag": { "in": "tags" } }] }, 0] }
```

The total is the number of counted documents whose key is one of the list's
distinct elements (for a `sumOf`, their summed property): the sum of the totals
each distinct element reads. It is an integer operand like any other total, so
every comparison works on it, not only `== 0`.

- A list the document leaves out, or an empty one, matches nothing: total 0.
- Elements are taken once each, as a typed array index takes them, so a list
  that repeats an element does not count a document twice.
- Everything else about a total is unchanged: read when the rule's own type is
  written, as it will be after the write, billed per state read, and it can go
  stale later (banning a tag does not touch posts already written).

### Rules at registration (all `InvalidContractStructure`, 10231)

- At most one `in` value per filter; the other keys bind as today.
- The path names a top-level typed array of the declaring type, stored (not
  `transient`), declaring `maxItems`, whose elements are strings, identifiers
  or integers, of the same kind as the key. No array of byte arrays.
- The counted type is not the rule's own type. What a document adds to a total
  of its own type would have to be matched per element, and nothing needs it.
- An `in` total counts as `maxItems` totals toward
  `SystemLimits::max_property_constraint_aggregates` (4): it makes that many
  state reads. No new limit. A post with `tags` of 4 uses the whole budget.
- The answering index is found as today: plain, countable (or summing the
  property), its properties exactly the filter's keys.

### Versioning

- `parse_property_constraints` generation 1, the unreleased protocol version 15
  generation that already adds byte array keys, edited in place. Generation 0
  (protocol version 14) keeps refusing an object other than `{ "const": ... }`
  as a filter value.
- The shape is admitted by the native schema check of registration
  (`try_from_schema/native_schema/constraints.rs`, `aggregate_filter`), which
  only protocol version 15 runs. Meta-schema v3 is untouched, so the form does
  not exist before 15.
- No drive method changes generation: `fetch_property_constraint_aggregate` 0
  is called once per element with ordinary filter values.

## Where the code changes

| File | Change |
|---|---|
| `rs-dpp/.../property_constraints/mod.rs` | `AggregateBinding::Elements { path, kind }`; `parse_aggregate` reads `{ "in": path }` under generation 1; `visit_property_reads` visits the array as a `contains` does |
| `rs-dpp/.../property_constraints/aggregate.rs` | `filter_values` returns one value set per distinct element (none for an empty list) instead of at most one; `contribution` is unreachable for such a read (`of_own_type` is refused) |
| `rs-dpp/.../try_from_schema/mod.rs` | the registration rules above; `validate_rule_limits` weighs an `in` total by its array's `maxItems` |
| `rs-dpp/.../native_schema/constraints.rs` | `aggregate_filter` admits `{ "in": "<path>" }` |
| `rs-drive-abci/.../transformer/v0/property_constraint_aggregates.rs` | `read_aggregates` fetches each value set and adds them up, with the existing overflow check |
| `book/src/contract-keywords/property-constraints.md` | the form, its rules, the limit weight |

The map handed to the action stays `AggregateRead -> i128`, so the actions, the
rule evaluation, the moderator-change, restore and delete paths, and the SDKs
need no change: they already read every total a rule names and judge on the
number.

## Alternatives rejected

- **A bare array path as the value** (`{ "tag": "tags" }`). Shorter, but the
  same spelling would mean "equals" for a scalar and "is one of" for a list.
- **A negative reference** (`refersTo` that requires no document). A second
  mechanism for what a total of 0 already says, and only the `== 0` case.
- **`tag1` .. `tag4` scalar properties in the contract.** Works today but loses
  the typed array index the hashtag timelines and the like references use.
- **Totals of the rule's own type through an array.** Needs per-element
  contribution arithmetic for a case nobody has.

## Failure modes

- **A retraction refused.** A rule is judged on every replace, so a plain "no
  banned tag" rule would stop an author retracting a post whose tag was banned
  after it was written. That is the contract's to handle, and the example below
  does (`anyOf` with `present: deleted`).
- **A restore refused.** A moderator's restore judges rules reading totals, so
  a removed post whose tag has since been banned can not be restored
  (`DocumentPropertyConstraintViolatedError`, 10422) until the tag is unbanned.
- **Cost.** Every create and replace of the type pays up to `maxItems` point
  reads, whoever pays the gas (the sponsor, for a sponsored post).

## What Yappr would write (social v20)

```json
"bannedTag": {
  "type": "object",
  "documentsMutable": false,
  "canBeDeleted": false,
  "moderatorAbilities": { "changeFields": ["banned"] },
  "indices": [
    { "name": "banned", "properties": [{ "banned": "asc" }, { "tag": "asc" }],
      "countable": "countable", "skipIfAbsent": ["banned"] }
  ],
  "properties": {
    "tag": { "type": "string", "pattern": "^[a-z0-9_]{1,61}$", "maxLength": 61, "position": 0 },
    "banned": { "type": "integer", "minimum": 1, "maximum": 1, "position": 1 }
  },
  "required": ["$createdAt", "tag"],
  "additionalProperties": false
}
```

and on `post`:

```json
"noBannedTags": {
  "anyOf": [
    { "present": "deleted" },
    { "equal": [{ "countOf": ["bannedTag", { "banned": 1, "tag": { "in": "tags" } }] }, 0] }
  ]
}
```

Only moderators can set `banned` (`changeFields`), on a document they create or
on one anyone nominated; a document without it is in no index and counts for
nothing. Removing `banned` unbans the tag. The elected declaration gives the
team `changeDocumentFields` on `bannedTag`. To check when implementing: that a
`changeFields` property may sit in a countable index another type's rule
counts (the book lists no rule against it).

## Test plan

- dpp, registration: the form parses at 15 and is refused at 14; each
  registration rule above refuses; the limit counts `maxItems`.
- dpp, `filter_values`: absent and empty lists give no value set; repeated
  elements give one; an element the key cannot hold matches nothing.
- drive-abci, `batch/tests/document/property_constraints.rs`: a post with a
  banned tag refused in a block (and in the mempool, to confirm: wherever the
  existing total rules are judged there); allowed after the unban;
  two of four tags banned give a total of 2; the retraction passes under the
  `anyOf`; the fee carries one read per distinct element.
- drive-abci, restore: a restore refused while a kept tag is banned.
