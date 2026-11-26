# Fixtures

Most test plan rows need something on chain before they can run: a counterparty with a name, a token the tester may mint, a pending action to co-sign, a poll that has already ended. A row without its fixture does not fail, it is `BLOCKED`. This file says how to map rows to fixtures and how to put them on chain.

## The map: `$QA_HOME/fixtures.md`

Build it from section 2 ("Prerequisites & fixtures") and the prerequisites column of each `TEST_PLAN.md`. One line per group of rows:

| Row ids (Swift / Kotlin) | Needed on chain | Available at | Fixture |
|---|---|---|---|
| TOK-05..14 | A token where the tester holds every admin right, and a target with a balance | protocol version 14 | Contract `qa-tok-admin`, one token per tester wallet, id `<id>` |
| TOK-16 | A group action waiting for the tester's signature | 14 | Contract `qa-tok-group`, one pending mint per tester, action ids `<ids>` |
| VOTE-08 | A decision proposal | 15 only | Seeded right after activation; a finished one exists `<date>` at the earliest |

"Available at" comes from the code, not from memory: `packages/rs-platform-version/src/version/feature_initial_protocol_versions.rs` and the version tables beside it. A feature the network's protocol version does not have yet is seeded after activation, and its rows wait until then.

Rules the map follows:

- **One fixture per tester for anything single use.** A pause blocks everyone using that token, a pending action is signed once, a fee pot is claimed once per epoch. Four testers need four.
- **Human testers' wallets only receive.** Seeding sends to them. It signs for them only where a row cannot exist otherwise, and those writes are listed.
- **Start the slow ones first.** A contested name takes its full voting period, a poll its minimum length, balance history needs entries spread over hours, a fee pot needs an earlier epoch. Seed these before anything else and note when each becomes usable.
- **Keep summed and counted values small**, well inside the limits of the protocol version the network will move to, so a contract seeded before an upgrade still loads after it.

## Putting fixtures on chain

Seeding is operations work: it changes no file in the repository and opens no pull request. A bug found while seeding is a ledger item like any other.

- **SDK matches the network.** Seed with an SDK of the network's own protocol version. A newer SDK can write what the network refuses or fail to read the proof of what it accepted.
- **Keys before use.** Every generated key or recovery phrase is written to `$QA_HOME/secrets.json` (mode 0600) before its first use, and is never printed, reported or committed.
- **Areas.** Split the work by which identities sign: tokens, moderation, documents and names, credits and addresses, the shielded pool. One agent per area, each with its own identities, its own state file (`state.<area>.json`), log and secrets file, merged by the coordinator at the end. No identity ever has two processes broadcasting from it.
- **Shared limits.** A faucet's hourly limit is shared by everyone on the network. Give each area a share and have it log each request.
- **Done means read back.** Every created id is recorded in the area's state file and confirmed by a proved read after the broadcast. The area's report lists what was seeded with ids and counts, what failed with the exact error, and what it changed in shared helpers.
- **Rerun, do not rewrite.** After a network reset, rerun the same scripts with the same keys so wallets and identity ids stay the same. Fields in old state files describe the old chain and are not trusted as chain state.
- **Archive, never delete.** Before a new run, move the old run's state, logs and snapshots to `$QA_HOME/archive-<date>/`.

## When the network will be upgraded

To test an upgrade, seed at the old protocol version, snapshot every seeded object with proved reads, upgrade, snapshot again and compare. The `protocol-upgrade-test` skill covers the upgrade itself. Rows that must be refused before activation (a feature "not available yet") can only be run before it: schedule them first.
