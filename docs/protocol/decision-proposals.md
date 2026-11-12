# Decision proposals

A decision proposal is a question that any identity puts to the masternodes.
A yes/no one is a `proposal`: creating it opens a masternode yes/no vote poll
on it, which the proposal's owner funds. A multiple choice one is a
`multipleChoiceProposal` and the `choice` documents entered for it, which the
masternodes vote among as a contest (see
[Multiple choice proposals](#multiple-choice-proposals)). A result is signaling
only: nothing on Platform acts on it.

The proposals live in the decision proposals system contract. It activates at
protocol version 15: a chain born at 15 registers it at genesis
(`create_genesis_state` v1, behind a `>= 15` version branch), an older chain
inserts it on the upgrade to 15 (`transition_to_version_15`), and the Drive
system contract cache and the trusted context provider serve it from 15 on.

- Contract ID: `3iNyoNwjCTkb7XS3RHyWUtLx7pgC6AFAEQNNDHzxqngF`
- Owner: the all-zero system identity
- Registry entry: `SystemDataContract::DecisionProposals = 11`
- Schema version: 1
- Document types: `proposal`, `multipleChoiceProposal`, `choice`

## `proposal`

The type is undeletable (`canBeDeleted: false`), so the proposal stays for as
long as its result is read. Its rule (`passPercent`, `minimumTurnoutPercent`
and `endDate`) is listed under `immutable` and never changes, so the poll always runs by the
rule it opened with. The owner may edit `title`, `description` and `url` until
`min(1 day, (endDate - $createdAt) / 5)` after the proposal was created: each is
an `immutable` entry whose condition,
`{ "greaterThan": [{ "subtract": ["$updatedAt", "$createdAt"] }, { "min": [86400000, { "divide": [{ "subtract": ["endDate", "$createdAt"] }, 5] }] }] }`,
freezes it once that time has passed. With proposals running at least a week, a
fifth is at least 1.4 days, so the day comes first today. An edit after the
window, or of the rule at any time, is refused
(`DocumentImmutablePropertyChangedError`, 40128). The type keeps its history
(`documentsKeepHistory: true`), so every version stays readable and a voter can
see the text as it was when it voted. Additional properties are rejected.

| Property | Type | Meaning |
| --- | --- | --- |
| `title` | string, 1 to 128 characters and at most 512 bytes (`maxBytes`), required | The question, in one line |
| `description` | string, 1 to 4096 characters and at most 4096 bytes, required | What a yes and a no mean |
| `url` | string, `format: uri`, 1 to 512 characters and at most 512 bytes | Where the proposal is discussed |
| `passPercent` | integer 50 to 80, required | The share of yes plus no, in percent, that yes must be strictly more than |
| `minimumTurnoutPercent` | integer 0 to 50, required | The least yes plus no voting power the proposal needs, as a percent of the masternode list's total voting power in the block that closes the poll (PoSe-banned masternodes left out), rounded up |
| `endDate` | integer, milliseconds since the Unix epoch, required | When voting closes |

Indexes:

| Index | Properties | Lists |
| --- | --- | --- |
| `byEndDate` | `endDate`, `$createdAt` | The proposals closing soonest first |
| `byOwner` | `$ownerId`, `$createdAt` | An identity's proposals, in filing order |
| `byCreatedAt` | `$createdAt` | The newest proposals |

None is unique: anyone may ask the same question again.

## Opening the poll

A `proposal` create opens a yes/no vote poll in the same batch. The poll's
purpose is `YesNoVotePollPurpose::DecisionProposal { document_id }`, the
proposal's document id, and its id is derived from that purpose alone, so a
proposal has exactly one poll. Its rule is read from the proposal:

- it passes when the yes voting power is strictly more than `passPercent`
  percent of the yes plus no voting power, abstentions left out of both sides
  (a supermajority of `passPercent / 100`), so at 50 a tie fails,
- and the yes plus no voting power reaches `minimumTurnoutPercent` percent of
  the network's total, rounded up (a share minimum of `minimumTurnoutPercent /
  100`, rounded `up`).

A masternode votes with 1 and an evonode with 4. The rule is written to the
poll's stored info when it opens and cannot change after that; a vote names the
poll by id alone.

The create states the fund it pays as its prefunded voting balance: the name
`proposal` and exactly the decision proposal fund. The transition builders fill
it in for a proposal, as they do for a contested document, and a contest fund
given in the creation options leaves it as it is. Any other fund is
refused (`DecisionProposalFundMismatchError`, 11100), as is a rule outside the
limits below (`InvalidDecisionProposalRuleError`, 11101) and an end date outside
them (`DecisionProposalEndDateOutOfRangeError`, 41300). The mempool judges the
end date against the last block and a block against its own time, so a proposal
should not end at the very edge of the window.

Opening the poll prefunds the masternode votes on it: 0.5 Dash
(`decision_proposal_fund_required_amount` in `VoteResolutionFundFees`, set in
`FEE_VERSION4`, protocol version 15) leaves the owner's balance for the poll's
prefunded balance. Each vote takes a fixed cost from it (0.00002 Dash), so the
fund pays for 25,000 votes, more than every masternode casting all of its votes
on it. What the votes leave is released as processing fees when the poll ends.

## Limits

The create is refused unless the rule is within these limits. The schema holds
the percentages to their ranges; the poll's own limits, which `SystemLimits`
carries, back them up:

| Limit | Value | Where |
| --- | --- | --- |
| Pass threshold | 50 to 80 percent | the schema; `min_yes_no_vote_poll_supermajority_percent` (50) for every poll |
| Turnout minimum | 0 to 50 percent of the network | the schema; `max_yes_no_vote_poll_minimum_voting_power_percent_of_total` (50) for every poll |
| End date | one to four weeks after the block that opens the poll | `min_yes_no_vote_poll_duration_ms`, `max_yes_no_vote_poll_duration_ms` |

A threshold of at least a simple majority means a proposal cannot pass with
fewer yes than no. The one-week floor gives masternode operators time to see a
proposal before it closes.

## Reading the result

The poll closes in a block whose time has reached its end date. Its result,
whether it passed with the yes, no and abstain voting power cast and the
voting power it needed, stays in the poll's stored info. Read it with
`getYesNoVotePollState`, naming the poll by its id (`YesNoVotePollPurpose::unique_id`
of the `DecisionProposal` purpose, `decision_proposal_vote_poll_id` in
`dpp::decision_proposal`): the response carries the poll as it was
opened, its rule included, and once it has finished, the result and the blocks
it opened and closed in.

The result is signaling only. Nothing on Platform acts on a passed or failed
proposal (`on_yes_no_vote_poll_finished` does nothing for this purpose); what
it means is up to the people who read it.

## Multiple choice proposals

A multiple choice proposal is two document types. The `multipleChoiceProposal`
is the question; each `choice` is an answer to it, entered by any identity.
The choices of one proposal are the contenders of one
[contest](../../book/src/data-model/contested-documents.md), so the vote among
them runs on the contested document machinery DPNS names use. The contest is a
vote among documents (`resolution: 2`, new in protocol version 15): each choice
is a contender of its own, so an identity may enter several.

### `multipleChoiceProposal`

Undeletable (`canBeDeleted: false`), so a `choice` can refer to it as a
`permanentDocument`. The owner may edit `title` and `url`, and point the
proposal at another description (`descriptionId`, `descriptionContractId`,
`descriptionDocumentType`), until two hours after the proposal was created:
each is an `immutable` entry whose
condition,
`{ "greaterThan": [{ "subtract": ["$updatedAt", "$createdAt"] }, 7200000] }`,
freezes it once that time has passed, and an edit after it is refused
(`DocumentImmutablePropertyChangedError`, 40128). The window runs from the
proposal's creation whether or not a choice has been entered, so a choice
entered in the first two hours may answer a text that is edited afterwards.
The type keeps its history (`documentsKeepHistory: true`), so every version
stays readable. `open` is listed under `immutable` by name and never changes.
Creating one opens no vote and states no fund. Additional properties are
rejected.

| Property | Type | Meaning |
| --- | --- | --- |
| `title` | string, 1 to 128 characters and at most 512 bytes, required | The question, in one line |
| `descriptionId` | identifier, required | The document saying what is being decided: a [reference whose target the proposal names](../../book/src/contract-keywords/refers-to-dynamic.md) |
| `descriptionContractId` | identifier, required | The contract that document lives in: `refersTo` a `contract` with `contractRequirements: { "moderation": "elected", "ownerProtected": false }` |
| `descriptionDocumentType` | string, 1 to 64 characters, required | That document's document type |
| `url` | string, `format: uri`, 1 to 512 characters and at most 512 bytes | Where the proposal is discussed |
| `open` | boolean, required | Whether anyone may enter choices. Only the proposal's owner may when it is `false` |

Indexes: `byOwner` (`$ownerId`, `$createdAt`) and `byCreatedAt` (`$createdAt`),
neither unique.

#### The description

A proposal costs its storage and nothing else, so text held in it would be
cheap to fill with anything and could never be taken down: the proposal is
undeletable. It therefore holds no description. It points at a document that
does, hosted where moderators can remove it:

```json
"descriptionId": {
  "type": "identifier",
  "refersTo": {
    "type": "moderatedDocument",
    "contractId": "$self.descriptionContractId",
    "documentType": "$self.descriptionDocumentType",
    "documentTypeRequirements": {
      "when": <removable by the elected team>,
      "properties": {
        "body":      { "name": "$any", "type": "string", "when": <taken with the document> },
        "bodyBytes": { "name": "$any", "type": "array", "byteArray": true, "when": <taken with the document> }
      },
      "require": { "anyOf": ["body", "bodyBytes"] }
    }
  }
},
"descriptionContractId": {
  "type": "identifier",
  "refersTo": {
    "type": "contract",
    "contractRequirements": { "moderation": "elected", "ownerProtected": false }
  }
}
```

where `<removable by the elected team>` is

```json
{ "allOf": [
  { "contains": ["$electedAbilities", { "const": "deleteDocuments" }] },
  { "anyOf": [
    { "absent": "$moderatorAbilities.deleteWithin" },
    { "present": "$moderatorAbilities.deleteSettled" }
  ] }
] }
```

and `<taken with the document>` is

```json
{ "allOf": [
  { "not": { "contains": ["$moderatorAbilities.deleteKeepsFields", "$property"] } },
  { "not": { "contains": ["$moderatorAbilities.changeFields", "$property"] } }
] }
```

Checked when the proposal is created, and again when an edit points it
elsewhere. The contract is checked on its own property; the rows after it are
in the order the description's type is checked, the first that applies
refusing the write:

| What the proposal names | Refused with |
| --- | --- |
| A contract that does not declare elected moderation, or whose declaration protects its owner from the team | `ReferencedContractRequirementNotMetError` (40135) |
| A contract that does not exist, or has no such document type | `ReferencedDocumentTypeNotFoundError` (40121) |
| A private document type, whose documents are encrypted entries the chain never reads | `ReferencedDocumentTypePrivateError` (40150) |
| An `indexOnly` document type, whose documents are index entries with none to find by its id | `ReferencedDocumentTypeIndexOnlyError` (40146) |
| A type its contract's elected team may not delete from (the contract elects no team, or leaves the type out of the team's `deleteDocuments`), or one moderators delete only within a window (`deleteWithin`) with no rule for later (`deleteSettled`) | `ReferencedDocumentTypeRequirementNotMetError` (40153) |
| A moderated type with no string or byte array a removal takes away: every one is kept in the removal record (`deleteKeepsFields`) or is the moderators' to write (`changeFields`) | `ReferencedDocumentTypeRequirementNotMetError` (40153) |
| A document type whose documents their owner can delete, or that never leave state | `ReferencedDocumentTypeNotModeratedError` (40143) |
| A document that is not there | `ReferencedEntityNotFoundError` (40120) |

What this does and does not give:

- A moderator's removal of the description leaves the proposal in place,
  pointing at the removal record: the question and its vote stay, the text is
  gone.
- The contract declares a team the masternodes elect. It does not have to have
  seated one yet: until it does, whoever the contract names for the interim
  moderates, which may be its owner or nobody. How long that lasts is the
  contract's to say: its `electionDelay`, counted from its creation, holds off
  the first election, and has no upper bound short of some 136 years. A
  reference requiring `"moderation": "electionOpen"` would admit only a
  contract whose delay has passed; it would still not require a seated team.
- The team may moderate the contract's owner like anyone: a contract whose
  declaration protects its owner (`ownerProtected`) is refused, since a
  description the owner wrote there could not be removed.
- The team is given the deletion of the description's type, and a description
  stays deletable. A contract that leaves the type out of the abilities it
  gives its elected team (`deleteDocuments`) is refused, since once a team is
  seated nobody could remove its documents. So is a type that lets moderators
  delete only within a window after a document's last change
  (`moderatorAbilities.deleteWithin`) without saying who may delete a settled
  one (`deleteSettled`), since a description would outlast the window for
  good. A settled document is deleted by a seated team alone, so within the
  interim a description past its window waits for the first election.
- The description's own edit rules are its document type's. A type that lets
  its author replace a document lets the author change the description after
  the two hours, and after choices were entered. The proposal's history keeps
  which document it pointed at, not what that document said.
- Which property of the document is the description is the first, in the
  type's `position` order, that fills the `body` slot, then `bodyBytes`
  (see [the binding rule](../../book/src/contract-keywords/refers-to-dynamic.md#documenttyperequirements)).
- `title` and `url` are still text of the proposal.

### `choice`

Immutable and undeletable. Additional properties are rejected.

| Property | Type | Meaning |
| --- | --- | --- |
| `proposalId` | identifier, required | The proposal this answers: `refersTo` a `permanentDocument` of type `multipleChoiceProposal`, so a choice for a proposal that does not exist is refused (`ReferencedEntityNotFoundError`, 40120) |
| `title` | string, 1 to 128 characters and at most 512 bytes, required | The answer, in one line |
| `description` | string, 1 to 4096 characters and at most 4096 bytes | What voting for this answer means |

Its only index, `byProposal` on `proposalId`, is unique and `contested` with
`resolution: 2` and no `fieldMatches`, so every choice is contested, each is a
contender named by its document id, and the contest is keyed by the proposal's
id:
`ContestedDocumentResourceVotePoll { contract_id, document_type_name: "choice", index_name: "byProposal", index_values: [proposalId] }`.

One `propertyConstraints` rule, `closedProposalTakesItsOwnersChoices`, reads
the proposal through `proposalId`:
`{ "ifThen": [{ "equal": [{ "referenced": ["proposalId", "open"] }, 0] }, { "equal": [{ "referenced": ["proposalId", "$ownerId"] }, "$ownerId"] }] }`.
If the proposal is not open (a rule reads `false` as 0), the choice's owner
must be the proposal's owner; a choice anyone else enters is refused
(`DocumentPropertyConstraintViolatedError`, 10422) and opens no vote.

### The vote

- **Entering a choice.** The create states the contest's fund as its prefunded
  voting balance, `["byProposal", amount]`, which the transition builders fill
  in: the contested document fund, 0.1 Dash, doubling once the contest holds
  250 choices and again for every 50 more. The first choice opens the vote;
  later ones join it while the join window is open (one week on mainnet) and
  are refused after it (`DocumentContestNotJoinableError`, 40111). A vote holds
  at most 1,000 choices.
- **Who enters choices, and how many.** A choice is a contender by itself, kept
  under its document id, so an identity enters as many as it wants, each
  paying the fund. For an open proposal anyone may enter choices, the proposer
  included, and the proposer does not decide which choices the vote holds. For
  a proposal that is not open only the proposer may, so the proposer supplies
  every choice.
- **Voting.** Masternodes and evonodes vote with `MasternodeVote` transitions
  for the length of the poll (two weeks on mainnet, counted from the first
  choice): towards a choice; abstain; or lock, which here means none of the
  choices. A vote towards a choice is a `TowardsIdentity` vote choice carrying
  the choice's document id, not its owner's identity id, which names no
  contender here and is refused (`VoteChoiceNotAllowedForVotePollError`,
  40307).
- **The result.** The choice with the most votes is stored as an ordinary
  document and the other choices are removed. A tie goes to the earliest
  choice, so a vote in which nobody votes keeps the first choice entered. When
  the lock votes outnumber the leading choice, no choice is stored, and no
  choice can be entered for that proposal again
  (`DocumentContestCurrentlyLockedError`, 40110). The vote runs its full
  length even with a single choice. The stored result names the winning choice
  by its document id, and the choice is stored under the identity that entered
  it.
- **Reading it.** `getContestedResourceVoteState` returns the choices and their
  tallies while the vote runs and its outcome afterwards. Once the vote has
  ended, the text of the choices that were not kept is no longer in state. The
  stored result, which the query returns, still lists every choice by its
  document id with its tally.

Unlike a yes/no proposal, a multiple choice proposal has no pass threshold, no
minimum turnout and no end date of its own: the contest's windows and its
most-votes rule apply.

## From the SDKs

The Rust SDK (`dash_sdk::platform::decision_proposals`), the JavaScript SDKs
(`sdk.decisionProposals` in `@dashevo/evo-sdk`, the `*DecisionProposal*`
methods of `@dashevo/wasm-sdk`) and the mobile SDKs (the
`dash_sdk_decision_proposal_*` C functions of `rs-sdk-ffi`, wrapped by the
Swift SDK and by the Kotlin SDK's `Sdk.decisionProposals` and
`PlatformWalletManager.decisionProposals`) build a proposal to create as a document, give
an existing one new text to replace it with, list proposals through the three
indexes a page at a time, and read and answer a proposal's poll by the
proposal's id. A masternode's vote names the poll by its id, which the SDKs
derive from the proposal's id with the protocol version they speak.

A multiple choice proposal and its choices are ordinary documents and an
ordinary contest: the SDKs' document create builds both (a `choice` create is
given its contest fund like any contested create), and their contested resource
queries and masternode vote read and answer the vote, with a choice's document
id wherever they name a contender by an identity id. They have no helpers of
their own yet.
