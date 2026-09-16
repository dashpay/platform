# Contract groups

Protocol version 14 adds contract groups: identity-owned sets of contracts, contract
document types and contract tokens. A group is registered by a data contract create
transition. Later contracts created by an owner of the group can add themselves, one
of their document types, or one of their tokens to the group in their own create
transition. Groups let a project describe "these contracts, types and tokens belong
together" in consensus state, provably, for anything that later wants to act on the
set as a whole.

Contract groups are unrelated to the `groups` a data contract declares for token
change control. Those are multi-party groups of identities inside one contract; a
contract group is a set of contracts owned by identities.

## Registering a group

Version 1 of `DataContractCreateTransition` carries two new fields:

- `contractGroup`: an optional registration `{ owner, name?, description? }`.
- `contractGroupMemberships`: a list of `{ contractGroupId, member }`.

Version 0 stays valid and carries neither. Below protocol version 14 a version 1
transition is rejected by the version bounds, as any unknown version is.

The group id is not on the wire. It is derived from the registering identity and the
transition's identity nonce:

```text
contractGroupId = hash_double("contract_group" || ownerId || identityNonce (u64, big endian))
```

The contract created by the same transition has id `hash_double(ownerId || identityNonce)`,
so a client knows both ids before broadcasting and the two can never collide.

`owner` is one of:

- `singleOwner(identityId)`: one identity owns the group and is the only one who may add
  members to it.
- `ownerAndAdmins { owner: identityId, admins: [identityId, ...] }`: one identity owns the
  group and the admins may add members alongside it. At least one admin, at most
  `maxContractGroupAdmins` (16), none of them the owner. Admins act alone; there is no
  threshold.

The registering identity must be the owner in both forms; being an admin is not enough to
register a group. `name` is 1 to 64 characters and
`description` 1 to 256 characters when present. A group may be registered empty.

## Joining a group

Each membership names a group and which part of the created contract joins:

- `contract`: the whole contract, every document type and token it has or will have.
- `documentType(name)`: one document type, which must exist in the contract.
- `token(position)`: one token, which must exist in the contract at that position.

A contract may only enrol itself. The group must exist, or be the one registered by
the same transition, and the creating identity must be its owner or one of its admins. At most
`maxContractGroupMembershipsPerContract` (16) memberships per transition. A membership
may not repeat, and a document type or token membership is rejected when the whole
contract already joins the same group.

Memberships are recorded at creation only. There is no update path and no leaving.

## Validation and fees

- Structure (unpaid): owner rules, text lengths, member existence in the contract,
  duplicates, redundancy and the membership cap.
  Errors 10360 to 10367.
- State (paid, identity nonce bumped): the registered group must not exist
  (`ContractGroupAlreadyExistsError`, 41000), every group joined must exist
  (`ContractGroupNotFoundError`, 41001) and have the signer as its owner or an admin
  (`IdentityNotContractGroupOwnerOrAdminError`, 41002). Each group lookup is billed.

Storage is paid at the standard rate: the info item and three empty subtrees for a
registration, one empty item plus one reference per membership, plus the trees a new
contract needs on the backwards side.

## Storage layout

A new root tree, `ContractGroups` (key `68`), holds every group and a backwards index
from each member contract:

```text
[68] ContractGroups
├── [0] Groups
│   └── <contract group id>
│       ├── [0] Info            -> Item(bincode ContractGroupInfo { owner, name?, description? })
│       ├── [1] Contracts       -> <contract id> -> Item([])
│       ├── [2] DocumentTypes   -> <contract id> -> <document type name> -> Item([])
│       └── [3] Tokens          -> <contract id> -> <token position, u16 BE> -> Item([])
└── [1] Members
    └── <contract id>
        ├── [0] Groups          -> <contract group id> -> Reference to Groups/<group>/[1]/<contract id>
        ├── [1] DocumentTypes   -> <document type name> -> <contract group id> -> Reference
        └── [2] Tokens          -> <token position> -> <contract group id> -> Reference
```

The backwards entries are plain GroveDB references (`UpstreamRootHeightReference`
keeping the root tree key, one hop). Memberships are append-only and contracts are
never deleted, so a reference can never dangle. The `Members` side is keyed by
contract first so "is document type D of contract C in group G" is one point lookup.

Fresh chains create the root tree and its two subtrees in the initial state structure
(version 4). Chains upgrading to protocol version 14 create them on the first block of
the new version.

## Proofs

Two path queries cover the tree:

- `Drive::contract_group_query(groupId)`: the info item and every member of one group,
  verified with `Drive::verify_contract_group`, which returns `None` for an absent group.
- `Drive::contract_group_memberships_for_contract_query(contractId)`: every group a
  contract belongs to, verified with `Drive::verify_contract_group_memberships_for_contract`.

Both are single GroveDB proofs against the state root. DAPI queries and SDK surfaces
for them follow separately.
