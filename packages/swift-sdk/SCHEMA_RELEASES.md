# App Store schema snapshots

SwiftData schemas become supported history when a build reaches App Store
distribution. TestFlight uploads capture provenance and a synthetic SQLite
fixture, but do not by themselves register a released schema. V1 is the agreed
existing baseline. Intermediate pre-release V2–V5 schemas have been collapsed
into the working V2, including the public-key usage-limit columns. A separate
compatibility bridge handles older, unregistered `1.0.0` database layouts when
they can migrate without losing existing data. Other intermediate development
databases are unsupported.

## Legacy stores before the release registry

Frozen V1 and its fixture remain unchanged. They describe the accepted baseline,
but do not establish the exact schema shipped by the first App Store binary.
Some older app sources created an unversioned `Schema(modelTypes)` and omitted
the explicit migration plan; those databases still report version `1.0.0`.

The shared `DashModelContainer` factory recognizes registered schema
fingerprints before opening an existing store. Unknown legacy `1.0.0` stores
may use the compatibility bridge: take a consistent backup including committed
WAL data, migrate an isolated copy automatically to `DashSchemaV2`, verify that
existing stored values and relationship rows survived, and reopen it through
the ordinary migration plan before installing it. The backup is retained for
recovery. Corruption, removed fields/entities, changed stored data, and newer
unknown schema versions are errors; this is not a general retry for any
container failure and never resets the store.

Applications with their own database paths must use
`DashModelContainer.create(url:)` before opening the store elsewhere. Direct
`ModelContainer` construction bypasses this compatibility bridge. The iOS host
uses the shared factory while retaining its existing store path and lifecycle.
The bridge is for local stores; CloudKit and in-memory containers continue to
use their ordinary migration plan.

Recovery files live beside the original store under
`<store filename>.legacy-v2-backups/`. A successful bridge retains an
`original.store` backup through that launch. A later successful ordinary open
reclaims completed backup directories; failed opens and pending migrations
never trigger cleanup. Cleanup failures do not prevent opening the wallet.
The `active.json` journal records an interrupted installation and a fingerprint
of the validated final data; the next open reconciles it before exposing a
container, even if scratch copies were removed. Older journals still require
their candidate when validating a committed installation. These
are local wallet data, protected like the original store and excluded from
device backup. Do not upload them as release fixtures or edit the recovery
journal to bypass a failure.

The historical regression fixture reconstructs Platform
`fd8d8d13e5d7cea17b00df5974934ab1910e8039` from the same checkout pair as iOS
`8094751eb2be8d52b57da3589fdd2ae2dcd0ecc6` in
[Actions run 32706880873](https://github.com/dashpay/dashwallet-ios/actions/runs/32706880873).
It contains synthetic records, not user data or an extracted App Store store.
The run failed before upload, so this provenance establishes a tested source
layout rather than proof of publication. Regression tests exercise the public
factory, data/default preservation, writes, reopen, and failure recovery.

Keep the bridge for installations that skip the V2 app release. When advancing
to V3, bind `DashSchemaV2` to its released snapshot and retain the legacy-to-V2
step before the normal V2-to-current plan. The bridge must never automatically
follow the latest live model graph. The release observer's one-time `bootstrap`
only records its observation baseline; it neither runs this migration nor
proves V1's App Store provenance.

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
   then run its monitor manually in dry-run mode. The baseline accepts V1 as
   agreed and does not claim to reconstruct an earlier binary.
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

A released snapshot has its own namespace, for example `DashSchemaSnapshotV2`.
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
  If the PR was closed without merge, reopen it deliberately before retrying.
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
python3 packages/swift-sdk/scripts/historical_schema_fixture.py --check
```

Swift SDK CI additionally checks the generated schemas against the released
fixtures, including entity hashes, indexes and migration behavior.
