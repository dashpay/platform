# Contract Groups

An application often outgrows one data contract. Indexes cannot be added to a document type after the contract is registered, so a project that grows tends to ship a second contract beside the first rather than reshape the original. A token is usually best kept in a contract of its own, so its change-control groups and supply rules are not entangled with an application's document types. And a project that publishes a version 2 contract keeps version 1 alive for the documents already stored in it. The result is a family of contracts under one owner, and before protocol version 14 nothing in consensus state said they belonged together. A client could guess from the owner identity, but that identity might own unrelated contracts too, and a guess cannot be proved.

A **contract group** is the answer. It is an identity-owned set of contracts, contract document types and contract tokens. It is registered and grown through ordinary data contract create transitions, stored under its own root tree, and readable with one GroveDB proof. Anything that later needs to act on "the set as a whole", from an SDK deciding which contracts to preload to a future identity key bound to every contract in a group, gets a consensus-level answer to "which contracts are in this set".

> **Not the same thing as `groups`.** A data contract declares `groups` for token change control (see [Data Contracts](data-contracts.md)). Those are multi-party sets of *identities* inside one contract, with voting power and a required threshold. A contract group is a set of *contracts* owned by identities. The two share a word and nothing else. The `SystemLimits` field that bounds a change-control group's members was renamed from `max_contract_group_size` to `max_group_member_count` in the same protocol version so the code does not confuse them either.

## The Model

Three facts define a contract group:

1. **One identity owns it.** The owner is the identity that signed the registering transition, and it is never on the wire. It can stand alone or name up to sixteen admins in the registration, existing identities other than itself that may add members alongside it. Admins act alone: there is no threshold and no vote.
2. **Its members are parts of contracts.** A member is a whole contract, one document type of a contract, or one token of a contract. A contract can only enrol itself. Memberships are declared by the create transition of the contract that joins, never by a third party and never for someone else's contract.
3. **It is append-only.** Memberships are recorded when the member contract is created. There is no update path and no leaving. Contracts are never deleted either. Together these two facts are what let the storage layout use plain references safely, as the storage section explains.

A group is registered inside a data contract create transition. The same transition may enrol the contract it creates, and later create transitions signed by any owner add more members. A group may also be registered empty and filled later.

### How a Group Gets Its Id

The group id is never on the wire. It is derived from the registering identity and the transition's identity nonce, in `packages/rs-dpp/src/contract_group/mod.rs`:

```rust
pub const CONTRACT_GROUP_ID_DOMAIN: &[u8] = b"contract_group";

pub fn generate_contract_group_id(
    owner_id: &Identifier,
    identity_nonce: IdentityNonce,
) -> Identifier {
    let mut bytes = CONTRACT_GROUP_ID_DOMAIN.to_vec();
    bytes.extend_from_slice(owner_id.as_slice());
    bytes.extend_from_slice(&identity_nonce.to_be_bytes());
    Identifier::from(hash_double(bytes))
}
```

Compare the id of the contract created by the same transition: `hash_double(owner_id || identity_nonce)`. The domain prefix keeps the two from ever colliding, and because both inputs are known before signing, a client knows the group id and the contract id before it broadcasts. That is what makes the one-transition case work: a create transition can register a group and, in the same membership list, join it by the id it is about to receive.

## The Types

Everything a transition carries about contract groups lives in `packages/rs-dpp/src/contract_group/mod.rs`. There are four wire types and one stored type.

```rust
/// Who owns a contract group, and so who may add members to it.
pub enum ContractGroupOwner {
    SingleOwner(Identifier),
    OwnerAndAdmins {
        owner: Identifier,
        admins: BTreeSet<Identifier>,
    },
}

/// What part of the contract being created joins a contract group.
pub enum ContractGroupMember {
    Contract,
    DocumentType(DocumentName),
    Token(TokenContractPosition),
}

/// A declaration that a part of the created contract joins a contract group.
pub struct ContractGroupMembership {
    pub contract_group_id: Identifier,
    pub member: ContractGroupMember,
}

/// The registration of a new contract group. The owner is the signer and is not on the wire.
pub struct ContractGroupRegistration {
    pub admins: BTreeSet<Identifier>,
    pub name: Option<String>,
    pub description: Option<String>,
}
```

The owner is deliberately absent from the registration: the only valid value would be the transition's signer, so carrying it would add thirty-two signed bytes and an error for the one mistake that field could express. The admins are a `BTreeSet`, so a duplicate admin cannot be expressed and the encoding is canonical.

`ContractGroupOwner` is the stored form of ownership, built from the signer and the registration when the transition becomes an action: an empty admin set stores `SingleOwner`, anything else `OwnerAndAdmins`. It has the helpers validation needs: `owner_id()`, `admin_ids()`, `admin_count()` and `may_add_members(&identity_id)`, which is true for the owner and for every admin.

The stored type wraps the owner with a version envelope, because it is persisted and must remain decodable forever:

```rust
#[platform_serialize(unversioned)]
pub enum ContractGroupInfo {
    V0(ContractGroupInfoV0),
}

pub struct ContractGroupInfoV0 {
    pub owner: ContractGroupOwner,
    pub name: Option<String>,
    pub description: Option<String>,
}
```

`ContractGroupInfo` derives `PlatformSerialize`, `PlatformDeserializeTrusted` and `PlatformDeserializeUntrusted` (see [Derive Macros](../serialization/derive-macros.md)). The trusted decoder reads this node's own GroveDB; the untrusted one decodes bytes that arrived inside a proof. `From<ContractGroupRegistration>` builds the `V0` variant, so the transition never carries a version tag for the info: Drive chooses the stored version.

Two details of the wire types are easy to miss. Both `ContractGroupRegistration` and `ContractGroupMembership` implement `JsonSafeFields` in `packages/rs-dpp/src/serialization/json/safe_fields.rs`, which the `json_safe_fields` derive on the transition requires for every field type. And the `ContractGroupInfo` module imports `ProtocolError` at the top even though no code in it names the type, because the `PlatformSerialize` derive expands to code that does.

## Version 1 of the Create Transition

`DataContractCreateTransition` gains a second variant. Version 0 is unchanged and stays valid; version 1 is version 0 plus the two contract group fields. From `packages/rs-dpp/src/state_transition/state_transitions/contract/data_contract_create_transition/v1/mod.rs`:

```rust
pub struct DataContractCreateTransitionV1 {
    pub data_contract: DataContractInSerializationFormat,
    pub identity_nonce: IdentityNonce,
    pub contract_group: Option<ContractGroupRegistration>,
    pub contract_group_memberships: Vec<ContractGroupMembership>,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}
```

Both new fields sit inside the signed payload. A plain contract creation in version 1 carries `contract_group: None` and an empty membership list, which is exactly what the `TryFromPlatformVersioned<CreatedDataContract>` conversion produces, so existing callers that build a transition from a contract keep working and simply emit version 1 under protocol version 14.

In JSON the transition looks like this (identifiers abbreviated):

```json
{
  "$formatVersion": "1",
  "dataContract": { "...": "..." },
  "identityNonce": 7,
  "contractGroup": {
    "admins": ["GWRSAVFM…S31Ec"],
    "name": "cardgame",
    "description": "Rules, marketplace and token contracts of the card game"
  },
  "contractGroupMemberships": [
    { "contractGroupId": "8sJ6Rk…Q2mV", "member": "contract" },
    { "contractGroupId": "4hYb2N…kW9p", "member": { "documentType": "listing" } },
    { "contractGroupId": "4hYb2N…kW9p", "member": { "token": 0 } }
  ],
  "userFeeIncrease": 0,
  "signaturePublicKeyId": 1,
  "signature": "…"
}
```

The first membership joins the group this very transition registers, by its derived id. The other two join a group registered earlier, one the signer owns or administers. A group with a single owner leaves `admins` out or empty; the owner never appears, since it is the signer. Both group fields may be omitted from a version 1 object, which then reads as a plain creation.

### Version Bounds

Which variant a client may send is governed by `STATE_TRANSITION_SERIALIZATION_VERSIONS_V3`, the table protocol version 14 selects (see [Feature Versions](../versioning/feature-versions.md)):

```rust
contract_create_state_transition: FeatureVersionBounds {
    min_version: 0,
    max_version: 1,
    default_current_version: 1,
},
```

Below protocol version 14 a version 1 transition has to be refused before anything looks inside it, and the bounds table above does not do that on a node: only client-side code consults it. The gate is `StateTransition::active_version_range`, which answers 14 to the latest version for a version 1 create, so `deserialize_from_bytes_untrusted_in_version` fails with `StateTransitionIsNotActiveError` on a protocol version 13 node. Without that arm an upgraded node would create the contract, charge the fee and silently drop the group data while a pre-upgrade binary rejected the same bytes. At protocol version 14 the default moves to 1, which is why the factory test in `packages/rs-dpp/src/data_contract/factory/v0/mod.rs` compares the version of the transition it builds against `default_current_version` rather than a literal.

### Accessors

Code reads the new fields through `DataContractCreateTransitionAccessorsV1`, following the same pattern the [Data Contracts](data-contracts.md) chapter describes for `DataContractV1Getters`:

```rust
pub trait DataContractCreateTransitionAccessorsV1 {
    fn contract_group(&self) -> Option<&ContractGroupRegistration>;
    fn contract_group_id(&self) -> Option<Identifier>;
    fn contract_group_memberships(&self) -> &[ContractGroupMembership];
}
```

A version 0 transition answers `None`, `None` and an empty slice. Validation and Drive never match on the variant; they ask the accessors and treat "no group data" as the ordinary case. `contract_group_id()` derives the id from the contract's owner and the transition's nonce, so nothing downstream recomputes the hash by hand.

## Validation

Contract group checks slot into the existing [validation pipeline](../state-transitions/validation-pipeline.md) at two stages, and the split between them is the important design decision. Everything that can be decided from the transition alone runs in basic structure, before the signature is checked, and costs nothing. Everything that needs a state lookup runs in state validation, after the signer is authenticated, and is paid for. Without that split an attacker could probe which group ids exist for free.

### Basic Structure (Unpaid)

`contract_group_basic_structure_error` in `packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/data_contract_create/basic_structure/v2/mod.rs` runs after the existing contract checks and returns the first violation it finds, in this order:

1. If the transition registers a group, its `admins`, when any, must number at most `max_contract_group_admins` (16) and must not include the signer, who is the owner.
2. `name`, when present, must be 1 to `max_contract_group_name_length` (64) characters. `description`, when present, 1 to `max_contract_group_description_length` (256). Lengths count characters, not bytes.
3. The membership list must hold at most `max_contract_group_memberships_per_contract` (16) entries.
4. For each membership: a `DocumentType` member must name a document type of the created contract, and a `Token` member must name a token position the contract defines.
5. No `(group id, member)` pair may repeat.
6. A document type or token membership is redundant, and rejected, when the whole contract joins the same group in the same transition.

It is a free function rather than a method on the transition because drive-abci cannot add inherent methods to a type defined in dpp; the orphan rule forbids it.

### State (Paid)

`validate_contract_groups_against_state` in `.../data_contract_create/state/v1/mod.rs` runs after the version 0 state checks (the contract must not already exist) and after the `refersTo` reference validation. It bills every lookup on the execution context and stops at the first failure:

1. The group the transition registers must not exist yet. Its id was derived when the transition became an action, and that double hash is billed here, as the contract id derivation is.
2. Every admin the registration names must be an existing non-masternode identity, the check change-control group members get; a missing one fails with `ContractGroupAdminNotFoundError`. Memberships are creation-only, so a mistyped admin could never be replaced.
3. Every group a membership names must exist and must have the signer as its owner or one of its admins. The group registered by this same transition is skipped, since the signer owns it by construction, and each other group is fetched once however many memberships name it.

A failure here returns a `BumpIdentityNonceAction` carrying the errors: the identity pays for the lookups and its nonce advances, exactly as for any other paid validation failure.

### The Errors

| Code | Error | Stage |
|------|-------|-------|
| 10360 | `ContractGroupMembershipsOverLimitError` | structure |
| 10361 | `DuplicateContractGroupMembershipError` | structure |
| 10362 | `RedundantContractGroupMembershipError` | structure |
| 10363 | `ContractGroupMemberNotInContractError` | structure |
| 10364 | `InvalidContractGroupAdminsError` | structure |
| 10366 | `InvalidContractGroupNameLengthError` | structure |
| 10367 | `InvalidContractGroupDescriptionLengthError` | structure |
| 41000 | `ContractGroupAlreadyExistsError` | state |
| 41001 | `ContractGroupNotFoundError` | state |
| 41002 | `IdentityNotContractGroupOwnerOrAdminError` | state |
| 41003 | `ContractGroupAdminNotFoundError` | state |

The basic errors live in `packages/rs-dpp/src/errors/consensus/basic/contract_group/` and the state errors in `.../consensus/state/contract_group/`. Both sets were appended at the tail of their enums; `StateError` has a frozen-discriminant test that pins the four new variants, so an insertion before them fails it. Code 10365 is unassigned: the registrant-not-owner rule it served became inexpressible once the owner left the wire. The basic codes follow the change-control group range as their own block, and the state codes open a new hundred, 41000 to 41099, rather than borrowing from the identity range. See [Error Codes](../error-handling/error-codes.md) for the code ranges.

## From Transition to Action to Operations

The [transform into action](../state-transitions/transform-into-action.md) step gets a version 1 action to match the version 1 transition:

```rust
pub struct DataContractCreateTransitionActionV1 {
    pub data_contract: DataContract,
    pub identity_nonce: IdentityNonce,
    pub user_fee_increase: UserFeeIncrease,
    pub contract_group: Option<(Identifier, ContractGroupInfo)>,
    pub contract_group_memberships: Vec<ContractGroupMembership>,
}
```

Two things happen in the transformer (`packages/rs-drive/src/state_transition_action/contract/data_contract_create/v1/transformer.rs`). The registration becomes a `ContractGroupInfo`, the stored form, with the signer as owner, and the group id is derived once and carried alongside it, so nothing after this point needs the transition's owner and nonce; state validation reads the id from the action rather than hashing again. The `BumpIdentityNonceAction` transformer gained matching version 1 arms so a paid failure can still be turned into a nonce bump.

Converting the action into [Drive operations](../state-transitions/drive-operations.md) is where ordering matters. The version 1 arm of `into_high_level_drive_operations` emits, in this order:

1. `UpdateIdentityNonce` and `UpdateIdentityContractNonce`, as version 0 does.
2. `ApplyContract` for the contract itself.
3. `RegisterContractGroup { contract_group_id, info }`, if the transition registers a group.
4. `AddContractGroupMemberships { contract_id, memberships }`, if it declares any.

The contract is applied before anything touches the group trees, and the registration comes before the memberships, so a contract may join the group its own transition registers: by the time the membership operation runs, the group's subtrees exist. Both operations are variants of `ContractGroupOperationType` in `packages/rs-drive/src/util/batch/drive_op_batch/contract_group.rs`, and they resolve to low-level GroveDB operations through the same `DriveLowLevelOperationConverter` machinery every other high-level operation uses (see [Batch Operations](../drive/batch-operations.md)).

## Storage

Contract groups get their own root tree, `RootTree::ContractGroups`, at key `124`. The number matters: a root key becomes a child of some existing node in the root Merk, and every write under that node's subtree then rewrites a slightly larger node. Key 124 attaches under `Versions` (120), which only the block-level app-version update writes, so no fee-bearing transition pays for it. A lower free key such as 68 would have attached under the asset-lock outpoints node and raised every identity create and top-up fee by 1480 credits for the life of the chain; the four conversion impls in `packages/rs-drive/src/drive/mod.rs` and the `KnownPath` mapping in the batch module were extended for it. Path helpers and the single-byte subtree keys live in `packages/rs-drive/src/drive/contract_groups/paths.rs`.

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

`Groups` is the forward store: everything about one group under its id. `Members` is the backwards index: everything one contract belongs to, under the contract id. Every leaf on the forward side is an empty item; the key carries all the information. Document type and token members sit on one level under a composite key, the contract id followed by the UTF-8 name or the two big-endian position bytes, rather than under a subtree per contract. The keys still sort by contract, and a flat level pages with a plain range after the cursor key. A subtree per contract would have forced a continuation page to descend into the cursor's contract, and GroveDB charges an empty descent against the page limit, which shortened every continuation page and ended a limit-one walk early. The decoders rebuild typed values from keys alone.

### Why Plain References

The backwards entries are GroveDB `Reference` elements of type `UpstreamRootHeightReference(1, …)`: keep the first path segment (the root tree key) and append the forward path, one hop. They are not GroveDB bidirectional references. Bidirectional references buy automatic cleanup when the target moves or disappears, at a cost on every write. Here neither can happen: memberships are append-only and contracts are never deleted, so a reference can never dangle. Plain references are cheaper and the invariant holds by construction.

### Why Members Is Keyed by Contract First

The `Members` side is laid out so that "is document type `D` of contract `C` in group `G`" is one point lookup at `[124, 1, C, 1, D, G]`, and the forward side answers it at `[124, 0, G, 2, C || D]`. That is the shape `ContractBounds::ContractGroup` on identity keys needs: for every member of a batch, one existence check under the contract the member belongs to. Today the batch transformer reads the contract's whole `Members` entry (at most `max_contract_group_memberships_per_contract` entries) into the action, for a batch signed by a group-bound key only, and the check runs in memory; the point lookup is there when a consumer wants it.

### Writing

Registration (`insert_contract_group_operations_v0`) inserts the group's tree under `Groups` with `batch_insert_empty_tree_if_not_exists`, and if the tree was already there returns `CorruptedDriveState`. State validation guarantees the group is new, so hitting that branch means the two disagree, which is exactly what a corrupted-state error is for. It then writes the `Info` item and the three empty member subtrees.

Memberships (`insert_contract_group_memberships_operations_v0`) write one empty item on the forward side and one reference on the backwards side per membership, creating the intermediate trees on both sides as needed, each with an if-not-exists insert so a tree left by an earlier declaration of the same contract is kept rather than overwritten once memberships can be added after creation. Two memberships often need the same parent tree, for instance two document types of the same contract joining the same group both need `Groups/<G>/DocumentTypes/<C>`. GroveDB rejects a batch that operates twice on one slot under batching consistency verification, so the inserter keeps a `HashSet` of tree paths it has already created in this batch and inserts each parent once.

### Cost Estimation

Both inserts have an estimation twin under `estimated_costs/` that declares `EstimatedLayerInformation` for every layer the write can touch (see [Cost Tracking](../drive/cost-tracking.md)): the root, the `ContractGroups` tree, the `Groups` and `Members` levels, a group's own tree as a mix of three subtrees and one item, and so on down. Three constants size the unknowns: an info item is estimated at 1024 bytes (a sixteen-owner group with a full-length name and description stays under it), a backwards reference at 128 bytes, and a document type name key at 16 bytes. The Drive test `should_build_the_same_operations_for_estimation_and_for_apply` pins the estimation path to the apply path so the two cannot drift apart silently.

### Creating the Trees

One helper, `Drive::insert_contract_groups_structure`, creates the root tree and its two subtrees. A fresh chain calls it from `create_initial_state_structure_v4` (`packages/rs-drive/src/drive/initialization/v4/mod.rs`), which `DRIVE_VERSION_V9` selects; a chain upgrading to protocol version 14 calls it from `transition_to_version_14`, the migration hook the [Versioned Dispatch](../versioning/versioned-dispatch.md) chapter describes. Sharing the helper is what keeps a node started at 14 and a node upgraded to it byte-identical under this key, the same reason the shielded pool has `insert_shielded_pool_structure`. Both subtrees are created up front so a registration only ever writes under `Groups` and a membership only under `Groups` and `Members`; no write path has to check whether the top of the tree exists.

## Reading and Proving

A group can be joined by any number of contracts, so nothing reads or proves a whole group at once. Drive splits reading into the group's information, a single item, and its members, read one kind at a time in pages bounded by a limit. Everything is versioned through `DriveContractGroupMethodVersions`:

- `fetch_contract_group_info(group_id)` reads only the `Info` item. State validation uses its `_with_fee` variant, which returns the `FeeResult` for the lookup alongside the result. `prove_contract_group_info` and `verify_contract_group_info` do the same through a proof, with `None` for a group that is provably absent.
- `fetch_contract_group_members(group_id, &query, limit)` returns one `ContractGroupMembersPage` of one kind. The `ContractGroupMembersQuery` names the kind, `Contracts`, `DocumentTypes` or `Tokens`, and carries a `start_after` cursor: a contract id alone, or a contract id with a document type name or a token position. Entries come back in key order, at most `limit` of them, and `page.next_query()` is the query for the page after it, `None` once a page is empty. `limit` must lie between 1 and the node's `max_query_limit`, on the fetch side and on the proof side, so no page and no proof grows with the size of the group. `prove_contract_group_members` and `verify_contract_group_members` take the same query and limit.
- `fetch_contract_group_memberships_for_contract(contract_id)` returns a `ContractGroupMembershipsForContract`: the groups the whole contract joined, the groups each document type joined and the groups each token joined. It needs no limit: memberships are recorded at creation only and capped per transition, so a contract belongs to at most sixteen. `prove_contract_group_memberships_for_contract` and `verify_contract_group_memberships_for_contract` are its proof pair.

Three path queries in `packages/rs-drive/src/drive/contract_groups/queries.rs` drive all of them. `contract_group_info_query` asks for the one info key under the group's tree. `contract_group_members_query` reads one kind's level: the whole level on a first page, or a range after the cursor's key on a continuation, where the key is the contract id for `Contracts` and the contract id followed by the name or position for the other two kinds. There is no subquery to descend into, so a page can end anywhere and the next page resumes exactly after it. `contract_group_memberships_for_contract_query` reads a contract's `Members` entry with a conditional subquery per kind. The same `PathQuery` is used to fetch and to prove, so what a node reads locally and what a client verifies are the same set of elements.

Verification lives in `packages/rs-drive/src/verify/contract_groups/`. Each verify function rebuilds its result from the proved `(path, key, element)` triples with the same decoders the fetches use, in `types.rs`. Member pages carry no data in their elements: every entry is read from its path and key. The info item is the one thing that decodes bytes, and the verifier uses the untrusted `ContractGroupInfo` decoder because proof bytes came from someone else, where the fetch uses the trusted one.

Two GroveDB behaviours shape this code and are worth knowing before you touch it:

- **A path query over a tree that does not exist is an error, not an empty result.** The fetches call `grove_has_raw` on the group's or contract's tree first and return `None` or an empty result when it is absent.
- **GroveDB cannot build a proof inside an open transaction.** `prove_*` with `Some(&transaction)` fails with `NotSupported`. Tests commit first and prove with `None`.

### The DAPI Queries

Three DAPI queries sit on top of these proofs, one per path query, so a client verifies exactly the elements a node read. Handlers live under `packages/rs-drive-abci/src/query/contract_group_queries/` and are versioned by `contract_group_queries` in the query version table.

- `getContractGroupInfo(contractGroupId)` answers with the group's owner, admins (empty for a single owner), name and description, or with no result when no group has the id. The proved form proves the info item or its absence; `rs-drive-proof-verifier` verifies it into a `ContractGroupInfo`.
- `getContractGroupMembers(contractGroupId, members, limit)` answers with one page of one kind. `members` is a `oneof` that names the kind and carries its cursor: `contracts { start_after: contract id }`, `document_types { start_after: contract id + name }` or `tokens { start_after: contract id + position }`. `limit` defaults to and is capped by `max_returned_elements` (100) on the node and in the verifier alike, because a client rebuilds the page query from the request it sent: an omitted limit has to mean the same page everywhere. The response is the page in the same kind, or a proof, verified into a `ContractGroupMembersPage`.
- `getContractGroupsForContract(contractId)` answers with the contract's memberships as a whole, by document type name and by token position, empty lists when it belongs to no group. The proved form proves that too, verified into a `ContractGroupMembershipsForContract`.

In `rs-sdk`, `ContractGroupInfo`, `ContractGroupMembersPage` and `ContractGroupMembershipsForContract` implement `Fetch` and `FetchUnproved` (`packages/rs-sdk/src/platform/contract_groups.rs`); a `ContractGroupMembersPageQuery` carries the group, kind, cursor and limit, and its `after(&page)` is the query for the next page. The wasm-sdk exposes `getContractGroupInfo`, `getContractGroupMembers` (a page object whose `nextStartAfter` is the next cursor) and `getContractGroupsForContract`, each with a `WithProofInfo` twin, and js-evo-sdk wraps them as `sdk.contractGroups`.

## Versioning Touchpoints

Contract groups arrived with protocol version 14, which was unreleased while they were built, so every table below was edited in place rather than copied into a new version (the rule in [Coding Conventions](../contributing/coding-conventions.md)).

| Table | What changed |
|-------|--------------|
| `STATE_TRANSITION_SERIALIZATION_VERSIONS_V3` | `contract_create_state_transition` max version 0 → 1, default 0 → 1 |
| `DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4` | `data_contract_create_transition` converter 0 → 1 (emits the group operations) |
| `DRIVE_VERSION_V9` | `create_initial_state_structure` 3 → 4 (creates the root tree) |
| `DRIVE_ABCI_VALIDATION_VERSIONS_V10` | the contract create `basic_structure` v2 and `state` v1 modules gained the group checks |
| `DriveMethodVersions` | new `contract_group: DriveContractGroupMethodVersions` slot (insert, fetch, prove, cost estimation), `DRIVE_CONTRACT_GROUP_METHOD_VERSIONS_V1`, every method at 0 |
| `DriveVerifyMethodVersions` | new `contract_group: DriveVerifyContractGroupMethodVersions` slot |
| `SystemLimits` | four new limits, plus the `max_group_member_count` rename |

The four limits:

| Limit | Value |
|-------|-------|
| `max_contract_group_memberships_per_contract` | 16 |
| `max_contract_group_admins` | 16 |
| `max_contract_group_name_length` | 64 characters |
| `max_contract_group_description_length` | 256 characters |

Numbers go in `SystemLimits` and nowhere else, so validation reads them from `platform_version.system_limits` rather than from constants in the validation module.

## Fees

Nothing about contract groups has a special fee. Storage is charged at the standard rates for what is written: the info item and three empty subtrees for a registration, one empty item plus one reference per membership, plus whichever intermediate trees a new contract needs on the backwards side. State validation bills each group lookup as a precalculated operation. A transition that fails basic structure costs nothing; one that fails state validation pays for the lookups it caused and has its identity nonce bumped.

## Keys Bound to a Group

An AUTHENTICATION key can carry `ContractBounds::ContractGroup { id }` instead of a contract bound (protocol version 14; see `docs/protocol/contract-bound-authentication-keys.md` for the full rules). The key may then sign only batches, and a batch member on contract `C` is inside the bounds when `C`, the member's document type, or the member's token is a member of the group. The batch transformer (v2) first resolves the signer's bounds from the identity the signature was validated against (handed over through `StateTransitionSignerAwareActionTransformer`, since the unversioned transformer trait is not to be changed). Only when the signing key is an AUTHENTICATION key bound to a contract group does it read the group memberships of every contract the batch touches into the action with `fetch_contract_group_memberships_for_contract_with_fee`, keeping the fee of the read next to them the way a contract's fetch info does. Batch advanced-structure validation v1 then judges the key's bounds from the action alone: it checks `contains` for the whole-contract membership and for the exact one, and bills that fee. A batch signed by any other key is transformed exactly as before: no extra read, no extra fee, nothing new that can fail. A miss is the same paid `ContractBoundedKeyOutOfBoundsError` a contract bound produces. Registration goes through contract-bounds validation v2, which requires AUTHENTICATION, a non-MASTER level and an existing group (one billed `fetch_contract_group_info_with_fee`); encryption and decryption keys cannot be group-bound, since their bounds are opt-in per contract and a group has nothing to opt in with. Below protocol version 14 a transition carrying a group-bound key is not active (`active_version_range`), so it is rejected at decode without a charge, the way a binary that cannot decode the variant rejects it.

Drive keeps group-bound keys in the identity's contract-info level under the group id, next to contract ids, through the `ContractGroupBased` apply info and the `ContractGroupBoundKey` key request. Nothing new is written to the `ContractGroups` tree. Two consequences worth knowing: memberships are append-only, so a group-bound key's reach grows with the group; and `IdentityCreateFromShieldedPool` refuses group-bound keys (shielded-proof validation v1), because its sighash preimage layout predates them.

## What Is Not There Yet

The first protocol version 14 change ships the consensus core only. Known gaps, all deliberate:

- **A group owned by a contract's change-control group.** A third `ContractGroupOwner` variant, where the owner is a `Group` inside a contract rather than a set of identities, was specified and left out. The simple reading, "the signer is a member of that group with power above zero", is the recommended first step; full required-power group actions can follow.
- **Updating or leaving.** Memberships are creation-only. A later `DataContractUpdateTransition` version would be needed to add members from an existing contract or to remove any.
- **Relaxing redundancy.** A document type membership is refused when the whole contract already joins the same group. That could be allowed if a consumer wants the explicit entry.
- **An owners index.** There is no `identity → groups it owns` tree. Finding the groups an identity owns means scanning `Groups`.
- **Creation surfaces.** The DAPI queries and the Rust, wasm and JavaScript read bindings exist (see [The DAPI Queries](#the-dapi-queries)), but registering a group or joining one still needs a hand-built `DataContractCreateTransitionV1`: the wasm-dpp JSON fields and the JavaScript, Kotlin and Swift contract-create options follow separately.

## Tests

Coverage sits at the three layers the feature touches (see [Unit Tests](../testing/unit-tests.md)):

- **dpp** (`contract_group/mod.rs`): the id derivation differs from the contract id and depends on both inputs; both owner kinds resolve membership; the stored info round-trips through bincode with the untrusted decoder.
- **drive** (`drive/contract_groups/tests.rs`): the root tree exists in the initial structure at protocol version 14 and not before; a group can be registered and proved present or absent; memberships land on both sides and prove; members page through with a cursor, a limit of one still reaches every entry past a contract with nothing left, and a zero or over-the-maximum page size is refused on both the fetch and the proof side; estimation and apply build the same operations; registering an existing group is refused.
- **drive-abci queries** (`query/contract_group_queries/*/v0/mod.rs`): each handler rejects a malformed id, cursor or limit; an absent group or a contract in no group answers empty, unproved and proved; members page through with a cursor and come back in the requested kind; every proof verifies through the `Drive::verify_*` twin.
- **drive-proof-verifier** (`proof/contract_groups.rs`, `unproved.rs`): request parsing fails before verification on a missing version, a short id, a missing kind or an out-of-bounds limit; unproved responses decode both owner kinds, reject a page of another kind, and read memberships into sets.
- **drive-abci** (`data_contract_create/contract_group_tests.rs`): end-to-end through `process_raw_state_transitions`, covering register-and-join in one transition, an owner adding a later contract by document type and token, joining a group the identity does not own as a paid failure, joining an unknown group, a group with admins accepting the owner and each admin and refusing outsiders, a registration naming an unknown admin rejected as a paid failure, a version 0 create still processed at the latest version, and the full set of malformed registrations and memberships rejected in basic structure. The protocol upgrade hook has its own test proving the trees exist and accept a registration after the 13 to 14 transition.

```bash
cargo test -p dpp --all-features -- contract_group
cargo test -p drive --lib -- contract_groups
cargo test -p drive-abci -- contract_group data_contract_create
cargo test -p drive-abci --lib -- query::contract_group_queries
cargo test -p drive-proof-verifier --lib -- contract_groups
```

## Rules and Guidelines

**Do:**
- Read contract group data through `DataContractCreateTransitionAccessorsV1` and treat "no registration, no memberships" as the normal case. Never match on `V0` versus `V1` to find out.
- Derive the group id with `generate_contract_group_id` or `contract_group_id()`. Never hash by hand and never accept an id from the wire.
- Keep lookups in state validation and everything else in basic structure. A check that needs Drive must be paid for.
- Use the path helpers in `paths.rs`. The subtree keys are single bytes and easy to transpose.
- Read members in pages with a limit. Never add a query that enumerates a whole group; the info item is the only thing read without a bound.

**Do not:**
- Confuse contract groups with a contract's change-control `groups`. Different types, different limits, different trees.
- Write to the `Members` side without the matching forward entry, or the other way round. Every membership is two writes and the decoders assume both exist.
- Turn the backwards references into bidirectional references. The invariant that makes plain references safe (append-only, no deletion) is what the design relies on; if that invariant ever changes, the storage layout changes with it.
- Add a membership path outside a create transition. The model is creation-only until an update transition version says otherwise.
- Prove inside an open transaction, or path-query a tree you have not confirmed exists.
