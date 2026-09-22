# Contested Documents

A unique index may be declared `contested`. A document whose index values fall in the contested
range is not stored outright: it opens or joins a **contest**, a `ContestedDocumentResourceVotePoll`
that names the contract, the document type, the index and the index values, and masternodes and
evonodes decide who gets the value. A masternode's vote counts once, an evonode's four times. Every
vote is a `MasternodeVote` state transition carrying the poll and a `ResourceVoteChoice`.

The contest is funded by the contenders' prefunded voting balances, and each vote costs a fixed
amount from that balance. Contenders may join for the **join window** (one week on mainnet) after
the first document; the contest runs for the **poll duration** (two weeks on mainnet). The first
document's owner may not be joined by the same identity twice.

The index's `contested.resolution` says how the contest is decided.

## Resolution 0: masternode vote

The DPNS rule. The choices are a contender, abstain, or **lock**, which gives the value to nobody.
The contender with the most votes wins unless the lock tally exceeds it, in which case the value is
locked and may be contested again later. The contest always runs the full poll duration, even with
a single contender, so the masternodes may lock the value.

## Resolution 1: masternode vote without locking

`ContestedIndexResolution::MasternodeVoteNoLocking`, meta-schema v3 (protocol version 14). The
choices are a contender or abstain. A Lock vote is refused with `VoteChoiceNotAllowedForVotePollError`
(40307). The contest always ends with a winner: the contender with the most votes, no minimum.

A contest without locking ends when its join window closes while it still has a single contender,
so that contender is awarded the value without a vote window. Its end-date entry is written at the
end of the join window when the contest opens; the first additional contender moves it to the full
poll duration, which opens the vote window. `getVotePollsByEndDate` shows whichever end applies.

The moderation charters contract uses this resolution to elect moderation teams.

## Ties

From protocol version 14, a tie among the top contenders goes to the **earliest** contender:
creation time, then block height, then core block height, then document id. This holds for both
resolutions; contests ending before version 14 awarded the latest contender.

## Storage

A contest's state lives under `votes / contested_resource / active_polls`, laid out like the
contested index it decides: the contenders' documents, one votes sum tree per contender, and the
abstain and lock tallies. The masternodes' vote references live under
`votes / contested_resource / identity_votes`, and the end dates under `votes / end_date_queries`.
Once the contest ends, the winning document is awarded, the losers are removed, and the stored
result stays for the `getContestedResourceVoteState` query.
