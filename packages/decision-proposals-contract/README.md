# Decision Proposals Contract

The decision proposals system contract holds the questions that anyone puts to
the masternodes: yes/no questions (`proposal`) and multiple choice ones
(`multipleChoiceProposal` and `choice`). It activates at protocol version 15, registered at genesis
by chains born at 15 and inserted by the upgrade to 15, and has the same ID on
every network: `3iNyoNwjCTkb7XS3RHyWUtLx7pgC6AFAEQNNDHzxqngF`.

Creating a `proposal` opens a masternode yes/no vote poll on it, whose result
stays readable with `getYesNoVotePollState`. Entering a `choice` for a
`multipleChoiceProposal` opens a masternode vote among the proposal's choices,
whose result stays readable with `getContestedResourceVoteState`. Both are
signaling only: nothing on Platform acts on a result.

## `proposal`

A yes/no question. Any identity may create one, and nobody may delete it.
Its rule (the threshold, the minimum and the end date) never changes, so
masternodes always vote by the rule the poll opened with. Its owner may edit the
`title`, `description` and `url` for a day after creating it, or for a fifth of
the time it runs if that is shorter; after that they freeze too. Every version
is kept (`documentsKeepHistory`), so a voter can see the text as it was when it
voted.

| Property | Type | Meaning |
| --- | --- | --- |
| `title` | string, 1 to 128 characters and at most 512 bytes, required | The question, in one line |
| `description` | string, 1 to 4096 characters and at most 4096 bytes, required | What a yes and a no mean |
| `url` | string, a URI of 1 to 512 characters and at most 512 bytes | Where the proposal is discussed |
| `passPercent` | integer 50 to 80, required | The proposal passes when yes is strictly more than this percent of yes plus no |
| `minimumTurnoutPercent` | integer 0 to 50, required | The least yes plus no voting power the proposal needs, as a percent of the masternode list's total voting power when the poll closes, rounded up (a masternode votes with 1, an evonode with 4) |
| `endDate` | integer, milliseconds since the Unix epoch, required | When voting closes |

Additional properties are rejected.

Indexes: `byEndDate` (`endDate`, `$createdAt`) lists the proposals closing
soonest first; `byOwner` (`$ownerId`, `$createdAt`) lists an identity's
proposals; `byCreatedAt` (`$createdAt`) lists the newest.

## The poll

At protocol version 15, a `proposal` create opens a yes/no vote poll for the
proposal and prefunds the masternode votes on it with 0.5 Dash from the
owner's balance (`decision_proposal_fund_required_amount`). What the votes
leave is released as processing fees when the poll ends.

The poll's rule is read from the proposal and must be within the protocol's
limits, or the create is refused:

- `passPercent` is 50 to 80: at 50, yes must be strictly more than no;
- `minimumTurnoutPercent` is at most 50;
- the end date is one to four weeks after the block that opens the poll.

## `multipleChoiceProposal`

A question whose answers are the choices entered for it. Any identity may
create one, and nobody may delete it. Its owner may edit the `title` and `url`,
and point it at another description, for two hours after creating it; after
that they freeze. Every version is kept (`documentsKeepHistory`), so a reader
can see the proposal as it was when a choice was entered. Whether it is `open`
never changes. Creating it opens no vote and states no fund: the first choice
does.

The proposal holds no description of its own. What is being decided is said
in one of two places, or both: at its `url`, for a description hosted
anywhere, which nothing on chain reads or moderates; or in a document the
proposal points at, in another contract, so that the description can be
taken down. A proposal gives at least one, and names a description document
with all three of its properties or none (the `describedByADocumentOrAUrl`
and `descriptionDocumentNamedWhole` rules). For a document, the document type
is a moderated one (its documents leave state
only through a moderator's recorded removal) holding a string or a byte array
that the removal takes away and that the document's author writes, the
contract is one whose moderators the masternodes elect and whose owner they
may moderate, and it gives that team the deletion of the type's documents,
at any time after they are written.

| Property | Type | Meaning |
| --- | --- | --- |
| `title` | string, 1 to 128 characters and at most 512 bytes, required | The question, in one line |
| `descriptionId` | identifier | The document saying what is being decided, when the proposal points at one |
| `descriptionContractId` | identifier, given with `descriptionId` and only with it | The contract that document lives in, one declaring elected moderation that does not protect its owner from the team |
| `descriptionDocumentType` | string, 1 to 64 characters, given with `descriptionId` and only with it | That document's document type |
| `url` | string, a URI of 1 to 512 characters and at most 512 bytes, required when `descriptionId` is left out | Where what is being decided is described, for a proposal that points at no description document; where it is discussed, for one that does |
| `open` | boolean, required | Whether anyone may enter choices. Only the proposal's owner may when it is false |

Indexes: `byOwner` (`$ownerId`, `$createdAt`) lists an identity's proposals;
`byCreatedAt` (`$createdAt`) lists the newest.

## `choice`

An answer to a multiple choice proposal. Nobody may change or delete it. A
choice leaves state only when its vote ends, which removes the choices that
were not kept.

| Property | Type | Meaning |
| --- | --- | --- |
| `proposalId` | identifier, required | The `multipleChoiceProposal` this answers, which must exist |
| `title` | string, 1 to 128 characters and at most 512 bytes, required | The answer, in one line |
| `description` | string, 1 to 4096 characters and at most 4096 bytes | What voting for this answer means |

Its only index, `byProposal` (`proposalId`), is unique and contested for every
value, so the choices entered for one proposal contend in one masternode vote.
The vote is among documents (`resolution: 2`): each choice is a contender of
its own, named by its document id.

- anyone may enter choices for an open proposal, the proposer included, and
  only the proposer for one that is not open (the
  `closedProposalTakesItsOwnersChoices` rule);
- an identity enters as many choices as it wants, each paying the fund;
- the first choice opens the vote, and later ones join it while its join window
  is open (one week on mainnet);
- each choice prefunds the masternode votes with the contested document fund:
  0.1 Dash, doubling once the vote holds 250 choices and again for every 50
  more. A choice paying less than the fund at that moment is refused
  (`DocumentContestNotPaidForError`, 40114);
- masternodes vote for a choice by its document id, abstain, or lock, which
  here means none of the choices;
- when the vote ends (two weeks after the first choice on mainnet), the choice
  with the most votes is stored and the others are removed, unless the lock
  votes outnumber it, in which case none is stored. A tie goes to the earliest
  choice, and among choices entered in the same block to the lower document
  id.

See [the protocol guide](../../docs/protocol/decision-proposals.md) for
details.

## Install

```sh
npm install @dashevo/decision-proposals-contract
```

## License

[MIT](LICENSE) © Dash Core Group, Inc.
