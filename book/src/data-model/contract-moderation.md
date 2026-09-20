# Contract Moderation

An application that stores user content needs a way to keep an abusive identity out. Before protocol version 14 nothing in consensus state could do that: the contract owner could delete nothing the user wrote, could stop nothing the user would write next, and a client-side blocklist bound nobody but the client that kept it. Every other user's node still accepted the identity's documents.

**Contract moderation** is the answer. A data contract may declare, in its config, that it keeps a **banlist** and/or a **suspension list** of identities, and who may edit them. An identity on the banlist, or on the suspension list with a suspension that has not lapsed, cannot act on the contract at the document level: every document transition it signs against the contract is refused, paid, in the mempool and in a block. Token transitions are not affected. The lists live under the contract's own subtree in Drive, are edited by one new state transition, and are readable with one GroveDB proof.

## The Model

Three facts define a contract's moderation:

1. **The contract declares it.** `DataContractConfigV2::moderation` is an optional `ContractModerationConfig { banlist, suspensions, moderators }`. At least one list must be kept, unless a document type lets the moderators delete its documents (see Deleting Documents below), in which case the declaration may keep none. Which lists a contract keeps is decided when it is created and never changes: an update can not make an unmoderated contract moderated, turn a second list on, or turn a list off (`validate_config_update` 2, `DataContractConfigUpdateError`). Whoever writes documents under a contract knows from its first version whether and how they can be barred from it, which matters most where documents are assets: a ban also stops transfers and sales. And a list that is on may hold entries. Only the moderators may be changed by an update.
2. **The owner moderates, alone or with a fixed set.** `ContractModerators` is `ContractOwner` or `AppointedModerators(set)`: at most `SystemLimits::max_contract_moderators` (16) identities. The owner may always moderate and need not be named; it may be named, and then counts toward the 16. Naming it changes nothing about authority. Every identity named must exist: the contract create, and the contract update for the identities it adds, look each one up in state and refuse, paid, with `ContractModeratorIdentityNotFoundError` (41110). A moderator that does not exist can never sign, so naming one is a mistake, and catching it once at the declaration is cheaper than guarding every later reader of the set. Moderators act alone: there is no threshold and no vote. Neither the owner nor a moderator can be banned or suspended. An entry one of them already carries can still be lifted: a contract update may name as moderator an identity that is banned or suspended, the entry keeps binding it, and the owner or another moderator unbans or unsuspends it without demoting it first (never the identity itself: a moderation cannot target its own signer).
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
    AppointedModerators(BTreeSet<Identifier>),
}

pub enum ContractModerationList { Banlist, Suspensions }

pub struct ContractModerationReason {
    pub code: Option<u16>,
    pub text: String,
}

pub struct ContractBan { pub reason: ContractModerationReason }
pub struct ContractSuspension { pub until: TimestampMillis, pub reason: ContractModerationReason }

pub struct ContractModerationStatus {
    pub ban: Option<ContractBan>,
    pub suspension: Option<ContractSuspension>,
}
```

Every ban and every suspension carries a **reason**, stored with the entry so that whoever reads the list reads why. The `text` is free: at most `SystemLimits::max_contract_moderation_reason_length` (1024) bytes of UTF-8, possibly empty. The `code` is reserved for the ban codes a contract may declare in a later protocol version. No contract declares any today, so it is expected to be `None`; a moderator may still write any `u16` there, and nothing checks it against anything. The moderator pays the storage of the reason, byte for byte, and gets it back when the entry is removed.

The config is `DataContractConfig::V2`, a new variant of the config's own bincode enum inside the contract. The config version follows the platform version, as V1 did from protocol version 9: from protocol version 14 every new contract carries a V2 config, moderated or not (`CONTRACT_VERSIONS_V6` sets both `max_version` and `default_current_version` to 2), and an existing V1 contract moves to V2 with its next update. `config_valid_for_platform_version` lowers a V2 only where the platform version does not admit it, never because of what it declares. Lowering would drop a moderation declaration, and moderation can never be turned on later, so that case is refused rather than dropped: serializing a contract whose config declares moderation at a platform version below 14 (`ensure_admitted_by_platform_version`), and parsing a config value with a `moderation` key at such a version, both fail with `ProtocolError::NotSupported`. For the same reason the declaration refuses an unknown key instead of skipping it (`deny_unknown_fields`, and the moderators' `$type` map likewise): a misspelled `suspensions` would otherwise leave the contract without the list for good. A contract create or update carrying a V2 config is active from protocol version 14 only (`StateTransition::active_version_range`): before that a node rejects it at decoding, unpaid, exactly as a binary that cannot decode the V2 discriminant does, so upgraded and older nodes agree on every block before activation. The JSON shape of the moderators is a flat `{"$type": "contractOwner"}` or `{"$type": "appointedModerators", "identities": [...]}`, the style of `AuthorizedActionTakers`.

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
    Ban { identity_id, reason: ContractModerationReason },
    Unban { identity_id },
    Suspend { identity_id, until: TimestampMillis, reason: ContractModerationReason },
    Unsuspend { identity_id },
}
```

It is signed like a contract update: a CRITICAL authentication key without contract bounds, under the signer's contract-scoped nonce, and its minimum fee is the contract update floor. It activates with `CONTRACT_USER_MODERATION_INITIAL_PROTOCOL_VERSION` (14).

### Validation

| Tier | What | Codes |
|---|---|---|
| Basic structure (unpaid) | the target is not the signer; a suspension ends at or before `SystemLimits::max_contract_suspension_until` (2^53 - 1 ms, the largest value JSON clients read exactly); the text of a ban's or a suspension's reason is at most `SystemLimits::max_contract_moderation_reason_length` bytes (its code is not checked) | 10901, 10700, 10903 |
| Signature and nonce | CRITICAL key, contract nonce | existing |
| Transform (state, paid) | the contract exists; it keeps the list the action edits; the signer is the owner or a moderator; the target of a ban or a suspend is neither; the target exists; the action fits the target's status | 41100-41106, 41109 |

The transform reads the contract and the target's status and refuses, paid, by bumping the signer's contract nonce. The action carries the status as read, so Drive edits the lists without reading them again, and the mempool, which transforms without a state validation stage, refuses with the same codes as a block. A suspend must end after the block time (41106). Suspending an identity that carries a suspension replaces it, longer or shorter.

### The Document Gate

The gate sits in the batch transformer (a barred signer has every one of its transitions against the contract refused on its own, each with its nonce bump), `transform_document_transitions_within_contract_v0`, right after the contract is fetched, as its own versioned helper: `contract_moderation_gate`, selected by `batch_state_transition.contract_moderation_gate` (`None` up to protocol version 13, `Some(0)` from 14). That transformer is shared with every earlier protocol version, so the gate is a version-table fact there, not something inferred from contract data. A config that declares no moderation costs nothing: no read, no branch. Otherwise the transformer reads the owner's status on the lists the contract keeps, bills the read, and:

- banned: every document transition of the batch on that contract except a deletion fails with `ContractUserBannedError` (41107), paid, each with its contract nonce bump;
- suspended and not lapsed: the same with `ContractUserSuspendedError` (41108);
- suspended and lapsed: the transitions go through and the batch action records the contract in `lapsed_suspensions` (the identity is always the batch owner, so it is not stored and nothing can queue another identity's); the batch converter (`documents_batch_transition` generation 1) appends one delete per contract.

Deletions (`Delete` and `IndexOnlyDelete`) are never refused: a barred identity can write nothing new, move nothing and sell nothing, but it may still take down what it wrote, under the document type's ordinary deletion rules. A batch of deletions alone carries on whole; in a mixed batch the deletions carry on next to the refusals. Because the transformer runs in `check_tx`, a barred identity's documents never enter the mempool. The other party of a document transition is checked too: a transfer to a banned or live-suspended recipient, and a purchase from a banned or live-suspended seller, are refused with `ContractModerationCounterpartyBarredError` (41114), paid by the signer, so a barred identity collects neither assets nor proceeds on the contract. That read is billed to the batch; a counterparty's lapsed suspension is left for the counterparty's own next transition to sweep. No contract could declare moderation before protocol version 14, so older blocks replay unchanged through the same code.

### The Errors

Basic, in their own band (10900-10949): `InvalidContractModerationConfigError` (10900), `ContractModerationSelfTargetError` (10901), `ContractModerationReasonTooLongError` (10903; 10902 is reserved). State, in their own sub-band: `ContractModerationNotEnabledError` (41100), `IdentityNotContractModeratorError` (41101), `ContractModerationTargetNotAllowedError` (41102), `ContractUserAlreadyBannedError` (41103), `ContractUserNotBannedError` (41104), `ContractUserNotSuspendedError` (41105), `ContractSuspensionNotInFutureError` (41106), `ContractUserBannedError` (41107), `ContractUserSuspendedError` (41108), `ContractModerationTargetNotFoundError` (41109), `ContractModeratorIdentityNotFoundError` (41110, from the contract create and update, not from the moderation transition), `ContractModerationCounterpartyBarredError` (41114, from the document gate; 41111 to 41113 are reserved). A contract update that turns a list on or off is refused with the existing `DataContractConfigUpdateError` (40002).

## Deleting Documents

A banlist keeps an identity out; it does not take down what the identity already wrote. A document type may let the contract's moderators do that:

```json
"post": {
  "type": "object",
  "canBeDeletedByModerators": true,
  "properties": { "text": { "type": "string", "maxLength": 280, "position": 0 } },
  "additionalProperties": false
}
```

`canBeDeletedByModerators` is a document type keyword of meta-schema v3 (`DocumentTypeV2::documents_can_be_deleted_by_moderators`), parsed by `apply_can_be_deleted_by_moderators`. Its rules:

- **The contract declares moderation.** The keyword on a contract without a `moderation` block is refused (`InvalidContractStructure`, 10231): moderation can not be switched on later, so nobody could ever delete anything. In return a `moderation` block may keep no list at all when at least one document type carries the keyword (`ContractModerationConfig::validate` takes that fact from the contract): a contract can moderate content without moderating users.
- **It is fixed with the type.** A contract update can not add the keyword to an existing document type or take it away (`DocumentTypeUpdateError`, 40212): authors keep the rules they wrote under. A document type an update adds may carry it.
- **It is independent of `canBeDeleted`**, which rules what a document's own owner may do. `canBeDeleted: false` with `canBeDeletedByModerators: true` is a post its author can not retract and moderation can remove.
- **Some types can not carry it**: one that keeps history (Drive refuses to delete such documents), an indexOnly one (there is no stored row to name by id), and one that restricts creation (its documents are the contract owner's, which no moderator may delete). Transferable and tradeable types may, and so may a type with a deletion token cost, which a moderator does not pay.
- **It may come with a window.** `canBeDeletedByModeratorsFor: 86400` lets the moderators delete a document for that many seconds after its last modification (`$updatedAt`), and no longer: once block time is past `$updatedAt` plus the window the document is settled, and no moderator deletes it any more, the contract owner included (`DocumentModerationWindowElapsedError`, 41116). At exactly `$updatedAt` plus the window the deletion still passes; the document's own owner still deletes it as `canBeDeleted` allows. Moderation acts on what was just written; it does not reach back into what has stood unchallenged. A replace or a price update moves `$updatedAt`, so new content opens the window again; a transfer or a purchase does not. The window needs the flag, at least one second, and `$updatedAt` in the type's `required`, so that every document carries the clock; like the flag it is fixed with the type, in both directions (a longer one would reopen documents that had settled). It is in seconds, as the other durations of a document type are, and it says nothing about a document's own owner, whose deletion `canBeDeleted` rules at any age.
- **For references it counts as deletable.** A `permanentDocument` reference refuses such a type (`ReferencedDocumentTypeDeletableError`, 40122) whatever its `canBeDeleted` says, so the guarantee that a validated permanent reference never dangles holds; a `deletableDocument` reference accepts it, `canBeDeleted: false` included. Both checks, at contract registration and at document write, read "deletable" as deletable by anyone. A like or a reply that points at a post moderators can remove therefore declares `refersTo: deletableDocument`: the join reports a removed post as a missing id, and the removal record says why it is missing.

The deletion is a fifth action of the same transition:

```rust
ContractUserModerationAction::DeleteDocument {
    document_type_name: String,
    document_id: Identifier,
    reason: ContractModerationReason,   // as on a ban: a code nothing checks, a text that may be empty
}
```

It names no identity (`identity_id()` is `None`): whose document it is is only known once the document is read. The transform checks, in order and each refusal paid: the document type exists (10406), it carries the keyword (`DocumentTypeNotDeletableByModeratorsError`, 41115), the signer is the owner or a moderator (41101), the document exists (`DocumentNotFoundError`), its owner is neither the contract owner nor a moderator (41102, the rule that protects them from a ban protects what they wrote), and block time is within the type's window after the document's `$updatedAt`, when the type sets one (41116). The document is read the way a document's own deletion reads it, billed the same. The action carries the contract, the document's owner and the block time, so Drive reads nothing again. Nothing the document type prices is charged: neither its deletion token cost nor its `actionFees` deletion fee, both of which are what a document's own owner pays for deleting it.

Drive then runs `DocumentOperationType::DeleteDocumentByModerator`, the ordinary deletion (so every index and aggregate of the type stays right) without its `canBeDeleted` guard, which is the owner's rule and not the moderators', and writes a **removal record**:

```rust
pub struct ContractDocumentRemoval {
    pub document_owner_id: Identifier,
    pub moderator_id: Identifier,
    pub reason: ContractModerationReason,
    pub removed_at: TimestampMillis,   // the block time
}
```

The record is what is left to say that a document was removed, not lost: a client holding a dangling id (a reply whose parent is gone) can prove who removed it, whose it was, when and why. The moderator pays for it, reason included, and nothing ever deletes it. A record is final: from protocol version 14 a document id commits to the nonce of its create transition and is produced at most once, so the removed id can not be created again, no second removal can replace the record, and the record and a live document of that id never coexist.

**The deleted document's owner gets no storage refund.** The batch carries `ContractModerationOperationType::ForfeitStorageRefunds`, a marker that writes nothing, and `apply_drive_operations` generation 1 turns every removal such a batch attributes to an identity into a removal attributed to nobody: the bytes still leave the system (`FeeResult::removed_bytes_from_system`), no refund is computed, and the credits stay in the storage pools they were distributed to when the document was written. An estimate carries no refund to begin with (refunds come from the flags of what is really removed), so the mempool's fee check is the same with or without the forfeiture. Forfeiting the whole batch is exact: a moderator's deletion removes the document and nothing else, since its record is written once and the nonce it bumps keeps its size. An author who deletes the same document with an ordinary document transition is refunded as always.

## Storage

```text
[64] DataContractDocuments
└── <contract id>
    ├── [0] the contract (or its history subtree)
    ├── [1] documents
    └── [2] other
        ├── [16]  document removals -> <document type name> -> <document id>
        │                            -> Item(owner id ‖ moderator id ‖ removed at ‖ reason)   (with such a document type)
        ├── [64]  contract version item (every contract)
        ├── [128] banlist       -> <identity id> -> Item(reason)                 (when declared)
        └── [192] suspensions   -> <identity id> -> Item(until ‖ reason)         (when declared)
```

`until` is a u64 of block time in milliseconds, big-endian. A reason is a tag byte (`0`: no code, `1`: a code), the code as a big-endian u16 when tagged, then the text as UTF-8 up to the end of the value, so an entry with an empty reason and no code costs one byte more than the bare entry would. A value without the tag byte is an entry written before entries carried a reason and reads as the empty reason. A document removal is the document owner's id, the moderator's id, `removed at` as a u64 of block time in milliseconds, big-endian, then the reason the same way (`types::encode_document_removal`).

The document removals tree exists exactly when the contract has a document type that carries `canBeDeletedByModerators`: a contract without one keeps the other tree, and the shape, it would have had. One subtree per such document type is created with the type, by `insert_contract` generation 2 or by `update_contract` generation 2 for a type an update adds, and the tree above them with the first: whether it is there is read off the stored contract, since an existing type never changes the keyword, so the update needs no read. Nothing is created lazily by the first removal. The key sorts below `128`, as a key added later should: a contract that keeps both lists, the one whose other tree then holds four keys, still has the banlist on top. With fewer keys the version item is on top, and the list one level down.

The contract's own subtree holds three keys whatever the contract keeps, so its Merk keeps `1`, the documents, on top: every document proof and write goes through that key, and a fourth key beside it would have pushed it one level down (a Merk built from one sorted batch roots at the middle key). Everything else a contract keeps goes into `2`, its **other tree**, which protocol version 14 introduces together with the version item. Inside, the keys are spread like the root tree's, so the tree stays balanced as it fills and the most read entry sits on top: the banlist at `128`, read by every document transition on a moderated contract, the version item at `64`, the suspension list at `192`. A key added later should sort below `128` to keep the banlist on top when four keys are created at once.

The other tree is written by every contract insertion, and by the migration on the first block of protocol version 14 for the contracts stored before it. A contract update finds it there, so its fee estimate writes none; applied, the update reads key `2` once, billed, rather than fail inside a block: a tree is left alone (it may hold the lists), a missing one is written. The 4.2 betas kept the version item itself at key `2`, before the other tree existed: an update of a contract that still holds that item puts the tree in its place and the item under it (`add_contract_to_storage` generation 1). Until such a contract is updated, the unproved `getDataContractsLatestVersions` reads its version from that item, and the proved form shows no version item for it.

The list trees are created by `insert_contract` generation 2 for a contract that declares them, and by nothing else: the lists are fixed at creation, so a contract update creates none and leaves the existing ones and their entries alone, and no tree is made lazily by the first ban. An entry's storage flags name the moderator that wrote it, so the storage refund of its deletion goes to that moderator whichever transition deletes it: the explicit unban or unsuspend, the ban over a suspension, or the document transition that sweeps a lapsed suspension. The sweep's processing fee is charged to the batch signer.

The writers, readers and provers live in `packages/rs-drive/src/drive/contract/moderation/`, versioned by `DriveContractModerationMethodVersions`. A suspend that replaces an entry is a `batch_replace`, because two operations on one key would fail the batch. The replacement brings its own reason, so the entry may change size: a longer replacement merges the flags as a document that changes hands does, the moderator that replaced it paying for the bytes it added and becoming the entry's owner, refunded when it is removed; a shorter or an equally long one stays the first moderator's, who is refunded the removed bytes at once and the rest on removal. A fee estimate prices a replacement as a fresh insert of the whole entry, because GroveDB's average-case replace assumes an item keeps its size and would price no storage for a longer reason; the entries a write walks past, and the one a delete removes, are estimated at a typical reason (128 bytes of text), not at the longest.

## Reading and Proving

`fetch_contract_moderation_status(contract, identity, lists)` reads the identity's entry on each list named, and the verifier of its proof rebuilds the same merged path query from the same lists. The lists are the ones the contract's config declares; an undeclared list has no tree and cannot be queried, so a status query names the lists it wants and the node refuses one the contract does not keep. `fetch_contract_moderation_entries` pages one list in identity id order, bounded by the platform version's `max_returned_elements` (the default page size too, and the number the proof verifier assumes when a request names no limit), with the last identity as the cursor. A page shorter than its limit is the last one and carries no cursor.

### The DAPI Queries

- `getContractModerationStatus(contract_id, identity_id, lists, prove)`: the identity's status on the lists named.
- `getContractModerationEntries(contract_id, list, start_after, limit, prove)`: one page of a list.
- `getContractDocumentRemovals(contract_id, document_type_name, document_ids | page, prove)`: the records of the documents moderators deleted, within one document type that carries `canBeDeletedByModerators` (no other keeps records, so the node refuses any other). By document ids, up to `max_returned_elements` of them and none twice: an id with no record is left out of the response, and proved absent by a proof. Or one page in document id order, with the last document id as the cursor. `Drive::verify_contract_document_removals` rebuilds the path query from the same request.

A status query answers for the lists it names and no others: `Drive::verify_contract_moderation_status` and the SDK result both return `ContractModerationListStatuses`, one `ContractModerationListStatus` per list queried, so a list that was not read is absent rather than reported as empty (`banned()` is `None` unless the banlist was queried). `ContractModerationStatusQuery::for_contract` names every list the contract keeps; the wasm-sdk does the same, fetching the contract, when the query names no list. Both have `Fetch` and `FetchUnproved` impls in the Rust SDK (`platform::contract_moderation`), wasm-sdk functions and `contracts.moderationStatus` / `contracts.moderationEntries` on the JavaScript SDK. The proof of a moderation transition's execution covers the lists the moderation touched and is classified as affected state: an earlier or later moderation leaving the same entries verifies just the same. A ban does two things, adds the ban and removes a suspension, so its proof covers every list the contract keeps (the banlist entry present, the suspension absent), which the prover and the verifier both read from the contract's config (so the SDKs fetch and cache the contract before broadcasting a ban, as they do for the contracts a document batch touches); an unban, a suspend and an unsuspend prove the one entry they edit. The result, `VerifiedContractModerationListStatuses`, holds one `ContractModerationListStatus` per list proved, never a full status: a list that was not proved is left unknown rather than reported as empty. An identity whose unsuspend was just proved may be banned; the status query answers that.

## Fee Pots and the Claim

A moderation team can be paid. A document type may charge a fixed fee in credits for an action on its documents (the `actionFees` keyword, see [Document action fees](../fees/overview.md#document-action-fees)), split in two parts. The `owner` parts collect in the contract's **owner pot**, the `moderators` parts in its **moderators pot**.

```text
[40] PreFundedSpecializedBalances (sum tree)
├── [64]  owner fee pots      (sum tree) -> <contract id> -> SumItem(credits)
├── [128] voting balances
└── [192] moderators fee pots (sum tree) -> <contract id> -> SumItem(credits)

[64] DataContractDocuments -> <contract id> -> [2] other
    ├── [32] last claim of the owner pot       Item(epoch u16 BE | time u64 BE | claimant id)   (after a claim)
    └── [96] last claim of the moderators pot  Item(epoch u16 BE | time u64 BE | claimant id)   (after a claim)
```

The pots are not under the contract. The per-block total credits check (`calculate_total_credits_balance`) sums a fixed set of root sum trees, and `DataContractDocuments` is a normal tree: credits parked under a contract would leave that sum and fail every block with `CorruptedCreditsNotBalanced`. `PreFundedSpecializedBalances` is one of the summed trees, so the pots live there, in two sum trees beside the voting balances, created at genesis (state structure 4) and by the upgrade to protocol version 14 through the same helper, one after the other, so that both node populations build the same Merk. A pot is created by the first fee it receives, and so is its tree on a chain that reached protocol version 14 on a build from before the pots: that first fee checks, with a billed read, that the tree is there. The estimation of a voting balance write moves to generation 1 with them, because the prefunded balances layer now holds three trees instead of one. The two last claims are plain items of the contract's other tree, below `128` so the banlist stays on top, written by the first claim and replaced by every later one. A last claim (`ContractFeePotLastClaim`) is 42 bytes: the epoch of the claim, which the next claim is judged against, the time of its block in milliseconds, and the id of the identity that signed it. The owner pot's claimant is always the owner; the moderators pot's is whichever member of the team claimed for all of them, so the team can see who paid them and when. Every last claim has the same size, so a replacement never changes the size of the item, and the item carries no storage flags: it is never removed, and no claim adds bytes for anyone to own.

**The team** that shares the moderators pot is the set of identities the contract appoints, the owner among them only when appointed, and the owner alone when nobody is appointed (`ContractModerators::team`). It is about earnings, not authority: an owner who is not appointed still may moderate. `ContractFeePot::recipients` names who a payout of a pot goes to: the contract owner for the owner pot, the team for the moderators pot, nobody for the moderators pot of a contract that declares no moderation.

`ContractFeeClaim` (state transition type 25) names a contract and a pot and pays the pot out. It is signed with a CRITICAL authentication key under the signer's contract nonce, and the claimant pays its gas like any other transition.

| Stage | Check | Error |
|---|---|---|
| Transform (state, paid) | the contract exists | `DataContractNotPresentError`, unpaid |
| | the signer is a recipient of the pot: the owner for the owner pot, a member of the team for the moderators pot | 41113 |
| | the pot was not paid out in this epoch yet | 41111 |
| | every recipient gets at least a credit | 41112 |

The owner pot goes to the owner whole. The moderators pot is split equally between the team, and what the split leaves over, less than a credit per member, stays in the pot for the next claim, so no member is favoured by the order of the identity ids. Each pot is paid out at most once per epoch and the two are independent: the owner's claim does not use up the team's, nor the reverse. A refused claim is paid for by a nonce bump and leaves the pot and its last claim alone. As for moderation, state validation *is* the transform, so the mempool refuses with the same codes as a block.

The team is read when the claim executes. An owner who changes the appointed set by a contract update and then claims pays the new set: that follows from the owner controlling the contract's config, and is not prevented. The claim credits every recipient's balance, which is why a named moderator must exist (41110): crediting a balance that is not there is an internal error.

The proof of a claim's execution shows the pot with its last claim and the balance of every recipient, which the prover and the verifier both read from the contract. `VerifiedContractFeeClaim` carries the contract id, the pot, that last claim (epoch, block time, claimant), the credits left in the pot and the balances. A pot that was never claimed proves no claim; a later claim of the same pot verifies just the same, so the result is classified as affected state.

### Reading the Pots

- `getContractFeePots(contract_id, prove)`: both pots of the contract, each with its credits and its last claim: the epoch and the block time it was paid out in, and the identity that claimed.

The query always reads both pots, so its proof is one fixed path query (`Drive::contract_fee_pots_query`) that the prover and `Drive::verify_contract_fee_pots` build alike, with nothing in the request to get wrong. A pot nothing was paid into yet has no element and reads as zero credits, and a pot never paid out has no last claim, which is not a claim in epoch 0: a pot can have been paid out in epoch 0, so the last claim is a message of its own on the wire, unset when there is none, and the JavaScript fields (`lastClaimEpoch`, `lastClaimTimeMs`, `lastClaimantId`) are absent together. The proof says nothing about the contract itself, only about what is stored under its id, so the node refuses the query for a contract it does not hold before it proves anything, and a client that needs to know the contract exists fetches it.

A recipient reads the pots to decide whether a claim is worth its gas: the credits are what it would pay, and a last claim epoch equal to the current epoch means the claim would be refused (41111). A member of the team also reads there which member last claimed for the team, and when. The Rust SDK has `Fetch` and `FetchUnproved` impls for `ContractFeePots` (`platform::contract_fee_pots`, queried by the contract id), the wasm-sdk `getContractFeePots` and `contractClaimFees`, and the JavaScript SDK `contracts.feePots` and `contracts.claimFees`.

The claim's proof is verified against the contract, which names who the pot pays, and the team can change by a contract update. So every client fetches the contract again before a claim instead of trusting a cached copy: `ClaimContractFees` in the Rust SDK, `contractClaimFees` in the wasm-sdk, and the wasm-sdk's generic `broadcastAndWait` for a `ContractFeeClaim` built by hand, which falls back to the cached copy when that fetch fails, because the transition is already broadcast by then.

## Versioning Touchpoints

All in place for protocol version 14: `CONTRACT_VERSIONS_V6` makes config V2 the config of every new contract (`max_version` and `default_current_version` 2) and `validate_config_update` 2; `STATE_TRANSITION_SERIALIZATION_VERSIONS_V3` and `DRIVE_ABCI_VALIDATION_VERSIONS_V10` carry the transition's slots and `batch_state_transition.contract_moderation_gate`, and the contract update's basic structure moves to 2 to validate the declaration; `DRIVE_CONTRACT_METHOD_VERSIONS_V4` bumps `insert_contract` to 2 and adds the `moderation` table (its `update_contract` 2 belongs to token distribution and does nothing for moderation); `DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4` adds the converter slot and bumps `documents_batch_transition` to 1 for the sweep; `DRIVE_VERIFY_METHOD_VERSIONS` and `DRIVE_ABCI_QUERY_VERSIONS` gain their moderation tables; `SYSTEM_LIMITS_V4` gains `max_contract_moderators`, `max_contract_suspension_until` and `max_contract_moderation_reason_length`.

The document deletion adds, all for protocol version 14 as well: the `canBeDeletedByModerators` keyword in meta-schema v3 (`CONTRACT_VERSIONS_V6` already selects it); five slots in `DriveContractModerationMethodVersions` and one in the verify and query tables; and `batch_operations.apply_drive_operations = 1` in `DRIVE_VERSION_V9`, the generation that forfeits the refund. The transition's own tables do not move: the action joins a transition no release contains.

## What Is Not There Yet

Deleting indexOnly documents (the action would have to carry the owner and the values), deleting every document of an identity at once, action fees on token transitions, group-based moderators (`AuthorizedActionTakers::Group` through group actions), keys bound to the contract allowed to sign its moderation, ban codes declared by the contract (the reason's `code` is where they will go), further entry metadata such as a timestamp or the moderator's id, and the Swift and Kotlin SDKs. The refusal a barred identity receives (41107, 41108, 41114) does not repeat the reason: the status query does.

## Tests

- `packages/rs-dpp/src/data_contract/document_type/class_methods/try_from_schema/v3/moderators_delete_tests.rs`: the keyword's rules and the window's (it needs the flag and `$updatedAt`, and its shape is refused on the stored path too); `validate_update/common`: the keyword frozen across updates.
- `packages/rs-drive/src/drive/contract/moderation/document_removal_tests.rs`: the trees created with the contract and with a document type an update adds, records written, replaced, read by ids and by page with proofs that verify to the same, the bounds of a read, estimate against applied cost, and a moderator's deletion refunding nobody where the author's own refunds the author.
- `packages/rs-drive-abci/src/query/contract_moderation_queries/contract_document_removals`: the query by ids and by page, its proof read back by the verifier, and every request it refuses.
- `packages/rs-dpp/src/data_contract/config/moderation/mod.rs` and `config/methods/validate_update/v2`: the declaration's rules and the update rules.
- `packages/rs-drive/src/drive/contract/moderation/tests.rs`: tree creation on insert, the trees and their entries surviving a contract update, every writer with estimation, status and page proofs, paging, the refund going to the first moderator after another one replaces its suspension, and a status proof over one list saying nothing about the other.
- `packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/batch/transformer/v0/contract_moderation_gate/mod.rs`: the gate is silent before protocol version 14 and for an unmoderated contract, and refuses each barred operation of one batch on its own while keeping the deletions.
- `packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/contract_user_moderation/tests.rs`: the whole pipeline, including the moderators' window (a deletion to the millisecond it ends on, refused one later for the contract owner too while the author's own still passes, reopened by a replace, fixed on update), a moderator deleting a post (record, execution proof, the author's balance unchanged, and the control where the author deletes it and is refunded), every refusal of a deletion, an update adding a document type moderators can delete from, a permanent reference to such a type refused, the mempool refusal, the lapse sweep, the moderator set, every refusal code, the lists staying as the contract was created with them, a barred identity deleting its own documents in a block and in the mempool, a barred identity refused as the recipient of a transfer and as the seller of a purchase, the ban's proof covering the suspension it removed, lifting the entry of an identity an update made moderator, the per-list execution proof, a named owner, a create or an update naming a moderator that does not exist, an update keeping its moderators, and inactivity of the transition and of a moderated contract create or update before protocol version 14.
