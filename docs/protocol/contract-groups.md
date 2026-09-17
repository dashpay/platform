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

- `contractGroup`: an optional registration `{ admins?, name?, description? }`.
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

The owner is the identity that signs the transition; it is not on the wire. `admins`, when
present, names the identities that may add members alongside the owner: at most
`maxContractGroupAdmins` (16), none of them the owner, every one an existing non-masternode
identity. Admins act alone; there is no threshold. Stored, the ownership is `singleOwner` when
no admin is named and `ownerAndAdmins { owner, admins }` otherwise. `name` is 1 to 64 characters and
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

- Structure (unpaid): admin count and the owner not among the admins, text lengths, member existence in the contract,
  duplicates, redundancy and the membership cap.
  Errors 10360 to 10367.
- State (paid, identity nonce bumped): the registered group must not exist
  (`ContractGroupAlreadyExistsError`, 41000), every group joined must exist
  (`ContractGroupNotFoundError`, 41001) and have the signer as its owner or an admin
  (`IdentityNotContractGroupOwnerOrAdminError`, 41002), and every admin named by a
  registration must be an existing non-masternode identity
  (`ContractGroupAdminNotFoundError`, 41003). Each lookup is billed, as is the group id
  derivation.

Storage is paid at the standard rate: the info item and three empty subtrees for a
registration, one empty item plus one reference per membership, plus the trees a new
contract needs on the backwards side.

## Storage layout

A new root tree, `ContractGroups` (key `124`, under the `Versions` node so no fee-bearing
transition pays for the extra child), holds every group and a backwards index
from each member contract:

```text
[124] ContractGroups
├── [0] Groups
│   └── <contract group id>
│       ├── [0] Info            -> Item(bincode ContractGroupInfo { owner, name?, description? })
│       ├── [1] Contracts       -> <contract id> -> Item([])
│       ├── [2] DocumentTypes   -> <contract id || document type name> -> Item([])
│       └── [3] Tokens          -> <contract id || token position, u16 BE> -> Item([])
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

A group can be joined by any number of contracts, so nothing reads or proves a whole group
at once. Three path queries cover the tree:

- `Drive::contract_group_info_query(groupId)`: the info item alone, verified with
  `Drive::verify_contract_group_info`, which returns `None` for an absent group.
- `Drive::contract_group_members_query(groupId, query, limit)`: one page of one kind of
  member, `contracts`, `documentTypes` or `tokens`, in key order, at most `limit` entries,
  continuing after the cursor the query carries (a contract id, or a contract id with a
  document type name or a token position). Verified with
  `Drive::verify_contract_group_members` for the same query and limit. The limit must be
  between 1 and the node's maximum query limit, so no proof grows with the size of a group.
  An absent group proves as an empty page.
- `Drive::contract_group_memberships_for_contract_query(contractId)`: every group a
  contract belongs to, verified with `Drive::verify_contract_group_memberships_for_contract`.
  It needs no limit: a contract's memberships are recorded at creation only, at most
  `maxContractGroupMembershipsPerContract` of them.

Each is a single GroveDB proof against the state root. DAPI exposes them as
`getContractGroupInfo`, `getContractGroupMembers` (the `members` oneof names the kind and
carries its cursor; `limit` defaults to and is capped by 100) and
`getContractGroupsForContract`, each with a `prove` flag. `rs-drive-proof-verifier` verifies
the responses into `ContractGroupInfo`, `ContractGroupMembersPage` and
`ContractGroupMembershipsForContract`, which implement `Fetch` and `FetchUnproved` in `rs-sdk`.

## Keys bound to a group

An identity's AUTHENTICATION key may carry `contractBounds` of type `contractGroup`
naming a group. The key may then sign batch members whose contract, document type or
token is a member of the group; consensus reads the member contract's memberships once
per batch member and bills the read. The group must exist when the key is registered,
any identity may bind a key to any group, and encryption and decryption keys cannot be
group-bound. `IdentityCreateFromShieldedPool` refuses group-bound keys. The rules and
errors are in `contract-bound-authentication-keys.md`.
