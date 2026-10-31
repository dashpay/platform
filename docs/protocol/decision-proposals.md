# Decision proposals

A decision proposal is a yes/no question that any identity puts to the
masternodes. Creating one opens a masternode yes/no vote poll on it, which the
proposal's owner funds. The result is signaling only: nothing on Platform acts
on it.

The proposals live in the decision proposals system contract. It activates at
protocol version 15: a chain born at 15 registers it at genesis
(`create_genesis_state` v1, behind a `>= 15` version branch), an older chain
inserts it on the upgrade to 15 (`transition_to_version_15`), and the Drive
system contract cache and the trusted context provider serve it from 15 on.

- Contract ID: `3iNyoNwjCTkb7XS3RHyWUtLx7pgC6AFAEQNNDHzxqngF`
- Owner: the all-zero system identity
- Registry entry: `SystemDataContract::DecisionProposals = 11`
- Schema version: 1
- Document types: `proposal`

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
