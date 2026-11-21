# Retiring legacy JS: the plan in plain words

This is the short version of `docs/LEGACY_JS_RETIREMENT_SPEC.md` (revision 2.1). The spec has the evidence and details. Five independent reviewers checked it: feasibility, scope, CI and release, dashmate, and external users. Their corrections are folded in; the spec's last section lists them. Neither file is meant to be committed.

> **Superseded in part (2026-10-29)** by `docs/E2E_TEST_ARCHITECTURE_WALKTHROUGH.md`. These parts no longer hold:
> - **"The plan for the e2e suite",** including "How the ported tests get money without wallet-lib": the old suite's protocol cases move to one Rust e2e suite, not into the new JS SDK's suite. The JS SDK no longer needs an asset-lock builder for tests.
> - **The last bullet of finding 4.** The JS SDK's suite stays as the JS smoke and gets no new protocol cases.
> - **Step 5 of "Order of work".** The port now goes to the Rust suite.
> - **In "Where this differs":** the bullets on not rewriting the old suite, on the asset-lock builder, and on the Rust work continuing separately. The Rust work is now the main track.
> - **Decision 4** (runs against testnet or a devnet). It is replaced by the new walkthrough's question about remote runs.
>
> Everything else here still holds.

## What we are removing

- the old SDK (`dash`);
- wallet-lib;
- dapi-client;
- wasm-dpp;
- the JS header-chain library (`dash-spv`);
- grpc-common;
- the npm package of dapi-grpc, with its generated clients for JavaScript, Java, Objective-C and Python. The Rust crate and the proto files stay;
- the old JS DAPI server;
- the bench suite.

All of it sits on dashcore-lib 0.22. The replacement, evo-sdk on top of wasm-sdk and wasm-dpp2, already exists.

## What we found

1. **The old JS DAPI server is already dead.** Dashmate stopped running it at version 3.0, and its Docker image hasn't been built for a year. CI still tests it on most rs-dpp pull requests. It can go today, with the bench suite, which no CI job runs.

2. **Dashmate barely uses the legacy stack, and the one thing it uses has a bug.**
   - Its wallet-lib code is never called.
   - dapi-client is used for one "is the node ready" check. That check always talks plain, unencrypted gRPC to the node's gateway. On nodes with real certificates, which is the default for testnet and mainnet evonodes, the gateway only speaks TLS, so `dashmate start -w` would wait forever. Two reviewers found this by reading the code; nobody has run it.
   - dashcore-lib makes a few random keys and builds two funding transactions during local setup. Dashmate's Core RPC client can't send the needed calls correctly, so we add a tiny JSON-RPC helper instead.

3. **The contract packages' JS tests check old schemas.** There are 228 of them, and they are the only reason those packages need wasm-dpp. They validate documents against version 1 schemas, while the network uses newer ones for DPNS, DashPay, withdrawals and token history. CI doesn't run them when the Rust code they test changes. That is how 14 of them sat broken until a fix on Oct 28. They call a Rust function that Rust tests can call directly. So we move them to Rust and test the current schemas.

4. **The e2e test suite is the real work.**
   - Every write it makes goes through the old SDK, and every identity it funds goes through wallet-lib and dashcore-lib.
   - About 75 cases test real protocol behaviour and are worth keeping. About a dozen test the old libraries themselves and can be dropped. A few more repeat rules that Drive's own Rust tests already check, and may be dropped case by case.
   - The new SDK already has its own end-to-end suite running in CI. It covers a good part of the same ground in Node and in the browser, using test identities our CI network creates at startup.

5. **There is a deadline: Dash Core 24.** dashcore-lib 0.22 can't read Core 24's new coinbase format.
   - That already broke the old JS client's node discovery on testnet on Oct 23.
   - The whole JS e2e suite breaks the day our local test network moves to Core 24. That comes before Platform's new withdrawal format (v2 asset unlocks) even matters.
   - Fixing the withdrawal format properly needs two changes, not one: accept the new version number, and compute the new transaction ID.
   - **Outside our repo:** the block explorer Insight uses the same library. It will choke on the new withdrawal format too, once that goes live with Core 24. Whoever releases Insight has to know.

## The plan for the e2e suite

I compared three options:

- **A. Port the tests to the new JS SDK.** Keeps browser coverage and tests the JavaScript API people install. A browser-only transport bug was fixed on Oct 27 (#5611), which shows why that matters. Roughly 4 to 6 engineer-weeks.
- **B. Rewrite them in Rust.** Loses browser and JS coverage. It would build on an unmerged draft of about 66,000 lines (the platform-wallet e2e framework). Roughly 5 to 8 weeks plus landing that draft.
- **C. Both.** Best coverage, two harnesses.

**I recommend A, with one twist.** Instead of rewriting the old suite package, we move the cases into the new SDK's existing end-to-end suite and then delete the old package. One JS test harness, not two. The Rust work (B) can continue on its own as the wallet-focused suite; nothing here waits for it.

### How the ported tests get money without wallet-lib

- **Most tests don't touch Core.**
  - Our CI network starts with three test identities that hold plenty of credits and whose keys we know.
  - Each test runner moves some credits from one of them to a Platform address it controls, then creates new identities from that address as needed.
  - All of this goes through the new SDK, in Node and in the browser.
- **Tests that must touch Core use a Node helper** that talks to the local Core node's RPC. These are asset-lock errors, top-ups from Core, withdrawals and Core endpoints. The helper finds coins, broadcasts, and fetches InstantSend locks.
  - Browser tests reach the same helper through the test runner.
  - Each runner gets its own funding key, so they never fight over the same coins.
  - As a bonus, we'll finally test InstantSend asset-lock proofs. Today's suite never does, because wallet-lib ignores InstantSend locks.
- **The SDK gets exactly one new feature: "build an asset-lock transaction from these coins and keys".**
  - It reuses existing Rust code (key-wallet's transaction builder); the bindings only pass data through.
  - It also un-skips the new SDK's own "create identity" tests.
  - "Wait for the lock" is not part of it. The Rust version of that only works natively today, and the tests get locks from Core directly. That stays an optional later feature.
- **Tests of DAPI's Core endpoints**, including the block-header stream, use a plain Node gRPC client on our proto files. No SDK changes are needed for those.

## Can we have an SPV wallet in JS?

I compiled the Rust crates for wasm, and a reviewer re-ran the check:

- **The wallet parts work.** key-wallet (keys, derivation, transaction building) and key-wallet-manager (wallet state) compile. key-wallet already ships inside wasm-sdk.
- **The sync part does not.** dash-spv needs raw TCP sockets, which browsers don't have. platform-wallet fails only because it always pulls dash-spv in.
- **A DAPI-based sync is possible, but a project of its own** (about 8 to 12 weeks). platform-wallet would get a second sync engine that follows DAPI's streams, the way wallet-lib syncs today. That needs:
  - making dash-spv optional;
  - a new client for the block-header stream;
  - one upstream change so the wallet can take filtered blocks;
  - a way to check InstantSend and ChainLock signatures without dash-spv's masternode-list engine;
  - browser storage and bindings.
- **What it can't fix.** Its trust is the same as wallet-lib's: the DAPI node learns your addresses and could hide transactions.
- **The test suite needs none of this.**

## Order of work

1. **Now:**
   - one last dashcore-lib release that understands Core 24 and the new withdrawal format;
   - Platform switches to it. This draft pull request already exists, and it also refreshes the stale list of testnet nodes;
   - a temporary patch only if Core 24 reaches our CI first.
2. **Delete what nothing needs:** the JS DAPI server and the bench suite.
3. **Dashmate, in three small steps:**
   1. drop wallet-lib and dapi-client and fix the readiness check. This one goes to the current release line too, so operators get the fix;
   2. drop dashcore-lib;
   3. stop building dashmate's Docker image on top of the heavy wasm build. That saves about 20 minutes per release per architecture.
4. **Contract schema tests move to Rust.**
5. **The new SDK gets the asset-lock builder.** The test cases then move into its end-to-end suite in a few batches, starting with withdrawals and identities. When the last batch lands, the old test-suite package, its CI job and its Docker image are deleted.
6. **Delete the legacy packages,** with their CI jobs, Docker steps and docs.
7. **Rewrite two book chapters** whose examples use wasm-dpp code.
8. **Stop publishing, then deprecate on npm.** Once a stable dashmate no longer needs them and the final legacy release is out, we stop publishing the legacy packages from every branch, while the old test suite keeps running until the port is done. Then deprecate on npm, only with your go-ahead.

## Who uses these packages outside this repo

- **`dash` (the old SDK):** a handful of active third-party projects, plus many accidental installs of an unrelated old package with the same name. evo-sdk already has twice its monthly downloads.
- **wallet-lib and dapi-client:** mostly our own organisation's repos.
  - dash-network-deploy (smoke tests);
  - dash-bridge. It already builds its asset-lock transactions itself and uses dapi-client to wait for InstantSend locks and read the masternode list;
  - the private website repo, which reaches into the old SDK's internal files.
- **wasm-dpp, dash-spv, grpc-common, dapi-grpc and the contract packages:** no real outside users found.
- **dashcore-lib:** widely used outside Platform, for example by xchainjs, Fireblocks' recovery tool and Insight. So we stop depending on it, but we don't deprecate it.
- **Docs:** the developer-docs tutorials already moved to evo-sdk in March. A few reference pages still show dapi-client examples.

**One catch: dashmate itself still lists wallet-lib, dapi-client and dashcore-lib.** If we deprecated them on npm today, every dashmate install would print warnings. So a stable dashmate without them has to ship first.

## What external users get, and when

- **In the next two weeks:**
  - notices in the legacy READMEs, with full links (the current one is broken on npmjs.com);
  - a "Deprecations" section in the release notes;
  - a pinned GitHub Discussion;
  - banners on the old docs pages.
  - Also, add the missing license and repository fields to evo-sdk, wasm-sdk and wasm-dpp2. Today npm shows no license for the package we're sending people to.
- **A final version.** The last stable release that still contains these packages, with the Core 24 fixes. We don't fix anything else in it; known problems are listed in the READMEs.
- **A migration guide** in the developer docs, from the old SDK to evo-sdk. It includes honest "no equivalent yet" rows: sending DASH from an HD wallet, balances, the masternode list.
- **`npm deprecate`, with 30 days' notice,** once two things are true:
  - a stable dashmate without the legacy packages is out;
  - the legacy packages no longer publish from any branch.

  It marks only existing versions, which is why publishing must stop first. Nothing is ever unpublished.
- **Total: roughly two to three months.** This doesn't wait for the test port, because we stop publishing before the old packages leave the repo.

**Security note.** One of dashcore-lib's crypto dependencies (elliptic) has a known issue with no fixed version, so the only fix is to stop using it, which this plan does. Another warning (bn.js) is caused by a version pin in our own root `package.json`, not by dashcore-lib. That one is a small separate fix.

## Where this differs from your original idea

- **We don't rewrite the old test-suite package.** The cases move into the new SDK's existing end-to-end suite, and the old package is deleted. That avoids two parallel JS harnesses. What we lose: the old package could also run against testnet or a devnet; the new home is local-only for now (decision 4 below).
- **The new SDK gets one small feature (the asset-lock builder),** because the tests need to create identities from Core funds. It is not the broader Core feature set I first proposed; reviewers showed the tests don't need that.
- **dashcore-lib gets one more upstream release, despite the retirement.** The final legacy versions need it to work on Core 24, and our CI needs it until the port is done. Platform takes it from the library's main line, matching the draft PR that already exists. A separate backport to the old 0.22 line is for Insight, not for us.
- **More is removed than just the JS packages.** Also the dapi-grpc npm package with its generated clients for other languages, the test-suite Docker image (its only outside user has been inactive since 2024), and the contract packages' npm releases. Their Rust crates stay.
- **The contract tests are rewritten in Rust against today's schemas,** not ported case by case, and not moved to wasm-dpp2. wasm-dpp2 can't validate a document against its schema.
- **The Rust e2e work (option B) is not dropped.** It continues on its own track.

## Decisions I need from you

1. **Fix dashcore-lib one last time?** That means one upstream release, which Platform then switches to. The alternative is to declare the old packages broken on Core 24 and keep our CI on Core 23 until the port is done. I recommend the fix: it's small next to the port. Insight needs a similar patch on the library's older 0.22 line anyway, so I'd ask the library's maintainers for that backport in the same go.
2. **Add "wait for the InstantSend lock" to evo-sdk?** The tests don't need it. It is what apps like dash-bridge would need to stop using dapi-client. I recommend doing it later as a product feature.
3. **Which release is the last one with the legacy packages?**
   - The last 5.0.x stable. That is fastest. The old packages will then probably stop working when networks move to protocol 15 (not verified).
   - The first 5.1.x stable. Then the old packages must keep working for the whole 5.1 cycle, and both "stop publishing" and the final deletion wait until 5.1.0 is released.

   I recommend the 5.0.x option, saying it clearly in the deprecation message.
4. **Do we need e2e runs against testnet or a devnet?** The old suite supports that; its new home doesn't yet. I recommend dropping it for now and adding it later only if someone needs it.
