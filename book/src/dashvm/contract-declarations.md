# Contract Declarations and the Author API

> **Status:** specification landed on the development branch as the crate
> `dash-sdk-contract` (`packages/rs-dash-sdk-contract`, import
> `dash_sdk_contract`). The attribute spellings, the export symbol scheme, the
> method, module, interface and rule name grammars and the diagnostic codes are
> provisional under the shared allocation register for the Rust macro grammar.
> The proc macros, generated persistence wrappers, host context and runtime are
> later deliverables of the same workstream and consume the types described
> here. The native translation and host validation crate,
> `dash-contract-build`, lands separately.

A DashVM contract is ordinary Rust. A struct marked `#[persistent]` is stored
as Platform documents; indexes are declared on that struct; only functions
marked `#[entry]` are callable from outside, and `pub` alone exports nothing.
This chapter describes the declaration model behind those attributes: what
the grammar admits, how declarations are identified, what the validator
checks and what it leaves to the native host, and what the canonical manifest
looks like.

The confirmed policy the model implements:

- Detached Rust values do not save automatically. Explicit insert and edit
  operations and successful exported mutable-receiver wrappers stage writes in
  the outer transaction. No save on `Drop`.
- Simple persistence, index and rule attributes plus typed builders for
  advanced native features produce one canonical manifest. Unsupported or
  conflicting declarations are rejected; native validation remains the
  authority. No unrestricted database paths.
- Existing versioned DPP and Drive index-update rules are reused exactly.
  There is no index backfill or migration subsystem.
- Private Rust fields and access-control checks are not encryption. The
  private document store stays in the capability catalogue with its interface
  disabled until encryption, key control, query visibility and proof behaviour
  are specified.
- A contested-index award is a native action. Contracts parameterize
  contested indexes only through the supported native declarations; no guest
  code, guard or predicate runs on the award.

## The sketch

The issue's `Score` example, in the shape the model accepts:

```rust
use dash_sdk_contract::prelude::*;

#[persistent(collection = "scores", schema = 1, write = "contract")]
#[index(name = "by_class", fields(class = "asc"), count, sum = "points")]
#[index(name = "by_owner", fields(owner = "asc"))]
#[index(name = "ranking", fields(class = "asc", points = "asc"), ranked_count)]
pub struct Score {
    #[document_id]
    pub id: DocumentId,
    #[field(position = 0, max_chars = 64)]
    pub class: String,
    #[field(position = 1, min = 0, max = 1_000_000)]
    pub points: i64,
    #[field(position = 2, refers_to = "identity")]
    pub owner: IdentityId,
}

impl Score {
    #[entry(name = "score.add")]
    pub fn add(&mut self, ctx: &mut Context, delta: i64) -> Result<()> { /* ... */ }
}

#[entry(name = "score.total", read_only)]
pub fn class_total(ctx: &Context, class: String) -> Result<CountAndSum> { /* ... */ }
```

One correction to the original sketch: the `ranking` index is declared with
`points = "asc"`, not `"desc"`. The native document meta-schema admits only
ascending index properties; a descending walk is a per-query choice, not an
index property. `fields(points = "desc")` is rejected with
`InvalidOptionValue`, and the diagnostic says why.

Until the macros ship, the same declaration is written through builders:

```rust
let scores = CollectionSpec::documents(CollectionName::new("scores")?)
    .write(WritePolicy::Contract)
    .document_id_field("id")
    .field(FieldSpec::new(PropertyName::new("class")?, 0, FieldType::string(64)))
    .field(FieldSpec::new(
        PropertyName::new("points")?,
        1,
        FieldType::bounded_integer(IntegerWidth::I64, 0, 1_000_000),
    ))
    .field(FieldSpec::new(PropertyName::new("owner")?, 2, FieldType::identity()))
    .index(
        IndexSpec::new(IndexName::new("by_class")?, vec![PropertyPath::new("class")?])
            .count()
            .sum(PropertyName::new("points")?),
    );

let manifest = ContractDeclaration::new()
    .collection(scores)
    .entry(
        EntrySpec::new(MethodName::new("score.add")?)
            .receiver(Receiver::Mut(CollectionName::new("scores")?))
            .param("delta", ValueType::Integer(IntegerWidth::I64)),
    )
    .validate()?;
```

Attributes and builders feed the same `ContractDeclaration`. A builder may
restate an attribute declaration verbatim, or add indexes to an
attribute-declared collection, but a builder that changes what an attribute
said is a `ConflictingDeclaration` naming both origins.

## The attribute grammar

The grammar is data: `dash_sdk_contract::grammar::ATTRIBUTES` lists every
attribute, its options and the value each option accepts. The proc macros
parse against that table, `grammar::check_keys` reports grammar diagnostics
from it, and a test pins this chapter's table against it, so the three cannot
drift. An option not in the table is `UnknownOption`; a value outside a closed
set is `InvalidOptionValue`; a repeated option is `DuplicateOption`; a
missing required option is `MissingOption`. Nothing is ignored.

| Attribute | On | Options |
|---|---|---|
| `persistent` | struct | `collection` (required), `schema` (integer, default 1), `write` (`any` / `owner` / `contract`), `mutable`, `deletable`, `keep_history`, `keep_transfer_history`, `keep_purchase_history`, `keep_pricing_history`, `transferable`, `trade` (`none` / `direct_purchase`), `security_level` (`critical` / `high` / `medium`), `encryption_key` and `decryption_key` (`unique` / `multiple` / `multiple_reference_to_latest`), `count`, `range_count`, `sum = "<property>"`, `range_sum`, `average = "<property>"`, `range_average`, `index_only`, `store` (`public` / `private`) |
| `singleton` | struct | `collection` (required), `schema`, `write`, `security_level`, `encryption_key`, `decryption_key`, `store` |
| `token_cost` | struct, repeatable | `on` (an ordinary action, required), `token_position` (required), `amount` (required), `contract` (base58), `effect` (`transfer_to_contract_owner` / `burn`), `gas_paid_by` (`document_owner` / `contract_owner` / `prefer_contract_owner`) |
| `index` | struct, repeatable | `name` (required), `fields(<path> = "asc", ...)` (required), `unique`, `null_searchable` (default true), `contested(field_matches(<path> = "<regex>"), resolution = "masternode_vote", description)`, `count` or `count = "offset"`, `range_count`, `sum = "<property>"`, `range_sum`, `average = "<property>"`, `range_average`, `ranked_count` or `ranked_count = ["<level>", ...]`, `ranked_sum`, `ranked_average`, `time_range(on, range_secs, step_secs, phase_secs)`, `terminal = "<property>"`, `preallocated`, `skip_if_absent` |
| `field` | field | `position` (required), `max_chars`, `min_chars`, `max_len`, `min_len`, `min`, `max`, `values = [...]`, `required` (default true), `transient`, `refers_to` (`identity` / `contract` / `token` / `permanent_document` / `identity_public_key`), `document_type`, `contract`, `agreement(<mine> = "<theirs>")`, `key_id_field`, `description` |
| `document_id` | field | none |
| `entry` | fn | `name` (required), `read_only`, `module` |
| `rule` | struct, repeatable | `name` (required), `on = [...]` (ordinary actions, required), exactly one of `guard = "<const>"` or `predicate = "<module>::<export>"` |
| `contract` | crate | `receipts` (`stored` / `disabled`, default stored), `requires = [...]` (`acl`, `randomness`) |
| `module` | module | `name` (required), `uses = [...]` |
| `interface` | trait | `name` (required), `provider` (required) |

The ordinary actions are `create`, `replace`, `delete`, `transfer`,
`purchase` and `update_price`. There is no `award`: `on = "award"` is
`InvalidOptionValue` and the reason names the native award rule.

## Identity

A declaration is identified by its declared name, never by the Rust path,
impl block, source file or declaration order that produced it.

| Identity | Grammar | Where the rule comes from |
|---|---|---|
| Collection | `^[a-zA-Z0-9_-]{1,64}$` | native document type name |
| Property | `^[a-zA-Z0-9_-]{1,64}$`, plus its position | meta-schema property names |
| Index | 1 to 32 characters, unique within its collection | meta-schema index name |
| Method | `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$`, at most 64 bytes, unique contract-wide | provisional |
| Module, interface | `^[a-z0-9_]{1,64}$` | provisional, aligned with the bundle validation crate |
| Rule | `^[a-z][a-z0-9_]{0,63}$`, unique within its collection | provisional |

An entry's WASM export symbol is `dash_entry_` followed by the method name
verbatim (`dash_entry_score.add`). The mapping is a prefix plus the identity,
so it is injective; WebAssembly export names are arbitrary UTF-8 and Rust's
`#[export_name]` accepts dots. The numeric method and type identifiers derived
from these names are allocated by the ABI work, not here.

## Bounded fields

Every string, byte array and list declares a maximum, in stored fields and in
entry parameters and returns alike; a missing maximum is `UnboundedField`,
never a default. Integer fields carry their Rust width (`u8` to `u64`, `i8` to
`i64`) and optional narrower bounds; bounds outside the width or inverted are
`IntegerBoundsOutsideType`. Bounds are stored as `i128` so that a `u64` bound
above `i64::MAX` is representable and rejected as
`IntegerBoundNotNativelyRepresentable` (the native schema reads bounds as
signed 64-bit) instead of being truncated. Positions are contiguous from 0 at
every nesting level and are never renumbered.

The width is part of the manifest. The build crate emits the width's own
range as the native `minimum` and `maximum` when the author gives no bounds,
so native sized-integer inference reproduces the declared width; an author
bound narrower than the width yields the narrower native storage type, which
generated codecs accept.

## What the validator checks, and what it leaves to native

`dash_sdk_contract::validate` runs every check and returns every diagnostic;
there is no first-error return. Diagnostics are typed, append-only and carry
stable codes (`DSC0001` onwards, provisional).

The validator owns what the native host cannot know:

- grammar (unknown attributes, options and values; missing, duplicate and
  mutually exclusive options; sugar conflicts);
- identity (duplicate collections, indexes, methods, export symbols, rules,
  modules, interfaces, properties and positions; non-contiguous positions;
  invalid names);
- conflicts between an attribute and a builder describing one item;
- boundedness of every variable-length field and wire value;
- cross-references (an index, sum, contested field match, ranked level, time
  range source, terminal, reference agreement, receiver, rule or guard naming
  something the declaration does not have);
- collection kinds (a singleton has no indexes and no document id field; a
  persistent struct needs one);
- entry semantics (a `&mut self` entry on a collection whose documents cannot
  be replaced has nothing to stage; a read-only entry cannot take `&mut
  self`);
- the module graph (unknown providers and interfaces, self-imports, cycles,
  an entry with no module when several exist);
- rule scope and kind;
- the capability catalogue (the private store is `CapabilityInterfaceDisabled`;
  derived capabilities cannot be required explicitly).

It deliberately does not mirror native numeric limits (ten indexes per type,
32-character index names, 63-character indexed strings, one hundred
properties, time-range caps) or native schema dependencies (a range count
needs a count, a ranking needs its range axis, a prefix ranking excludes sum
axes, a time range needs a system timestamp). Those are enforced once, by Dash
Platform Protocol, and the build crate surfaces them by running the real
contract validation. Two definitions of "valid" would drift the day one of
them changed.

## Persistence

`dash_sdk_contract::persistence` states the persistence semantics as enums so
generated wrappers and this chapter cannot drift:

| | Result |
|---|---|
| Construct or mutate a detached value | nothing is written |
| `documents::<T>().insert(value)` | stages a native create |
| `documents::<T>().edit(id, closure)` | loads one record and stages the update when the closure returns successfully |
| An exported `&mut self` entry returns successfully | its wrapper loads exactly the addressed record and stages the receiver update |
| `Drop`, an escaping reference, an unmarked helper | nothing is written |

Document ids, revisions, owners and storage flags are host managed. A receiver
entry on a document collection takes the target document id as its first wire
argument; a singleton receiver takes none, its key being reserved.

## Indexes and the native catalogue

`IndexSpec` expresses every index feature the native catalogue supports:
unique, null searchability, contested parameters (field matches, masternode
vote resolution, description), count and offset count, range count, sum and
range sum, average sugar, count ranking at the terminal level or at named
prefix levels, sum and average ranking, time-range buckets, and the index-only
options terminal, preallocated and skip-if-absent. Collection-level count,
sum, average and index-only flags, the document type switches, the bounded key
requirements and per-action token costs are on `CollectionSpec`. Every
reference target (identity, contract, token, permanent document with property
agreement, identity public key) is a `FieldType::Reference`.

Average sugar (`average = "p"`, `range_average`) is expanded into `count`
plus `sum = "p"` and `range_count` plus `range_sum` before the manifest,
with the same conflict rules the native parser applies to `averageable`; the
manifest has no average fields.

Index definitions on existing document types are frozen by the existing
versioned native update validation: adding, removing or changing an index on
an existing type is rejected, and a new type may declare indexes through
normal validation. The SDK adds no migration or backfill.

## Rules and native awards

A rule guards ordinary actions of one collection with either a native bounded
guard expression (`GuardExpr`, the author-facing form of the proposed guard
node set: literals, old, new and context field reads, existence and null
tests, comparisons, checked arithmetic, boolean operators and a conditional)
or a read-only WASM predicate exported by one of the contract's modules. The
rule's identity is its collection and name; its action set is stored sorted.

The contested award is unrepresentable. `ActionScope` has no award variant, a
contested index is declared only through its supported parameters, and a
contested unique index may sit next to rules on ordinary actions of the same
collection; those rules apply to ordinary writes and never to the award.

## Capabilities

The manifest's capability table lists everything the contract needs, explicit
(`requires = ["acl", "randomness"]`) or derived (`write = "contract"`, native
guards, WASM predicates, typed collections, the private store, stored
receipts, entries, several modules), each with its catalogue status:

| Status | Meaning |
|---|---|
| `Native` | supported by the native host today |
| `PendingNative` | declarable and specified, no native implementation yet; the build crate reports it as a gap and refuses to call the manifest deployable |
| `InterfaceDisabled` | catalogued, rejected by the validator until specified (the private store) |

Typed specialized collections (`TypedCollectionSpec`: sum, big sum, count,
count-and-sum, provable sum, provable count, ranked, append and commitment
families) are a manifest slot with a key type, an element type and an optional
element bound. Their operation sets, limits, privacy model and native adapters
are separate capability work; there is no raw path, raw element or database
handle anywhere in the model.

## Named modules

A package may build several WASM module targets. `ModuleSpec` names each and
lists the interfaces it imports; `InterfaceSpec` names a provider module and
its functions. The manifest records the sorted modules and interfaces and the
`(importer, provider, interface)` bindings, and rejects cycles and
self-imports. A package that declares no module has one implicit module named
`main`. An entry binds to one module; moving it between modules changes the
binding and nothing else, which a test pins by comparing the method table
before and after a move.

## The canonical manifest

`CanonicalManifest` is sorted throughout: modules and interfaces by name,
bindings by tuple, collections by name with fields by position and indexes by
name, typed collections by id, methods by name, rules by collection and name,
capabilities by requirement. Every table is keyed by a required unique
identity, so two declarations that differ only in source order, in attribute
versus builder origin, or in which module hosts an entry (beyond the binding
itself) produce equal manifests. The manifest is built only by the validator
and has no wire encoding or digest here.

## What is provisional

- The attribute spellings and option names.
- The export symbol scheme `dash_entry_<method name>`.
- The method, module, interface and rule name grammars and the implicit
  module name `main`.
- The meaning of `schema = N` (an author-declared schema revision recorded for
  compatibility reports).
- The diagnostic codes.
- The `GuardExpr` node set, which mirrors the proposed native guard grammar;
  the canonical guard AST is specified by the guards work.
- The home of the manifest shape in this crate, until the ABI work allocates
  its encoding.
