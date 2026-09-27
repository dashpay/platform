# Contested Documents

A unique index may be declared `contested`. A document whose index values fall in the contested
range is not stored outright: it opens or joins a **contest**, a `ContestedDocumentResourceVotePoll`
that names the contract, the document type, the index and the index values, and masternodes and
evonodes decide who gets the value. A masternode's vote counts once, an evonode's four times. Every
vote is a `MasternodeVote` state transition carrying the poll and a `ResourceVoteChoice`.

From protocol version 14, a vote towards an identity must name a contender of the poll. Any other
identity, including the reserved keys under which the poll keeps its stored result and its abstain
and lock tallies, is refused with `VoteChoiceNotAllowedForVotePollError` (40307). Before 14 such a
vote failed with an internal error, or counted as abstain or lock when it named a reserved key.

The contest is funded by the contenders' prefunded voting balances, and each vote costs a fixed
amount from that balance. Contenders may join for the **join window** (one week on mainnet) after
the first document; the contest runs for the **poll duration** (two weeks on mainnet). The first
document's owner may not be joined by the same identity twice.

From protocol version 14, a contest accepts at most 1,000 contenders
(`max_contenders_per_contest`): a document that would add one more is refused, paid, with
`DocumentContestMaximumContendersReachedError` (40141). The bound is what lets one block end a
contest: its end tallies up to `maximum_contenders_to_consider` contenders (10,000 from version 14)
and removes the entries of those it tallied, so a contest within the cap is tallied and cleaned up
whole. Before 14 a contest accepted any number of contenders and its end tallied at most 100; a
contest that grew past 10,000 contenders before 14 is tallied and cleaned up for its first 10,000
only.

From protocol version 14, the fund a contender pays doubles for every 100 contenders the contest
holds (`contested_document_contenders_per_fund_doubling`), so filling a contest to its cap costs
far more than the fund times its contenders:

| Contenders the contest holds | DPNS fund to join | Moderation election fund to join |
| --- | --- | --- |
| 0 to 99 | 0.1 Dash | 0.5 Dash |
| 100 to 199 | 0.2 Dash | 1 Dash |
| 200 to 299 | 0.4 Dash | 2 Dash |
| ... | doubles every 100 | doubles every 100 |
| 900 to 999 | 51.2 Dash | 256 Dash |

Filling a DPNS contest to 1,000 contenders costs 10,230 Dash (100 at a flat 0.1 Dash). A contender
states its fund in its prefunded voting balance, and one stating less than the fund of the contest
it joins is refused, paid, with `DocumentContestNotPaidForError`, which carries the fund it has to
pay. A contender may state more; everything it states goes to the contest's fund. Before 14 every
contender stated exactly the fund, however many had joined.

The index's `contested.resolution` says how the contest is decided.

From protocol version 14, an identifier property among the index values is written as an
identifier in the poll (`Index::extract_contested_values`), whether the document gave it as an
identifier, as 32 bytes or as an array of byte values. The index keys store all of these alike,
but a poll is hashed from its values, and that hash keys the contest's prefunded balance and end
date: two contenders writing the same identifier two ways would otherwise split one contest into
two polls. Before 14 the values are taken as given.

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

## Moderation elections

An `electedCharter` contest of the moderation charters contract (protocol version 14), keyed by
the target contract id, is a **moderation election** and does not take the generic parameters:

- Its join window and vote window are the `joinWindow` and `voteWindow` of the target contract's
  elected moderation declaration (one day to four weeks each, one week by default), on every
  network. A single applicant wins when the join window closes; a second applicant moves the end
  to the join window plus the vote window. A late applicant is refused with
  `DocumentContestNotJoinableError` naming the target's join window.
- Each application prefunds the votes with the moderation fund, 0.5 Dash
  (`moderation_vote_resolution_fund_required_amount`), instead of the contested document fund.

The target's declaration is read, and billed, when an application opens the contest and when a
later one joins it; it is frozen at the target's creation, so both reads agree. Nothing at the
end of a contest reads the target: the end date was written when the contest opened or was
joined. A target that is missing or declares something else leaves a contest on the generic
windows instead of failing, and the application's own reference validation refuses it. Every
other contest, DPNS included, keeps the generic windows and fund.

## Ties

From protocol version 14, a tie among the top contenders goes to the **earliest** contender:
creation time, then block height, then core block height, then document id, among every tied
contender. This holds for both resolutions; contests ending before version 14 awarded the latest
contender.

## Storage

A contest's state lives under `votes / contested_resource / active_polls`, laid out like the
contested index it decides: the contenders' documents, one votes sum tree per contender, and the
abstain and lock tallies. From protocol version 14 the tree below a contest's last index value,
which holds its contenders, stored result and tallies, is a count tree, so a join reads how many
contenders there are in one fetch; a contest started before 14 keeps its plain tree, and a join
counts its contenders by reading their keys. The masternodes' vote references live under
`votes / contested_resource / identity_votes`, and the end dates under `votes / end_date_queries`,
one tree per end date holding an entry for each contest ending then.
Once the contest ends, the winning document is awarded, the losers are removed, and the stored
result stays for the `getContestedResourceVoteState` query.

A block ends at most `maximum_vote_polls_to_process` contests (a drive-abci `event_constants`
value, two at protocol version 14), the earliest end date first, so contests due together may end
over several blocks. The cleanup removes each ended contest's
end-date entry, and removes an end date only once none of its contests remain under it; the rest
end in the next blocks.
