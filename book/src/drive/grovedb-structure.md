# The GroveDB Structure

Drive keeps everything in one GroveDB: a tree of trees. The root layer holds
one subtree per `RootTree` variant, each of those holds fixed keys or one
subtree per identity, contract or token, and so on down to the items. The other
chapters of this section draw the part they are about. This chapter is about
the whole, and about the two things that keep a picture of it honest: a
description written as code, and a viewer that reads it.

**[Open the structure viewer](https://dashpay.github.io/grovedb-structure-viewer/)**

<iframe
  src="https://dashpay.github.io/grovedb-structure-viewer/?embed=1"
  title="GroveDB structure viewer"
  loading="lazy"
  style="width: 100%; height: 640px; border: 1px solid var(--table-border-color); border-radius: 8px;">
</iframe>

Dive into a tree to go a layer down, use the breadcrumb or Backspace to come
back up, drag the protocol version at the bottom to see what each version
added, and switch a layer to *Merk tree* to see its real binary tree.

## The description is code

The structure is declared in Rust, in `drive::structure`
(`packages/rs-drive/src/structure`). Each area declares its own part in a
`structure.rs` beside its `paths.rs`, from the same constants the paths use:

```rust
StructureNode::fixed(
    "balances",
    &[TOKEN_BALANCES_KEY],
    "TokenBalances",
    "TOKEN_BALANCES_KEY",
)
.kind(ElementKind::BigSumTree)
.since(9)
.describe("Who holds how much of each token.")
.child(
    token("The balances of one token; the sum is its circulating supply.")
        .kind(ElementKind::SumTree)
        .child(
            StructureNode::identifier("identity", "identity_id", "The holder's identity id")
                .kind(ElementKind::SumItem)
                .value("token amount")
                .describe("One identity's balance of the token."),
        ),
)
```

A node says:

- **its key**: fixed bytes and the constant they come from, or a template
  such as "identity id, 32 bytes" standing for many keys;
- **the element kinds** that can sit there. Several when the code chooses, as
  the primary key tree of a document type does between eight tree kinds;
- **the element flags** on it. GroveDB stores a byte string of flags with
  every element and never reads it; Drive keeps its storage flags there: the
  epoch the element's bytes were paid for in, the bytes added in later epochs
  if it grew, and for owned flags the identity that paid, which is who a
  refund goes to when the element is deleted or shrinks. A node says whether
  its element carries none, storage flags, or storage flags with an owner, and
  who that owner is. The exported file explains each kind and its byte layout
  once, under `flag_kinds`;
- **`since`**, the first protocol version it exists in, which children inherit;
- whether it is created with its parent, **lazily** on first use, or with its
  parent and **deleted later** while the parent stays, as an epoch's storage
  fee item is at payout;
- what an item holds, what a reference points to, and `recurse` where the
  structure repeats to any depth, as index levels do;
- its source file and, when there is one, its chapter in this book.

The identifier of a node is the dotted path of its segments
(`tokens.balances.token.identity`). Identifiers are what two versions of the
structure are compared by, so treat them as append-only: renaming one reads as
a removal and an addition.

The module is compiled for tests and under the `structure` feature only.
Nothing in the node depends on it.

## What keeps it true

The tests in `packages/rs-drive/src/structure/tests.rs` stand between the
description and drift. A lint first checks the description against itself:
identifiers are unique, reference and `recurse` targets exist, every source
file exists and names the constant a key claims to come from, and no two
templates of a layer could claim the same element.

**Conformance.** `check_conformance` walks a real GroveDB layer by layer and
reports every element no node describes, every element of a kind its node
does not list, every element whose flags are of a kind its node does not list, every element outside its node's protocol versions, and every
node that should have been created with its parent but is missing. Where
several templates of a layer accept a key, the one whose description fits what
is below the element wins: below a contested index a 32 byte key is a
contender's identity id at the last level and an index value before it, and
the key alone cannot tell. It runs against the initial state structure of
**every protocol version**, which pins each `since` to what
`create_initial_state_structure` really builds, and against populated state:
identities, contracts with documents and history, tokens, group actions,
address balances, an epoch before and after payout, contested documents. A
change that adds a root tree, a subtree key or a level fails here until it is
described.

**The strategy tests.** The drive-abci strategy tests check the state of every
chain they run with the same walker (`assert_state_conforms_to_structure`), at
whatever protocol version the chain ended on. They write far more than the
fixtures do: votes, withdrawals, token distributions, epochs changing, protocol
upgrades. Whatever a change's own strategy tests write is walked, so structure
created only during some operation is caught too.

**Coverage.** Every node must be reached by some rs-drive fixture or be listed
as reached by the strategy tests. `UNVERIFIED`, the list for nodes written from
reading the code and never checked against a real GroveDB, is empty and meant
to stay so: whoever describes a node can write a fixture that creates it. The
test also fails when a listed node does get reached by a fixture, so the lists
can only shrink.

**The exported file.** The description is serialized to
`packages/rs-drive/grovedb-structure.json`, which is what the viewer reads. A
test fails when the committed file is stale:

```bash
UPDATE_GROVEDB_STRUCTURE=1 cargo test -p drive --lib structure::tests
```

## The shape of a layer

GroveDB stores each layer as a Merk, a balanced binary tree, and a proof of
one element carries the hashes along the way from the layer's root. So where a
key sits matters twice: keys near the top have shorter proofs, and every write
below a node rewrites its ancestors in the layer. That is why the root keys
are spread over the byte range with `DataContractDocuments` on top, and why
`ContractGroups` took key 124, which hangs below `Versions`, a tree only the
block-level version bookkeeping writes to (see
[Contract Groups](../data-model/contract-groups.md#storage)).

The exported file records the exact Merk of every layer whose keys are all
fixed, under `layer_shapes`. GroveDB does not expose the links between the
nodes of a Merk, but a proof does: the proof of a query for everything in a
layer lists every node and how they connect, so replaying its operations
rebuilds the tree. The viewer draws it when you switch a layer to *Merk tree*.

The shape depends on the order of insertion, not only on the keys. For a
layer reached through fixed keys only, the recorded shape is that of a fresh
chain at the latest protocol version (`"origin": "genesis@14"`). A chain that
upgraded through earlier versions inserted the same keys in another order and
can differ.

A layer below a template exists once per identity, contract, epoch and so on.
Its shape is recorded from one instance: the fullest one the test fixtures
build (`"origin": "fixture contracts_with_documents@14"`). That is where
layouts designed around the Merk show: a contract's layer keeps its documents
on top with the contract and everything else below, the `other` tree keeps
the banlist in the middle, and an identity's seven keys form a full tree with
the keys at the root. Another instance can differ when it holds fewer keys or
got them in another order.

Some layers gain and lose keys over their life, and then one shape is not
enough. An epoch's layer is created at genesis holding only its storage fees;
its first block adds the start fields, the proposers tree and the processing
fees in one batch; and the batch that pays it out deletes the proposers and
both fee items and writes the finished epoch info. No epoch ever holds all nine
keys. A node declares such **states** in the order the layer goes through
them, each with a title, what it means and what moves the layer into it, and
the keys it holds then:

```rust
.state(
    "paid",
    "Paid out",
    "One batch pays the proposers, deletes the proposers tree and both fee \
     items, and writes the finished epoch info.",
    &["start_block_core_height", "finished_epoch_info", "start_block_height", ...],
)
```

The fixtures record one shape per state, and a state's shape must come from an
instance holding exactly the keys the state declares. The viewer shows them as
*State 0*, *State 1*, ... with their explanations. What matters for a faithful
shape is that the fixture writes the same keys in the same batches as the
block pipeline does, since a Merk batch of several keys gives another tree
than the same keys written one at a time.

## Pull requests that change the structure

When a pull request changes `grovedb-structure.json`, the
`GroveDB Structure Preview` workflow comments with a link that opens the viewer
on the difference between the merge base and the head of the pull request:
new nodes glow, removed ones stay as ghosts, every ancestor of a change
carries a count so the trail is visible from the root, and a tour walks
through each change. The viewer fetches both files from GitHub and compares
them itself, so the link works before the merge and for forks.

To add structure, follow the
[checklist](../contributing/coding-conventions.md#checklists) in the coding
conventions.
