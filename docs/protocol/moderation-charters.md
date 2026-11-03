# Moderation charters

The moderation charters system contract holds how the moderation team of a
contract that declares elected moderation comes to be: the reasons a team may
act on, a leader's proposal, the identities that offer to join it, and the
proposal put to the vote with its team. It activates at protocol version 14:
a chain born at 14 registers it at genesis (`create_genesis_state` v1, behind
the same version branch as the app-connect contract), an older chain inserts it
on the upgrade to 14 (`transition_to_version_14`), and the Drive system contract
cache and the trusted context provider serve it from 14 on.

Seating writes nothing. Awarding the contest for a target writes the winning
`electedCharter` to this contract's storage, the only one ever written there
for that target, so the charter seated on a contract is the one
`byTargetContract` finds, and the target's moderation paths read it from here
(see [Seating](#seating)).

- Contract ID: `EG7RGfV8fDTayC2FyVr8HwdpJh3fXDbVztcfE94UmN88`
- Owner: the all-zero system identity
- Registry entry: `SystemDataContract::ModerationCharters = 10`
- Schema version: 1
- Document types: `reason`, `submittedCharter`, `joinRequest`, `electedCharter`,
  `addedModerator`, `removedModerator`, `resignationRequest`

Every type is immutable. `reason`, `submittedCharter`, `joinRequest` and
`electedCharter` are undeletable, so each document another one refers to
permanently stays exactly as it was when it was referred to. The three team
changes are deletable: `addedModerator` and `removedModerator`, which the
leader takes back by deleting them, and `resignationRequest`, which its writer
withdraws. Additional properties are rejected on every type.

## The flow

1. Anyone files `reason` documents, or reuses someone else's.
2. A leader files a `submittedCharter` for a target contract that declares
   elected moderation. Proposals may be filed during the target's election
   delay, so a team can form before the election opens.
3. Identities that want to serve file a `joinRequest` for the proposal, with a
   message encrypted to the leader.
4. Once the target's election delay has passed, the leader files an
   `electedCharter` naming the proposal and the members chosen from those who
   asked to join. That create opens or joins the contest for the target.

5. After the election the leader may add members from the same join requests,
   at most the target's `maxAddedModerators` at a time, and take them off again
   by deleting the addition; an elected member is taken off with a removal,
   and put back by deleting it. A member asks to leave with a resignation
   request, which the leader acts on.

The seated team acts with the target contract's whole elected moderation
declaration: every document type and ability it lists. A team narrows what it
acts on only through the reasons its proposal lists, since every action names
one. There are no powers: any one member acts alone. The team that acts is:

```
leader + (electedCharter.members - removedModerator.memberId)
       + addedModerator.memberId
```

where both lists are the documents that exist now, and a removal can only name
one of the charter's `members`.

## `reason`

A ground for a moderation action.

| Property | Type | Meaning |
| --- | --- | --- |
| `code` | string, three uppercase letters, required | Unique among the owner's reasons (`byOwnerCode`, unique on `$ownerId` and `code`); what an action shows. An action names the reason by its document id, in its reason's `reasonDocumentId` |
| `label` | string, 1 to 64 characters, required | The reason's name, such as Spam |
| `description` | string, 1 to 1024 characters | What the reason covers and how the team applies it |

Two owners may both file a `SPM`; a proposal says which one it means by
document id.

## `submittedCharter`

A leader's proposal. Its owner is the leader. The type declares
`requiresIdentityDecryptionBoundedKey`, so the leader can hold a decryption key
bound to it, the key join requests are encrypted to.

| Property | Type | Meaning |
| --- | --- | --- |
| `targetContractId` | identifier, required, `refersTo: { "type": "contract", "contractRequirements": { "moderation": "elected" } }` | The contract the team proposes to moderate; a target that does not exist refuses the create (40120), one that does not declare elected moderation refuses it with `ReferencedContractRequirementNotMetError` (40135) |
| `description` | string, 1 to 4096 characters and at most 4096 bytes (`maxBytes`), required | What the team would moderate and how, for joiners and voters. Informational |
| `reasons` | typed array of at most 64 unique identifiers, required, each `refersTo` a `reason` | The moderation reasons the team's actions may name; empty is allowed, a team that can take no action; a missing reason refuses the create, naming the element (`reasons[2]`) |
| `moderatorsShare` | integer 0 to 100 | The percentage of each moderated document type's declared moderators fee the team takes, rounded down to the credit. Absent is the full amount; a lower number is a discount an action may agree to once the team is seated; 0 is a team that will not moderate and takes no rewards |
| `rewardSplit` | object, required | `leader`, `equal` and `actions`, three percentages summing to 100: the leader's share, the share split equally among the other members (the leader's when it has none), and the share split between the whole team, the leader included, by each one's action count since the last settle (equally when nobody acted). The sum is the type's `propertyConstraints` rule `rewardSplitIsWhole`, checked on every create (`DocumentPropertyConstraintViolatedError`, 10422) |

Indexes: `byTargetContract` (`targetContractId`, `$createdAt`) lists the
proposals for a contract in filing order; `byOwner` (`$ownerId`) lists a
leader's proposals.

A type's declared moderators amount is already the most a team may charge, so a
proposal can only lower the price, and the signer's fee agreement to the
declared amounts never mismatches a seated team.

## `joinRequest`

An identity's offer to serve on the team of a proposal. The owner is the one
offering, so the offer is consent the owner signed; no property names the
joiner. The type declares `requiresIdentityEncryptionBoundedKey`.

| Property | Type | Meaning |
| --- | --- | --- |
| `submittedCharterId` | identifier, required, `refersTo` a `submittedCharter` with `propertyAgreement: { "recipientId": "$ownerId" }` | The proposal; `recipientId` must be its owner, the leader |
| `recipientId` | identifier, required, `refersTo: { "type": "identityPublicKey", "keyIdProperty": "recipientKeyId", "keyRequirements": { "purpose": "decryption", "boundTo": "submittedCharter" } }` | The leader, and through `recipientKeyId` the key the message is encrypted to: a decryption key bound to this contract's `submittedCharter` type |
| `recipientKeyId` | integer 0 to 4294967295, required | The leader's key id |
| `senderKeyId` | integer 0 to 4294967295, required, `refersTo: { "type": "identityPublicKey", "identityProperty": "$ownerId", "keyRequirements": { "purpose": "encryption", "boundTo": "joinRequest" } }` | The owner's encryption key, bound to this contract's `joinRequest` type, the shared secret is derived from |
| `encryptedMessage` | bytes, 32 to 1040, required, `encryptedFor` recipient `recipientId`, keys `recipientKeyId` and `senderKeyId`, scheme `ecdh-secp256k1-aes256-cbc` | Why the owner wants to join, readable by the leader alone: a 16-byte IV followed by AES-256-CBC blocks under the ECDH shared key, the scheme dashpay contact requests use. Consensus checks only the shape |

Indexes: `bySubmittedCharter` (`submittedCharterId`, `$ownerId`), unique, so
one offer per identity per proposal, and the index an elected charter's
members are looked up through; `byOwner` (`$ownerId`). The type is neither
transferable nor tradeable, which a lookup keyed on `$ownerId` requires.

## `electedCharter`

A proposal put to the vote with its team. Its owner is the leader.

| Property | Type | Meaning |
| --- | --- | --- |
| `targetContractId` | identifier, required, `refersTo: { "type": "contract", "contractRequirements": { "moderation": "electionOpen" } }` | The contract contended for; it must declare elected moderation and its own `electionDelay` since its creation must have passed (40135 otherwise) |
| `submittedCharterId` | identifier, required, `refersTo` a `submittedCharter` with `propertyAgreement: { "$ownerId": "$ownerId", "targetContractId": "targetContractId" }` | The proposal the team runs on: only its owner may file this, and for the proposal's own target |
| `members` | typed array of at most 15 unique identifiers, required, elements `distinctFrom: "$ownerId"` and `refersTo` a `joinRequest` through `lookup: { "index": "bySubmittedCharter", "keys": { "submittedCharterId": "submittedCharterId", "$ownerId": "." } }` | The team besides the leader; may be empty. Each member must be the owner of a join request for this proposal, found through the join request's unique index, and none may be the leader |

The lookup reads: for each member, the join request whose
`submittedCharterId` is this document's `submittedCharterId` and whose owner is
the member. A member with no such request refuses the create with
`ReferencedEntityNotFoundError` (40120), naming the element (`members[1]`).

Indexes: `byTargetContract`, the contested index below, and
`bySubmittedCharter` (`submittedCharterId`), which lists the elected charters
of a proposal. It is not unique: a type with a contested unique index may carry
no other unique index. None is needed: an identity may be a contestant once
per contest (`DocumentContestIdentityAlreadyContestantError`), every entry of a
proposal lands in its target's contest, and only the proposal's owner may
enter, so a proposal has at most one contender at a time. Contenders live in
the contest until it is awarded, so this index lists seated charters only.

## After the election

All three types refer to an `electedCharter`. A reference finds a document in the
type's own storage, and only a winner is ever written there (contenders live in
the contest), so these documents can only name a seated charter. Each is
unique on the charter and the member, so a member has at most one of each at a
time, and each can be deleted: an addition to take the member off, a removal to
put an elected member back, a request to withdraw it.

| Type | Properties | Rules |
| --- | --- | --- |
| `addedModerator` | `electedCharterId`, `submittedCharterId`, `memberId` | `electedCharterId` carries `propertyAgreement: { "$ownerId": "$ownerId", "submittedCharterId": "submittedCharterId" }`: only the leader adds, and `submittedCharterId` is the charter's proposal. `memberId` refers to a `joinRequest` through the same `lookup` as `members` and is `distinctFrom: "$ownerId"`: an addition needs the member's consent, disclosed on the proposal |
| `removedModerator` | `electedCharterId`, `memberId` | Only the leader removes (`propertyAgreement: { "$ownerId": "$ownerId" }`); no resignation is needed; `memberId` must be one of the charter's elected `members` (`refersTo: { "type": "listElement", "documentType": "electedCharter", "propertyAgreement": { "electedCharterId": "$id" }, "inList": "members" }`, 40120 otherwise): an added member is taken off by deleting its addition. Deleting a removal puts the member back |
| `resignationRequest` | `electedCharterId`, `recipientId`, `recipientKeyId`, `senderKeyId`, `encryptedMessage` | The owner is the member asking to leave, and must be on the team: `ownerRefersTo: { "anyOf": [...] }` requires the writer to be an element of the elected charter's `members` (`listElement`, the charter found by `electedCharterId` through `propertyAgreement: { "electedCharterId": "$id" }`) or the `memberId` of an `addedModerator` for that charter (a `deletableDocument` reference with a `lookup` through `byElectedCharterMember`, `"."` the writer: the addition must exist when the request is filed). The leader is in neither list, so it cannot file one. The message is encrypted to the leader (`propertyAgreement: { "recipientId": "$ownerId" }` on `electedCharterId`) with the leader's decryption key bound to `submittedCharter` and the member's encryption key bound to `joinRequest`, the keys join requests use. A request changes nothing by itself: the leader acts on it by deleting the member's addition, or with a `removedModerator` for an elected member. Deleting it withdraws the request, and nothing refers to it |

**The cap on additions.** The target contract's elected declaration carries
`maxAddedModerators`: how many members a seated team's leader may have added
at a time, 0 when left out and at most
`SystemLimits::max_contract_moderation_added_moderators` (15). It counts the
charter's additions that exist now, so deleting one frees its slot. The schema
cannot count documents, so a consensus rule refuses an
addition over the cap, paid, with `ModerationCharterAddedModeratorLimitReachedError`
(41202): the batch's state validation reads the charter, its target and at most
the cap's number of additions, all billed, once the addition's own references
passed. Like a unique index conflict, it is judged in the block and not in the
mempool, which runs no state validation for a batch: an addition over the cap
is admitted and then refused, paid.

## The contest

The `byTargetContract` index of `electedCharter` is a contested unique index
keyed by the target contract, with `"resolution": 1`:

```json
{
  "name": "byTargetContract",
  "properties": [{ "targetContractId": "asc" }],
  "unique": true,
  "contested": { "resolution": 1 }
}
```

Resolution `1` is `ContestedIndexResolution::MasternodeVoteNoLocking`:
masternodes (weight 1) and evonodes (weight 4) vote for a contender or abstain,
with no Lock choice, so the contest always ends with a winner, a tie goes to
the earliest contender, and a contest with a single contender at the end of the
join window is awarded at once. An elected charter create opens or joins that
contest for its target contract.

The contest runs on the target contract's own windows, on every network: the
join window is the target's `joinWindow`, and a second applicant moves the end
to `joinWindow` plus `voteWindow`. An application prefunds the masternode
votes with 0.5 Dash (`moderation_vote_resolution_fund_required_amount`), not
the 0.1 Dash of other contests; what the votes leave is released as processing
fees when the contest is cleaned up. The target's declaration is read when an
application opens or joins the contest, never when it ends.

The contested key is the target contract alone, with no round or seat
component, and it stays so. A contract's seat is filled once: in protocol
version 14 an elected charter for a target whose seat is taken is refused, paid,
with `DuplicateUniqueIndexError` (40105), the seated charter holding the index,
and the seated charter keeps the seat. Challenges come after 14, and a challenge will be
a new contest on this same unique index, not a document type of its own, open
only against a target whose elected declaration says its seat can be contested
(`seatContestable: true`, with the `challengeCoolDown` that must have passed
since the last seat change). A target declaring `seatContestable: false` keeps
its first team for good. The key is required on every elected declaration, and
frozen with it, so it is in place before any challenge can read it.

## Seating

Nothing is written when a contest is awarded, and nothing is copied under the
moderated contract: the charter seated on a contract is its `electedCharter`
in this contract's storage, found through `byTargetContract` (only a contest's
winner is ever written there, and in protocol version 14 a seat is never
replaced). The moderation paths of the target read it:

- **Moderation.** Once a charter is seated, only its team moderates the
  target: the leader and the active members, each alone. The interim
  moderators, the owner among them, are refused
  (`IdentityNotContractModeratorError`, 41101); before a charter is seated the
  interim rules apply as they did. The signer check lists no team: the leader
  is the charter's owner, an elected member costs a point read of
  `removedModerator`, anyone else a point read of `addedModerator` (both
  unique on `electedCharterId` and `memberId`).
- **Abilities.** The team holds the abilities the target's declaration gives
  it: a deletion or a restore needs `deleteDocuments` on the type, a list
  action the ability on some moderated type
  (`ContractModerationAbilityNotGrantedError`, 41201 otherwise).
- **Protection.** The leader and the active members can be neither put on a
  list nor have their documents deleted (41102), and the owner too when the
  declaration protects it.
- **The interim block.** A `notYetUsable` interim stops blocking the
  moderated types.
- **Fees.** An action agreeing to the declared moderators fee reads no
  charter. One agreeing to less must agree to exactly the seated proposal's
  `moderatorsShare` of it (rounded down to the credit) and is charged that,
  at the cost of the charter lookup and the proposal fetch
  (`DocumentActionFeeModeratorsShareMismatchError`, 40139, for any other
  amount, and for a discount with no seated charter).
- **Reasons.** Every ban, suspension, warning and document deletion of the
  team names, in its reason's `reasonDocumentId`, a `reason` document the
  seated proposal lists; any other is refused, paid, in a block and in the
  mempool (`ModerationReasonNotListedError`, 41203), a reason naming none
  included, so a proposal listing no reason can take no such action. The
  proposal is read, billed, only when a reason document is named. Lifting and
  restoring carry no reason and are not checked, and the interim is not bound.
- **The pot.** The interim team's claim of the moderators pot is refused once
  a charter is seated (41113); the pot carries over to the seated team. The
  leader or an active member claims it for the team, at most once per epoch,
  and it is paid out by the proposal's `rewardSplit`: the leader share to the
  leader, the equal share in equal parts to the other active members (to the
  leader when there are none), and the action share between the whole team
  by the bans, suspensions, warnings and document deletions each one signed
  since the last settle, equally when nobody acted. Every share and every
  part rounds down to the credit; what is left stays in the pot. The counts
  live under the target contract (key `48` of its other tree) and every
  settle deletes them.
- **Settles before team changes.** Creating or deleting an `addedModerator`
  or a `removedModerator` first pays the pot out to the team as it was, the
  same way, and resets the counts, whatever the epoch's claim: the settle
  writes no last claim, and the team may still claim in the same epoch.
- **Resignations.** A `resignationRequest` changes nothing by itself: the
  leader acts on it by deleting the member's `addedModerator`, or with a
  `removedModerator` for an elected member.

## Validation beyond the schema

Every rule above is enforced by the schema's keywords when a document is
written, the description's 4096-byte cap and the reward split's sum included:
`maxBytes` refuses a longer description with
`DocumentPropertyMaxBytesExceededError` (10421), and the `propertyConstraints`
rule `rewardSplitIsWhole` refuses a split that does not add up to 100 with
`DocumentPropertyConstraintViolatedError` (10422). The cap on additions is the
exception (see above), and the settle a team change forces is an effect of the
change, not a rule on it (see [Seating](#seating)).
`SubmittedCharter::from_document_properties` in `rs-dpp`
(`packages/rs-dpp/src/moderation_charter/`) only reads a proposal, without
reading state:

| Rule | Error | Code |
| --- | --- | --- |
| A property is missing or of the wrong type | `ModerationCharterMalformedFieldError` | 11000 |

`ElectedCharter` reads an elected charter's properties, and
`moderation_charter::moderators_share_of` applies a proposal's share to a
declared moderators fee.

## Reading and writing from a client

Every read is an ordinary proved document query on the system contract, through
the indexes above; no endpoint is specific to charters. The Rust SDK
(`dash_sdk::platform::moderation_charters`) and the JavaScript SDK
(`sdk.moderationCharters` in `@dashevo/evo-sdk`) offer them by name:

| Read | Query | Rust | JavaScript |
| --- | --- | --- | --- |
| A contract's seated charter | `electedCharter.byTargetContract`, at most one | `Sdk::fetch_seated_charter` | `seatedCharter` |
| A proposal | `submittedCharter` by id | `Sdk::fetch_submitted_charter` | `submittedCharter` |
| The team | the seated charter, then `addedModerator` and `removedModerator` by `byElectedCharterMember`, combined as `ElectedCharter::active_members` does | `Sdk::fetch_moderation_team` | `team` |
| The proposals for a contract | `submittedCharter.byTargetContract`, in filing order, paged | `Sdk::fetch_submitted_charters` | `submittedCharters` |
| The join requests for a proposal | `joinRequest.bySubmittedCharter`, paged | `Sdk::fetch_join_requests` | `joinRequests` |
| A charter's pending resignation requests | `resignationRequest.byElectedCharterOwner` whose writer is still on the team | `Sdk::fetch_pending_resignation_requests` | `pendingResignationRequests` |

`Sdk::build_join_request` and `Sdk::build_resignation_request`
(`buildJoinRequest` and `buildResignationRequest` in JavaScript) build the two
documents whose message only the leader reads. They pick the keys the schema's
`keyRequirements` demand, the leader's decryption key bound to
`submittedCharter` and the writer's encryption key bound to `joinRequest`,
encrypt the message and set `recipientId`, `recipientKeyId` and `senderKeyId`.
The encryption is the generic `encryptedFor` helper
(`dash_sdk::platform::encrypted_for`, `sdk.encryptedFor`), which reads the
declaration from any contract; the leader decrypts with it too.
