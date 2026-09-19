# Contract Moderation

An application that stores user content needs a way to keep an abusive identity out. Before protocol version 14 nothing in consensus state could do that: the contract owner could delete nothing the user wrote, could stop nothing the user would write next, and a client-side blocklist bound nobody but the client that kept it. Every other user's node still accepted the identity's documents.

**Contract moderation** is the answer. A data contract may declare, in its config, that it keeps a **banlist** and/or a **suspension list** of identities, and who may edit them. An identity on the banlist, or on the suspension list with a suspension that has not lapsed, cannot act on the contract at the document level: every document transition it signs against the contract is refused, paid, in the mempool and in a block. Token transitions are not affected. The lists live under the contract's own subtree in Drive, are edited by one new state transition, and are readable with one GroveDB proof.

## The Model

Three facts define a contract's moderation:

1. **The contract declares it.** `DataContractConfigV2::moderation` is an optional `ContractModerationConfig { banlist, suspensions, moderators }`. At least one list must be kept. Which lists a contract keeps is decided when it is created and never changes: an update can not make an unmoderated contract moderated, turn a second list on, or turn a list off (`validate_config_update` 2, `DataContractConfigUpdateError`). Whoever writes documents under a contract knows from its first version whether and how they can be barred from it, which matters most where documents are assets: a ban also stops transfers and sales. And a list that is on may hold entries. Only the moderators may be changed by an update.
2. **The owner moderates, alone or with a fixed set.** `ContractModerators` is `ContractOwner` or `OwnerAndIdentities(set)`: at most `SystemLimits::max_contract_moderators` (16) identities. The owner may always moderate and need not be named; it may be named, and then counts toward the 16. Naming it changes nothing about authority, only about the **moderation team**: `ContractModerationConfig::team(owner_id)` is the set as written (the owner on it only when named), or the owner alone for `ContractOwner`. The team is who shares what moderation earns, not who may moderate. Every identity named must exist: the contract create, and the contract update for the identities it adds, look each one up in state and refuse, paid, with `ContractModeratorIdentityNotFoundError` (41110), because crediting a team member that was never created would be an internal error inside a block. Moderators act alone: there is no threshold and no vote. Neither the owner nor a moderator can be banned or suspended. An entry one of them already carries can still be lifted: a contract update may name as moderator an identity that is banned or suspended, the entry keeps binding it, and the owner or another moderator unbans or unsuspends it without demoting it first (never the identity itself: a moderation cannot target its own signer).
3. **A ban lasts until an unban; a suspension lasts until a block time.** A suspension names the block time, in milliseconds, at which it lapses. A lapsed suspension is not deleted by the clock: the first document transition of the identity that runs at or after that time executes normally and, in the same execution, sweeps the stale entry. An explicit unsuspend deletes it too. A ban supersedes a suspension: banning a suspended identity removes the suspension, and suspending a banned identity is refused.

## The Types

The declaration and the status live in `packages/rs-dpp/src/data_contract/config/moderation/mod.rs`:

```rust
pub struct ContractModerationConfig {
    pub banlist: bool,
    pub suspensions: bool,
    pub moderators: ContractModerators,
}

pub enum ContractModerators {
    ContractOwner,
    OwnerAndIdentities(BTreeSet<Identifier>),
}

pub enum ContractModerationList { Banlist, Suspensions }

pub struct ContractModerationStatus {
    pub banned: bool,
    pub suspended_until: Option<TimestampMillis>,
}
```

The config is `DataContractConfig::V2`, a new variant of the config's own bincode enum inside the contract. A V2 config whose `moderation` is `None` is lowered to V1 before storage (`config_valid_for_platform_version`), so an unmoderated contract keeps the bytes it had before. A V2 config that declares moderation is never lowered: lowering would silently drop the declaration. A contract create or update carrying a V2 config is active from protocol version 14 only (`StateTransition::active_version_range`): before that a node rejects it at decoding, unpaid, exactly as a binary that cannot decode the V2 discriminant does, so upgraded and older nodes agree on every block before activation. The JSON shape of the moderators is a flat `{"$type": "contractOwner"}` or `{"$type": "ownerAndIdentities", "identities": [...]}`, the style of `AuthorizedActionTakers`.

## The Transition

`ContractUserModerationTransition` (type 24) carries one action:

```rust
pub struct ContractUserModerationTransitionV0 {
    pub owner_id: Identifier,          // the moderator that signs
    pub data_contract_id: Identifier,
    pub identity_contract_nonce: IdentityNonce,
    pub action: ContractUserModerationAction,
    pub user_fee_increase: UserFeeIncrease,
    pub signature_public_key_id: KeyID,
    pub signature: BinaryData,
}

pub enum ContractUserModerationAction {
    Ban { identity_id },
    Unban { identity_id },
    Suspend { identity_id, until: TimestampMillis },
    Unsuspend { identity_id },
}
```

It is signed like a contract update: a CRITICAL authentication key without contract bounds, under the signer's contract-scoped nonce, and its minimum fee is the contract update floor. It activates with `CONTRACT_USER_MODERATION_INITIAL_PROTOCOL_VERSION` (14).

### Validation

| Tier | What | Codes |
|---|---|---|
| Basic structure (unpaid) | the target is not the signer; a suspension ends at or before `SystemLimits::max_contract_suspension_until` (2^53 - 1 ms, the largest value JSON clients read exactly) | 10463, 10700 |
| Signature and nonce | CRITICAL key, contract nonce | existing |
| Transform (state, paid) | the contract exists; it keeps the list the action edits; the signer is the owner or a moderator; the target of a ban or a suspend is neither; the target exists; the action fits the target's status | 41100-41106, 41109 |

The transform reads the contract and the target's status and refuses, paid, by bumping the signer's contract nonce. The action carries the status as read, so Drive edits the lists without reading them again, and the mempool, which transforms without a state validation stage, refuses with the same codes as a block. A suspend must end after the block time (41106). Suspending an identity that carries a suspension replaces it, longer or shorter.

### The Document Gate

The gate sits in the batch transformer (a barred signer has every one of its transitions against the contract refused on its own, each with its nonce bump), `transform_document_transitions_within_contract_v0`, right after the contract is fetched, as its own versioned helper: `contract_moderation_gate`, selected by `batch_state_transition.contract_moderation_gate` (`None` up to protocol version 13, `Some(0)` from 14). That transformer is shared with every earlier protocol version, so the gate is a version-table fact there, not something inferred from contract data. A config that declares no moderation costs nothing: no read, no branch. Otherwise the transformer reads the owner's status on the lists the contract keeps, bills the read, and:

- banned: every document transition of the batch on that contract except a deletion fails with `ContractUserBannedError` (41107), paid, each with its contract nonce bump;
- suspended and not lapsed: the same with `ContractUserSuspendedError` (41108);
- suspended and lapsed: the transitions go through and the batch action records the `(contract, identity)` pair in `lapsed_suspensions`; the batch converter (`documents_batch_transition` generation 1) appends one delete per pair.

Deletions (`Delete` and `IndexOnlyDelete`) are never refused: a barred identity can write nothing new, move nothing and sell nothing, but it may still take down what it wrote, under the document type's ordinary deletion rules. A batch of deletions alone carries on whole; in a mixed batch the deletions carry on next to the refusals. Because the transformer runs in `check_tx`, a barred identity's documents never enter the mempool. Only the actor is checked: the recipient of a transfer and the seller of a purchase are not. No contract could declare moderation before protocol version 14, so older blocks replay unchanged through the same code.

### The Errors

Basic, in the data contract sub-band: `InvalidContractModerationConfigError` (10462), `ContractModerationSelfTargetError` (10463). State, in their own sub-band: `ContractModerationNotEnabledError` (41100), `IdentityNotContractModeratorError` (41101), `ContractModerationTargetNotAllowedError` (41102), `ContractUserAlreadyBannedError` (41103), `ContractUserNotBannedError` (41104), `ContractUserNotSuspendedError` (41105), `ContractSuspensionNotInFutureError` (41106), `ContractUserBannedError` (41107), `ContractUserSuspendedError` (41108), `ContractModerationTargetNotFoundError` (41109), `ContractModeratorIdentityNotFoundError` (41110, from the contract create and update, not from the moderation transition). A contract update that turns a list on or off is refused with the existing `DataContractConfigUpdateError` (40002).

## Storage

```text
[64] DataContractDocuments
└── <contract id>
    ├── [0] the contract (or its history subtree)
    ├── [1] documents
    ├── [2] contract version item
    ├── [3] banlist       -> <identity id> -> Item([])                    (when declared)
    └── [4] suspensions   -> <identity id> -> Item(until, u64 BE millis)   (when declared)
```

The trees are created by `insert_contract` generation 2 for a contract that declares them, and by nothing else: the lists are fixed at creation, so a contract update creates none and leaves the existing ones and their entries alone, and no tree is made lazily by the first ban. An entry's storage flags name the moderator that wrote it, so the storage refund of its deletion goes to that moderator whichever transition deletes it: the explicit unban or unsuspend, the ban over a suspension, or the document transition that sweeps a lapsed suspension. The sweep's processing fee is charged to the batch signer.

The writers, readers and provers live in `packages/rs-drive/src/drive/contract/moderation/`, versioned by `DriveContractModerationMethodVersions`. A suspend that replaces an entry is a `batch_replace`, because two operations on one key would fail the batch.

## Reading and Proving

`fetch_contract_moderation_status(contract, identity, lists)` reads the identity's entry on each list named, and the verifier of its proof rebuilds the same merged path query from the same lists. The lists are the ones the contract's config declares; an undeclared list has no tree and cannot be queried, so a status query names the lists it wants and the node refuses one the contract does not keep. `fetch_contract_moderation_entries` pages one list in identity id order, bounded by the platform version's `max_returned_elements` (the default page size too, and the number the proof verifier assumes when a request names no limit), with the last identity as the cursor. A page shorter than its limit is the last one and carries no cursor.

### The DAPI Queries

- `getContractModerationStatus(contract_id, identity_id, lists, prove)`: the identity's status on the lists named.
- `getContractModerationEntries(contract_id, list, start_after, limit, prove)`: one page of a list.

A status query answers for the lists it names and no others: the SDK result, `ContractModerationListStatuses`, holds one `ContractModerationListStatus` per list queried, so a list that was not read is absent rather than reported as empty (`banned()` is `None` unless the banlist was queried). `ContractModerationStatusQuery::for_contract` names every list the contract keeps; the wasm-sdk does the same, fetching the contract, when the query names no list. Both have `Fetch` and `FetchUnproved` impls in the Rust SDK (`platform::contract_moderation`), wasm-sdk functions and `contracts.moderationStatus` / `contracts.moderationEntries` on the JavaScript SDK. The proof of a moderation transition's execution is the edited entry, present or absent, and is classified as affected state: an earlier or later moderation leaving the same entry verifies just the same. Its result, `VerifiedContractModerationListStatus`, is a `ContractModerationListStatus` of that one list (`Banlist { banned }` or `Suspensions { suspended_until }`), not a full status: the other list was not proved, so it is left unknown rather than reported as empty. An identity whose unsuspend was just proved may be banned; the status query answers that.

## Versioning Touchpoints

All in place for protocol version 14: `CONTRACT_VERSIONS_V6` admits config V2 (`max_version: 2`, default stays 1) and `validate_config_update` 2; `STATE_TRANSITION_SERIALIZATION_VERSIONS_V3` and `DRIVE_ABCI_VALIDATION_VERSIONS_V10` carry the transition's slots and `batch_state_transition.contract_moderation_gate`, and the contract update's basic structure moves to 2 to validate the declaration; `DRIVE_CONTRACT_METHOD_VERSIONS_V4` bumps `insert_contract` to 2 and adds the `moderation` table (its `update_contract` 2 belongs to token distribution and does nothing for moderation); `DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4` adds the converter slot and bumps `documents_batch_transition` to 1 for the sweep; `DRIVE_VERIFY_METHOD_VERSIONS` and `DRIVE_ABCI_QUERY_VERSIONS` gain their moderation tables; `SYSTEM_LIMITS_V4` gains `max_contract_moderators` and `max_contract_suspension_until`.

## What Is Not There Yet

Group-based moderators (`AuthorizedActionTakers::Group` through group actions), keys bound to the contract allowed to sign its moderation, checks on the counterparty of a transfer or a purchase, entry metadata such as a reason or a timestamp, and the Swift and Kotlin SDKs.

## Tests

- `packages/rs-dpp/src/data_contract/config/moderation/mod.rs` and `config/methods/validate_update/v2`: the declaration's rules and the update rules.
- `packages/rs-drive/src/drive/contract/moderation/tests.rs`: tree creation on insert, the trees and their entries surviving a contract update, every writer with estimation, status and page proofs, paging, and the refund going to the first moderator after another one replaces its suspension.
- `packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/contract_user_moderation/tests.rs`: the whole pipeline, including the mempool refusal, the lapse sweep, the moderator set, every refusal code, the lists staying as the contract was created with them, a barred identity deleting its own documents in a block and in the mempool, lifting the entry of an identity an update made moderator, the per-list execution proof, a named owner, a create or an update naming a moderator that does not exist, an update keeping its moderators, and inactivity of the transition and of a moderated contract create or update before protocol version 14.
