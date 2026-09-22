# App Store schema snapshots

SwiftData schemas become supported history when a build reaches App Store
distribution. TestFlight uploads capture provenance and a synthetic SQLite
fixture, but do not by themselves register a released schema. The accepted frozen V1 remains unchanged. Historical V2 is now reconstructed
from `52e8d4ec68f0c772313fa1bbef223fb1eabbf1cc`; all 35 entity hashes and the
model checksum match the observed App Store 9.0.2 database. Active models are
V3. Other intermediate development shapes remain unsupported.

The old V2 fixture was generated from September 8 sources containing 13
properties added on August 28, after the August 27 App Store release. The
reconstructed V2 instead predates those additions. Its synthetic fixture and
source provenance live in `schema-releases.json` under `historical_schemas`,
separate from archive-captured releases. This identifies a matching model
source, not the confirmed build commit of Apple's binary. V2 is reserved:
automated release capture must not register another shape under that number.

The main migration plan is historical V2 → V3. Accepted V1 has a separate
V1 → V3 plan: V1 already contains the 13 properties missing from historical
V2, so a V1 → V2 → V3 chain could discard values. Routing uses model metadata
and runs after recovery; a version label alone never selects an unknown beta
schema. The former live V2 is accepted only when its complete graph matches
current V3 exactly, entity hashes and checksum alike. That alias therefore
lasts only until the next live-graph change: after it, every store still
labelled `2.0.0` with the former live shape becomes `unsupported-v2` and fails
closed. Check internal devices still carrying that label before the next
shape change and migrate or deliberately reset them then, rather than
discovering them afterwards as failed opens.

The route decision is logged as `store_migration_route` with the validated
`source_version`, `source_checksum` and one of `new-store`,
`accepted-v1-to-v3`, `historical-v2-to-v3`, `previous-live-v2-current-shape`,
`unsupported-v2`, `labelled-current-v3`, `ordinary-current-plan` or, from the
bridge, `legacy-v1-bridge-to-v3`. Resolving a frozen schema's identity builds
a temporary store, so it is memoized per process and consulted only where a
label is undecidable without it (`1.0.0`, `2.0.0`); a `3.0.0` label takes the
default plan without any probe, and a probe failure on the undecidable labels
refuses the open with the probe's own error while leaving the store untouched.

## Legacy stores before the release registry

Frozen V1 and its fixture remain unchanged. They describe the accepted baseline,
but do not establish the exact schema shipped by the first App Store binary.
Some older app sources created an unversioned `Schema(modelTypes)` and omitted
the explicit migration plan; those databases still report version `1.0.0`.

The shared `DashModelContainer` factory recognizes registered schema
fingerprints before opening an existing store. Unknown legacy `1.0.0` stores
may use the compatibility bridge: take a consistent backup including committed
WAL data, migrate an isolated copy automatically to `DashSchemaV3`, verify that
existing stored values and relationship rows survived, and reopen it through
the ordinary migration plan before installing it. The backup is retained for
recovery. Corruption, removed fields/entities, changed stored data, and newer
unknown schema versions are errors; this is not a general retry for any
container failure and never resets the store. Preservation is deliberately
strict: existing SQLite primary keys, foreign keys and join rows must survive
unchanged (entity ordinals are normalized by name). Even a semantically equivalent
Core Data migration that renumbers primary keys is rejected; such a layout
requires an explicit, separately validated migration rather than relaxing this
bridge's data-loss checks.

Before making full-size copies, the bridge acquires its exclusive store lock
and removes abandoned UUID attempt directories only when no migration journal
is pending. This also recovers space after a process was killed before writing
its journal. It then measures actual free space on the store's volume; failed
deletions are never counted as available space. A missing, nonpositive or failing
ImportantUsage capacity reading falls back to filesystem free bytes; if both
report zero, migration remains blocked. Its conservative estimate is four times
the combined main-file and WAL size, plus the larger of 64 MiB or half that
combined size. This budgets the two
copies, inferred-migration/promotion journals and schema/index growth. It is a
preflight estimate, not a reservation: other processes or larger-than-estimated
growth can still exhaust space, and SQLite errors remain fatal without replacing
the original. Insufficient headroom reports the needed and available space and
asks the user to free device storage and retry; it does not delete wallet data.

Applications with their own database paths must use
`DashModelContainer.create(url:)` or its async twin before opening the store
elsewhere. Both synchronous `create` overloads may block for seconds while
opening or migrating a large database and should not run on a UI actor. Direct
`ModelContainer` construction bypasses this compatibility bridge. The iOS host
uses `DashModelContainer.createAsync(url:)` while retaining its existing store
path and lifecycle. This async variant opens/migrates on a dedicated serial
queue and returns only the Sendable container; callers create and use contexts
on their owning actor. Coalesce concurrent opens of the same URL in the app.
The bridge is intentionally local-only: copying
and replacing SQLite does not establish preservation of CloudKit's synchronization
state. CloudKit and in-memory containers use their ordinary migration plan;
unknown CloudKit schemas require a separately supported migration.

Recovery files live beside the original store under
`<store filename>.legacy-v2-backups/`. A successful bridge retains an
`original.store` backup through that launch. A later successful ordinary open
reclaims completed backup directories. That optional cleanup takes the same
nonblocking store lock and rechecks the journal; lock contention skips cleanup
without failing an ordinary open. Known stores without attempt directories do
not create a bridge lock file. Failed opens and pending migrations never trigger
cleanup. Cleanup failures do not prevent opening the wallet.
Explicit wallet deletion has a stricter contract:
`PlatformWalletPersistenceHandler.deleteCompletedMigrationSnapshots()` removes
completed copies under the same store lock and propagates cleanup errors.
`deleteWalletData` and the manager's wallet deletion invoke it before deleting
SDK keys or live rows. Call it for a full-store wipe even when no wallet rows
remain, and do so for every affected network store. Pending recovery is preserved
and blocks deletion. Because a snapshot contains the whole historical database,
removing one wallet discards the whole completed snapshot, while unrelated live
wallets and other stores remain untouched. A cached container does not bypass
this deletion boundary. This is separate from best-effort startup cleanup.
The `active.json` journal records an interrupted installation and a fingerprint
of the validated final data; the next open reconciles it before exposing a
container, even if scratch copies were removed. Older journals still require
their candidate when validating a committed installation. These
are local wallet data, protected like the original store and excluded from
device backup. Do not upload them as release fixtures or edit the recovery
journal to bypass a failure. Clearing the journal is part of a successful open,
not optional cleanup: the factory has not yet returned the container to the app.
If clearing fails, the open fails and the next attempt validates recovery evidence
again. Ignoring that error could expose a writable container while leaving a
stale marker that rejects the app's legitimate later writes.

A missing primary database with a pending journal requires deliberate recovery.
The bridge never deletes or renames that primary file, so its absence is not a
normal interrupted-install state; it may be an intentional external reset. The
SDK preserves the journal and any copies and reports their location. It does
not silently restore `original.store` (which could be older than a committed
installation) or clear the marker and create an empty database. With the app
closed, restore the authoritative original from a verified backup, or use support
to identify an appropriate recovery source. If no source can be verified, retain
the evidence and stop; do not edit the journal to force startup.

The historical regression fixture reconstructs Platform
`fd8d8d13e5d7cea17b00df5974934ab1910e8039` from the same checkout pair as iOS
`8094751eb2be8d52b57da3589fdd2ae2dcd0ecc6` in
[Actions run 32706880873](https://github.com/dashpay/dashwallet-ios/actions/runs/32706880873).
It contains synthetic records, not user data or an extracted App Store store.
The run failed before upload, so this provenance establishes a tested source
layout rather than proof of publication. Regression tests exercise the public
factory, data/default preservation, writes, reopen, and failure recovery.

Keep the bridge for installations that skip the V3 app release. When advancing
to V4, bind `DashSchemaV3` to its released snapshot and retain the legacy-to-V3
step before the normal V3-to-current plan. The bridge must never automatically
follow the latest live model graph. The release observer's one-time `bootstrap`
only records its observation baseline; it neither runs this migration nor
proves V1's App Store provenance.

After publication, changing a runtime version's model shape in place can leave
existing App Store stores with no registered matching checksum and block startup.
`DashReleasedSchemaTests.testPublishedSnapshotsAndRuntimeVersionsMatchCapturedStores`
compares both the archival snapshot and the runtime version against the captured
published store; it fails on that drift. Keep the published version bound to its
snapshot, introduce a new live version and register the connecting migration.
The companion `testPublishedStoresMigrateAndRemainWritableThroughLiveTypes` then
checks that published stores still open and remain writable. No mutable live
fixture is needed for this publication boundary.

## Release flow

The iOS release workflow saves an immutable build manifest and content-addressed
fixture on `dashpay/dashwallet-ios`'s `schema-release-data` branch. Its publication
monitor polls App Store Connect twice daily, or on manual request, and matches
the published version to the exact Apple build. A publication proof refers to
that build's manifest and its digest. The manifest contains the full Platform
commit; the latest development commit is never substituted.
Versions marked `REPLACED_WITH_NEW_VERSION` also count as published history:
a release superseded between the twice-daily checks still requires its snapshot.

Before storing candidate evidence or uploading to TestFlight, iOS retains the
Platform commit under the lightweight tag `swift-schema-source/<full SHA>`.
This tag is source retention, not a schema freeze or a product release: it lets
the later freeze read that exact commit even if its development branch has been
force-updated during Apple review. The worker also verifies/creates these tags
for every registered source before publishing a snapshot PR. Existing tags must
point directly to their named commit; mismatches stop processing and are never
force-updated. Full-history Actions checkouts fetch these tags. A manual shallow
checkout must fetch the retained source tags before regenerating snapshots.

The monitor dispatches **Freeze SwiftData App Store release** with `release_id`
and `data_commit`. The worker verifies that this commit belongs to the metadata
branch, verifies the proof, build identity and artifact digests, and generates
the snapshot from the manifest's Platform commit. It uses a temporary clone;
the source checkout is unchanged.
The generator executable is copied from the reviewed `v4.2-dev` base into a
separate temporary directory before the draft is checked out. It runs with an
explicit target repository, isolated Python imports and a credential-free
environment. Draft files are input/output data, including any edits to the
generator itself; they are not executed by this worker. Only authenticated Git
fetch/push processes receive the PAT, with ambient Git configuration and hooks
disabled. Human changes on the draft remain available for review.

The worker opens a draft PR on `codex/freeze-swift-schema-v<schema version>`,
targeting `v4.2-dev`. The PR contains the generated snapshot, synthetic fixture
and release association in `schema-releases.json`. Standard Swift SDK checks
run on these same-repository draft branches. A maintainer must review and merge
the PR; the worker never enables auto-merge or updates runtime model types.
The iOS monitor considers the release handled only once its registry entry is
merged. New production-capable uploads must use a Platform commit containing
all required releases, not merely a commit from before the snapshot merge.

## Setup and manual operation

1. Deploy the generator, registry and worker to `v4.2-dev`, and register the
   dispatch workflow on the repository's default branch. GitHub requires the
   workflow to exist on the default branch for `workflow_dispatch`.
2. Configure `SCHEMA_RELEASE_TOKEN` in both repositories: a fine-grained PAT
   limited to `dashpay/platform` and `dashpay/dashwallet-ios`, with repository
   Contents, Actions and Pull requests permissions needed by the workflow.
   The Platform worker needs metadata read access, Platform branch/PR write
   access, and uses the PAT so its PR events trigger CI. Apple credentials stay
   in the iOS repository. Do not put tokens in command arguments or manifests.
3. Initialize the iOS release baseline before the first newly tracked release,
   then run its monitor manually in dry-run mode. The baseline must reference
   the verified historical V2 binding for 9.0.2. Correct the old V1 association
   without moving the publication cutoff; see the iOS runbook.
4. After publication, use the iOS manual monitor for the normal operator flow.
   For a worker retry, select the recorded Apple version ID and a full commit
   on `schema-release-data`. Select `dry_run` to validate and generate the patch
   without committing, creating source tags, pushing or creating a PR. Dry runs still require read
   credentials and query GitHub.
   Existing source tags are validated in dry runs too, including for already
   merged releases; missing tags are allowed and are not created.

Do not use the Platform workflow to bypass App Store publication: it requires
a published-state proof written by the trusted iOS monitor. Protect the data
branch against deletion/force-push and restrict write access to release operators
and automation. On PAT expiry, replace the repository secret in both repos and
retry; no schema should be regenerated manually just to recover authentication.
Protect `swift-schema-source/*` tags against update/deletion while allowing the
release automation to create new ones. These tags do not match this repository's
branch-only push workflows and do not trigger product release publishing.

## Developing the next schema

Keep `schema-models.json` complete for models and their stored value types.
Run `freeze_schema_models.py --check-inventory` before capture; the iOS capture
workflow does this automatically, and snapshot rendering validates the historical
source inventory again. Validation follows explicit fields and enum payloads,
including nested optional/array/set/dictionary values. Missing user-defined
types and unsupported storage declarations fail instead of silently binding to
live definitions. The validator uses a restricted Swift declaration grammar;
standard Swift/Foundation names are assumed not to be shadowed by application
types. It does not prove custom encoding or helper-method behavior, so native
captured-store hash/index checks and code review remain required.

A released snapshot has its own namespace, for example `DashSchemaSnapshotV3`.
It is not automatically registered alongside identical current models. When
changing the structure after a release, explicitly register the historical
snapshot as the old schema, introduce the next active version and its migration,
and retain the released fixture. A release with unchanged version, hashes and
indexes associates another App Store version with the existing snapshot.

If development changes while Apple reviews a build, freezing still uses the
uploaded commit. CI must expose any missing migration or incompatible current
models; reconcile the next version and migration in the draft PR before merging.
Never roll development back automatically and never change a released fixture
or snapshot to make a test pass. A changed structure under an already released
schema number is an error, including changes to SQLite indexes (which entity
hashes alone do not cover).

Fixtures contain synthetic records created by the release code on an arm64
simulator in Release configuration. They are not extracted from the device IPA
and never contain user wallet material.

## Retries and recovery

- Re-running the monitor or worker reuses the deterministic branch and open PR.
  The worker merges current development into the branch, preserves human edits,
  and pushes without force. Resolve merge conflicts manually before retrying.
- A push that succeeds before PR creation fails is recovered on the next run.
  If an unregistered release's PR was closed without merge, reopen it deliberately
  before retrying. Already-registered releases are validated against the merged
  registry first; a later rejected association for another release on the shared
  schema branch does not prevent their source-tag validation or repair.
- A missing build record, changed digest, conflicting schema number or
  rewritten release association stops processing. Restore the correct original
  record through the release-data recovery process; never guess a source SHA.
- If a retained source tag is missing, rerun the worker while the exact original
  commit remains available. It verifies all registry sources and recreates only
  missing references. If the commit has already been garbage-collected, recover
  that exact object from an original checkout/backup first; a replacement commit
  with similar source does not satisfy its identity. A conflicting tag requires
  investigation instead of an automatic overwrite.
- An unsupported development database may require an explicit app-data reset
  by its owner. Export/recover any needed development wallet first. The app does
  not silently erase an unrecognized database to make migrations succeed.

Run the automation tests with:

```sh
python3 -m unittest discover -s packages/swift-sdk/scripts -p 'test_*.py'
python3 packages/swift-sdk/scripts/freeze_schema_models.py --check
python3 packages/swift-sdk/scripts/freeze_schema_models.py --check-inventory
python3 packages/swift-sdk/scripts/historical_schema_fixture.py --check
```

Swift SDK CI additionally checks the generated schemas against the released
fixtures, including entity hashes, indexes and migration behavior.

## Local verification with a private database copy

`DashModelMigrationTests.testPrivateHistoricalStoreMigrationWhenExplicitlyProvided`
accepts a local standalone SQLite input through `DASH_PRIVATE_MIGRATION_STORE`.
For an Xcode simulator test run, forward it using the
`TEST_RUNNER_DASH_PRIVATE_MIGRATION_STORE` environment variable. The test copies
that input to a disposable directory, verifies preservation of every supported
SQLite table/relationship, writes through current models and reopens the copy.
Without the explicit input it skips; normal CI uses only synthetic fixtures.
Do not add a real wallet database to resources, XCTest attachments, commits or
CI artifacts. If capturing another input, preserve committed WAL contents with
a consistent SQLite backup rather than copying an active main file alone.
