# Moderation Charters Contract

The moderation charters system contract holds the charters of the moderation
teams that masternodes elect for data contracts that declare an elected
moderation team. It activates at protocol version 14, registered at genesis by
chains born at 14 and inserted by the upgrade to 14, and has the same ID on
every network: `EG7RGfV8fDTayC2FyVr8HwdpJh3fXDbVztcfE94UmN88`.

It has seven document types. All are immutable. The four a charter is made of
(`reason`, `submittedCharter`, `joinRequest`, `electedCharter`) are
undeletable, so everything a charter points at, and the charter itself, is a
fixed text; the three team changes are deletable.

The schema carries almost every rule through its keywords: typed arrays with
a reference per element, a reference resolved through a unique index
(`lookup`), `distinctFrom`, key requirements on key references, the
`encryptedFor` envelope, `maxBytes` for the description's 4096-byte cap and a
`propertyConstraints` rule holding the reward split to 100. What it cannot say,
the cap on additions, is checked by the batch's state validation, and
`SubmittedCharter` in `rs-dpp` only reads a proposal.

Seating writes nothing. Awarding the contest for a target writes the winning
`electedCharter` here, the only one ever written for that target, and the
target's moderation paths read it: its team moderates instead of the interim
moderators, is protected, and may charge its proposal's `moderatorsShare` of a
declared moderators fee (see [the protocol guide](../../docs/protocol/moderation-charters.md#seating)).

## `reason`

A ground for a moderation action. Anyone may file one.

| Property | Type | Meaning |
| --- | --- | --- |
| `code` | string, 3 uppercase letters, required | Unique among the owner's reasons (`byOwnerCode`); what an action shows. An action names a reason by document id (`reasonDocumentId`), and a seated team only one its proposal lists (41203) |
| `label` | string, 1 to 64 characters, required | The reason's name |
| `description` | string, 1 to 1024 characters | What the reason covers and how the team applies it |

## `submittedCharter`

A leader's proposal to moderate one contract, on the contract's own terms:
the target's elected moderation declaration is the team's whole mandate, so
the proposal names no abilities. Its owner is the leader, and must hold a
decryption key bound to this type so join requests can be encrypted to it.

| Property | Type | Meaning |
| --- | --- | --- |
| `targetContractId` | identifier, required, `refersTo` a contract with elected moderation | The contract the team proposes to moderate |
| `description` | string, 1 to 4096 characters and at most 4096 bytes, required | What the team would moderate and how. Informational |
| `reasons` | array of at most 64 unique reason ids, required, each `refersTo` a `reason` | The moderation reasons the team's actions may name; a team with none can take no action |
| `moderatorsShare` | integer 0 to 100 | The percentage of each moderated type's declared moderators fee the team takes; absent is the full amount, 0 a team that will not moderate and takes no rewards |
| `rewardSplit` | object, required | `leader`, `equal` and `actions` percentages summing to 100 (the `rewardSplitIsWhole` rule of `propertyConstraints`): how every settle of the target's moderators pot is paid out, a claim or a team change |

Indexes: `byTargetContract` (target, `$createdAt`) lists the proposals for a
contract in filing order; `byOwner` lists a leader's proposals.

## `joinRequest`

An identity's offer to serve on the team of a proposal, one per identity per
proposal (`bySubmittedCharter`, unique on the proposal and the owner), with a
message only the leader can read. The owner must hold an encryption key bound
to this type.

| Property | Type | Meaning |
| --- | --- | --- |
| `submittedCharterId` | identifier, required, `refersTo` a `submittedCharter` | The proposal; `recipientId` must equal its owner (`propertyAgreement`) |
| `recipientId` | identifier, required, `refersTo` an identity public key through `recipientKeyId` | The leader and the decryption key the message is encrypted to |
| `recipientKeyId` | integer, required | The leader's key id |
| `senderKeyId` | integer, required | The owner's encryption key the shared secret is derived from |
| `encryptedMessage` | bytes, 32 to 1040, required | Why the owner wants to join, encrypted for the leader (ECDH on secp256k1, AES-256-CBC) |

## `electedCharter`

A proposal put to the vote with its team: the only type that opens or joins
the contest for a target. Only the proposal's leader may create one, for the
proposal's own target (`propertyAgreement` on `$ownerId` and
`targetContractId`). The `byTargetContract` index is a contested unique index
keyed by the target contract with resolution `1`, the vote without a Lock
choice by masternodes (weight 1) and evonodes (weight 4): a create on it
opens or joins the contest, a tie goes to the earliest applicant, and a
single applicant is seated when the join window closes. `bySubmittedCharter`
lists the elected charters of a proposal. It is not unique: a type with a
contested unique index may carry no other unique index, and none is needed,
since an identity may contend once per contest and only the proposal's owner
may enter it, so a proposal has at most one contender at a time. The seated
leader and members act with the target's full mandate; there are no powers.

| Property | Type | Meaning |
| --- | --- | --- |
| `targetContractId` | identifier, required, `refersTo` a contract whose election is open | The contract contended for; its own `electionDelay` since its creation must have passed |
| `submittedCharterId` | identifier, required, `refersTo` a `submittedCharter` | The proposal the team runs on |
| `members` | array of at most 15 unique identity ids, required, each the owner of a `joinRequest` for this proposal (`lookup`) and none the leader (`distinctFrom`) | The team besides the leader; may be empty |

## After the election

Once an elected charter is seated, its team can change without a new vote:

| Type | Written by | Properties | Rules |
| --- | --- | --- | --- |
| `addedModerator` | the leader | `electedCharterId`, `submittedCharterId`, `memberId` | `memberId` owns a `joinRequest` for the charter's proposal (`lookup`) and is not the leader; at most the target's `maxAddedModerators` additions per charter at a time, a consensus rule of the batch's state validation (41202); deleting it takes the member off and frees its slot |
| `removedModerator` | the leader | `electedCharterId`, `memberId` | Needs no resignation; `memberId` is one of the charter's elected `members` (`listElement`); deleting it puts the member back |
| `resignationRequest` | a member of the team | `electedCharterId`, `recipientId`, `recipientKeyId`, `senderKeyId`, `encryptedMessage` | The writer is in the charter's `members` or has an addition now (`ownerRefersTo` with `anyOf`, the addition a `deletableDocument` lookup); a message only the leader can read; deletable, which withdraws it; the leader acts on it by deleting the addition or removing an elected member |

Each exists at most once per member and charter (unique indexes). The team
that acts is the leader plus the elected members less the removals, plus the
additions (`ElectedCharter::active_members` in `rs-dpp`).

See [the protocol guide](../../docs/protocol/moderation-charters.md) for
details.

## Install

```sh
npm install @dashevo/moderation-charters-contract
```

## License

[MIT](LICENSE) © Dash Core Group, Inc.
