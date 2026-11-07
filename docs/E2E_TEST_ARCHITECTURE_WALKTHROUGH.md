# One e2e test system: the plan in plain words

This is the short version of `docs/E2E_TEST_ARCHITECTURE_SPEC.md` (revision 3), which has the evidence and file references. Five independent reviewers checked the first draft, each from one angle: does it fit the real code and CI, does it lose coverage from the old suite, what does it cost in CI, can it be maintained across the SDKs, and is it too big. Revision 3 answers your question about the draft PR #3549 (below). The spec's last section lists every change.

It replaces the e2e part of the legacy-JS retirement plan. The rest of that plan still stands, as one track inside this one. None of these files is meant to be committed.

## Your question first

> The old test suite tested both Platform and the SDK. If its cases move into the new JS SDK's e2e suite, the SDK's e2e would carry Platform's functional coverage. Is that right?

**No, and this plan doesn't do it.** Every SDK we ship is a thin layer over the same Rust SDK:
- the JS SDK is the Rust SDK compiled to wasm;
- Swift and Kotlin call it through FFI, a C-level bridge from their language into the Rust code.

So Platform behaviour is tested **once**, in Rust, against a real network. Each SDK then only has to show that its own layer works, in two ways:
- **offline:** by reading recorded real responses correctly;
- **live:** with a small smoke test.

## Your second question: build on #3549 instead of from scratch?

> Its author probably already solved many real problems. Should its framework be the base rather than a fresh one?

**Mostly yes. Revision 3 changes the plan to do that.** I read its framework module by module, its 342 test commits and its test spec. Almost all of those commits are lklimek's; the PR was opened by an agent account acting for lklimek. The framework is 17.6k lines, in two parts.

**A small part that needs no wallet, about 2k lines.** It builds the SDK, picks the network, and waits for results with proofs. It already solves problems a fresh framework would run into again:
- one SDK shared by all tests dies with the first test unless all tests share one async runtime;
- the tests need an 8 MB stack, not the default 2 MB;
- a contract deployed during the run is unknown to the proof checker until it is registered;
- a rejected transition has to be checked after the block, with its exact error code, not just at mempool entry.

It also solves problems that show up on testnet, which matters once we run there:
- after a write, one node can have the result while the next request lands on a node that is still behind;
- a node one block behind gets banned for a minute;
- a client and a network on different protocol versions misread proofs.

**A large part that is a wallet test harness, about 14k lines:**
- the hand-funded bank wallet;
- moving money between Core and Platform;
- sweeping leftover funds back;
- SPV, persistence and shielded.

It is good work, but protocol tests on our local network don't need it. That network is thrown away after each run, and genesis already gives us funded identities.

**Three options:**
- **A. Fresh framework.** This is what revision 2 said. We would re-implement those fixes from its code without its months of runs, and later either port its wallet harness onto a different base or keep two frameworks.
- **B. Its wallet-free part becomes the base. Chosen.**
  - The new protocol crate starts by copying that part, credited to lklimek, with the internal ticket references in its comments replaced by plain explanations.
  - Soon after, #3549 rebases onto that copy and deletes its own, so the two can't drift apart.
  - When we do tokens, after the old suite is deleted, its token helpers and 17 token tests move over.
  - Its wallet harness and wallet tests land later as the wallet domains, owned by lklimek.
  - It needs nothing from the four open wallet PRs #3549 is stacked on.
- **C. Merge #3549 as it is and grow it.** No:
  - it covers none of what the old suite covers: documents, contracts, DashPay, withdrawals, DAPI Core;
  - its protocol tests run through the wallet, so a wallet bug fails a protocol test;
  - it has never run on our local network and has no way to fund itself there.

  Smaller points: it pulls in the four PRs it is stacked on, and our wallet CI would run its live tests and go red.

**Honest cost.** B saves little time before the old suite is deleted: about as much code gets adapted as would have been written, and local-network funding is new either way. What it saves is debugging, the token work, and most of the wallet work later.

**One thing I did not copy:** #3549 serialises funding with a lock. Our bank is a genesis identity, and the Rust SDK and Drive already handle concurrent identity nonces, so a lock would only slow every test down.

**It needs lklimek's agreement before work starts** (decision 5).

## The structure

| Level | What it proves | Runs against | Who owns it |
|---|---|---|---|
| **In-process** | The rules: consensus, fees, state, multi-block behaviour, upgrades. Also wallet logic that needs no network | No network | Whoever changes the rule |
| **Protocol e2e** (one Rust test crate) | A real network accepts and rejects transitions and proves results with quorum signatures. Core ↔ Platform (asset locks, withdrawals paid on Core), DAPI's Core endpoints and streams | A live network: local in CI; devnet or testnet on demand | Platform team |
| **SDK checks** | Each SDK reads real responses correctly (offline replay), and works end to end: transport, signing, broadcast, error codes (live smoke). **No protocol rules** | Recorded files; a live network | Each SDK's maintainers |
| **Node operations** | dashmate setup, start, stop, reset | Local; testnet | dashmate maintainers |

**Where a new test goes:**
- A rule goes in-process.
- Something that needs the real network goes to protocol e2e.
- A new query is one Rust SDK test: it runs live and records its file. The SDKs that replay files (the FFI layer and Swift) add a replay of it.
- A new transition gets in-process tests for its rules, one protocol-e2e happy path, and a unit test in each SDK that exposes it.
- A change to a binding (the thin per-language layer over the Rust SDK) gets that binding's unit tests and, where the SDK replays files, a replay.

**Tiers.** Every protocol-e2e test is *smoke* or *full*, and belongs to a *domain*: identities, contracts, documents, DPNS, DashPay, withdrawals, DAPI Core, and later tokens, groups, voting, addresses, wallet and shielded.
- **Smoke** is about a dozen cases, under 10 minutes. It runs on PRs that can change network behaviour.
- **Full** runs nightly, by hand, and before releases.
- **Choosing** is a name filter: `::smoke::` for smoke, `tokens::` for one domain.
- The SDK checks have no tiers. They are small by design.

**Network.** Every live suite reads the same environment variables (`DASH_E2E_…`): node addresses, the quorum-key source, Core RPC access, and the "bank" identity.
- **Local network:** our existing setup scripts write them, during the step whose result CI already caches.
- **Devnet or testnet:** CI inputs and secrets set them.
- There is no new settings file and no new script.

**Money.** One model everywhere:
- **The bank.** On our local network, the three test identities that genesis already creates with known keys. On a devnet or testnet, a funded identity stored as a secret.
- **Funding a test.** Move credits from the bank to a fresh Platform address, then create identities from it. Every SDK already has both calls, and none of this touches Core.
- **Core.** Of the new tests, only the Rust e2e touches Core. Swift's existing live tests keep their own Core funding:
  - a key minted during local setup pays for asset locks;
  - the InstantSend lock arrives through the Rust SDK's own wait. InstantSend already works on the local network; the Swift tests rely on it;
  - withdrawals check the payout on Core.

## What we have today (the short inventory)

- **In-process tests:** about 4,800 in Drive. On PRs only one big multi-block "strategy" test and 5 short ones run; the full set runs after merge and nightly.
- **The old JS test suite:** 95 network cases plus 5 unit, in Node and Chrome. About 47 runner-minutes per run.
- **The new JS SDK's functional suite:** 130 tests in Node and Chrome against our CI network, which it has hardcoded.
- **The Rust SDK's recorded tests:** 130 read-only tests replaying 108 recorded directories. Swift and the FFI layer already replay some of the same files through Rust. But:
  - none of the recordings come from the current protocol (15); 87 of 163 come from protocols 4 to 12;
  - 75 of 188 file names don't match their contents, a sign of hand edits;
  - CI never re-records them, and never checks them against a live network;
  - re-recording needs hand edits in four places.
- **Swift:** about 860 unit tests in CI. 15 live tests against a local network, mostly wallet and Core, which no CI job runs.
- **Kotlin:** unit and emulator tests on PRs. A nightly with 3 read-only testnet queries.
- **dashmate:** 3 lifecycle tests.
- **The draft Rust e2e PR (#3549):**
  - About 66,000 lines, about 21,000 of them from four other open PRs it is stacked on.
  - Strong on wallet, addresses, tokens and shielded; nothing on documents, contracts, DashPay or withdrawals.
  - Testnet-first and funded by hand. It has never run in CI or on our local network.
  - About 2,000 of its 17,600 framework lines need no wallet. They become the base of the new protocol suite (see above).

**A real gap.** A PR that changes only Rust consensus code runs **no network test at all** before merge. The trigger for the Docker images and e2e jobs lists no Rust crates.

## Where the old suite's cases go

| Old area | New home |
|---|---|
| Identities (19): asset locks, top-ups, transfers, keys, masternode identities | Protocol e2e, identities. The masternode case reads the masternode list, so it needs no secret and runs on any network |
| Contracts (7), documents (8), index-only documents (17) | Protocol e2e, contracts/documents. 8 cases are dropped: 6 rules already tested in Drive (each drop names the test), 1 rule that first gets a test at the latest version, and 1 old-SDK error |
| DashPay contacts (10), DPNS (8) | Protocol e2e, dashpay/dpns |
| Withdrawals (6) | Protocol e2e, withdrawals (5), now also checking the payout on Core. The "wrong key type" case is dropped in favour of a new Drive test. "More than balance" only ever hit the old SDK's own check; it becomes a real Platform rejection |
| DAPI Core endpoints and the header stream (8) | Protocol e2e, DAPI Core. **Plus two new stream tests** (transactions, masternode list), because the old wallet test was their only coverage |
| Epoch, protocol version, status (4); DPNS contract (1) | Already in the Rust SDK's recorded tests |
| Proof "shape" checks (3), wallet-lib sync (5), plus the 5 unit tests of the old proof adapter | Dropped. They test the old libraries |

About 73 cases are ported, 5 are already covered, about 17 are dropped with a reason, and 2 stream tests are new.

**What the reviewers caught:**
- Three "already covered by Drive" claims were wrong:
  - document of an unknown type;
  - an index-only like on a missing post;
  - one withdrawal case that never reached Platform.
  
  Those are now kept, or replaced with a real test.
- One rule had no test anywhere (a withdrawal signed with the wrong key type), and one was tested only at older versions (a contract-schema position rule). A small PR adds both tests before anything is dropped.

**One deliberate reduction.** Protocol flows no longer run in a browser. What we keep in Chrome is the JS SDK's 130 functional tests, writes included. What we lose in the browser:
- the old protocol flows;
- browser streaming of the transaction and header streams;
- creating an identity from a Core asset lock in the browser.

## Order of work

1. **Now, if you agree to the first decision below:** consensus PRs start running a network smoke. That means building the three Docker images, reused when only the Rust SDK changed. Until the Rust suite exists, that smoke is the JS SDK's functional suite. It gets its own trigger, so the old suite doesn't start running on every Rust PR. A legacy-package failure no longer skips it.
2. **The Rust e2e crate.** Its framework starts from the wallet-free part of #3549's. Then it adds what our local network needs and #3549 never had: the bank, a Core faucet and asset locks. The step also brings the first smoke set, a CI job and a short book chapter. lklimek agrees first (decision 5).
3. **Port the old suite** in four PRs:
   1. identities;
   2. withdrawals and DAPI Core;
   3. contracts and documents;
   4. DPNS and DashPay.

   Plus the small PR that adds the two missing Drive tests. **Then delete the old suite.**
4. **In parallel, fix the recordings.**
   - Record them in CI (or locally) through one script. A manifest file holds the values that are hand-pinned today.
   - Add guards against hand edits, against orphan files, and against recordings older than the previous protocol version.
   - Run the read tests live every night.
   - Later, add test data to genesis so contested names and groups can be recorded too.
   - Swift gets a fuller replay table.
5. **Align the SDK smokes.**
   - The JS suite reads the same variables and adds the three missing smoke steps: fund an address from the bank, create an identity from it, check an error code.
   - Swift's live tests run nightly on the self-hosted Mac runner, if it can run Docker. A catch: our test images are built only for Intel/AMD Linux, and that runner is an ARM Mac. It needs ARM images or emulation.
6. **After deletion,** in order of value:
   - tokens, groups and voting, starting from #3549's token helpers and 17 token tests;
   - runs against a devnet or testnet;
   - the wallet, addresses and shielded domains. #3549's wallet harness and wallet tests, already rebased onto the same base, land as lklimek's PRs and replace #3549.

**Effort:** about 5.25 to 6.75 engineer-weeks to delete the old suite. A fresh framework would be about 5.5 to 7. The previous plan (port into the JS SDK) was 4 to 6. The difference buys:
- one protocol suite that no single SDK owns;
- a Core path with no Core 24 parsing problem, because Rust already handles it.

Recordings, remote runs and the wallet domains are separate work, listed separately.

## Where this differs from the earlier plan and from your framing

- **The port target changed.** The old suite's cases go to Rust, not into the JS SDK's suite. The JS suite stays as the JS smoke and gets no new protocol cases. Its few existing protocol-flavoured cases stay until Rust has them.
- **The JS SDK no longer needs an asset-lock builder for tests.** It becomes an optional product feature. Its two skipped JS tests become offline unit tests on a fixed asset lock.
- **The JS Core faucet, browser middleware and Node gRPC client** from the earlier plan are dropped.
- **#3549's framework is now the base, not just a source of ideas.** Its wallet-free part starts the new crate, and its wallet harness lands later on the same base. It is still not merged as is, because it doesn't cover what the old suite covers and its protocol tests run through the wallet.
- **"Real e2e and/or fixtures":** both, but only *read* requests are recorded. Recording writes would churn every file and would need a fake clock and fixed randomness in the SDK.
- **"Replay the same files in every SDK":** possible only through the Rust layer each SDK wraps. Swift and the FFI layer do it today, and Swift gets more. For JS it is bigger than it looks, and it overlaps the live JS suite, so I propose later. Kotlin is not planned.
- **"The same network selection everywhere":** the same variable names in every suite, written by the setup we already have, instead of a new settings file.
- **Testnet and devnet runs come back after deletion,** for Platform-only cases with a funded identity as a secret. Core-side runs on testnet need the draft PR's "bank with sweep-back" (a long-lived funded wallet that takes leftover test funds back), which arrives with its wallet domains.
- **The legacy JS retirement is otherwise unchanged:** the dashcore-lib fix, deleting the JS DAPI and bench suite, dashmate's cleanup, moving the contract tests to Rust, stopping publishing, deleting the packages, and npm deprecation.

## Decisions I need from you

1. **Should every PR that changes Rust consensus or SDK code run a network smoke test?** Each such PR would build the three Docker images and start a local network. Today none of these PRs run any network test before merge.
   - **Who it hits:** about a third of merged PRs (37%) would newly trigger it, about 135 CI runs a week.
   - **Cost:**
     - roughly 4,000 to 7,000 extra runner-minutes a week once the old suite is gone, and 9,000 to 12,000 a week in the weeks before, while both run. That is free on a public repo, but it means queueing;
     - those PRs wait about 28 to 35 minutes for this check;
     - about twice as many image pushes to AWS.
   - **Two follow-ups for you:** whether the check should block merging (today only "PR Hygiene" does), and whether a merge queue is worth it. A merge queue would cut the runs about five times.

   I recommend yes, through its own trigger, reusing images when only the Rust SDK changed, and not blocking merges at first.
2. **Is it OK that protocol flows stop running in a browser?** We keep the JS SDK's 130 functional tests in Chrome, writes included. We lose three things in the browser: the old protocol flows, browser streaming of transactions and headers, and creating an identity from a Core asset lock. I recommend accepting it: the protocol code in the browser is the same Rust code, and the browser-specific transport stays covered.
3. **Is it OK that, when the old suite is deleted, the new suite runs only on our local network for a while?** Runs against a devnet or testnet come back later (after tokens, groups and voting), for Platform-only cases, using a funded identity kept as a secret. Core-side runs on testnet need the draft PR's bank with sweep-back, which arrives with its wallet domains. The old suite's remote mode isn't used by CI today. I recommend accepting it.
4. **Should JS and Kotlin also replay the recorded files?** It is possible only through the Rust layer. Swift and the FFI layer already do it, and Swift gets more. For JS it is more work than it looks, and it mostly repeats what the live JS tests already check. Kotlin would need a small addition to its Java-to-Rust bridge (JNI). I recommend JS later, only if a bug in how the JS layer converts responses slips past the live tests, and no Kotlin replay.
5. **Build on #3549 (option B above)?** I recommend yes. lklimek would be asked to agree, before work starts, to:
   - our copy of its wallet-free part becoming the maintained one, with lklimek reviewing that PR and credited in it;
   - rebasing #3549 onto it soon after, and deleting #3549's own copy;
   - owning the wallet harness and wallet tests, while the protocol-level tests sit with the Platform team;
   - the clean-up: test names, comments without internal ticket references, and the shared variable names.

   lklimek also picks where the wallet part lives, by the time the first PR merges:
   - **in the same new crate,** behind a switch that is off by default. I recommend this: one runner and one CI job, and our wallet CI can't pick it up by accident;
   - **or in the wallet crate,** with the shared part split into a small crate of its own.

   If lklimek says no, you choose between copying with credit anyway and writing a fresh framework.

**Already decided elsewhere.** If Core 24 reaches our CI before the Rust suite covers the old cases, the old suite stays alive through the dashcore-lib fix. That is the first decision of the retirement plan, and this plan assumes it.
