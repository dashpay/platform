# Derived image defaults for dashmate configs

Status: spec v2. Spike completed, design reviewed, not yet implemented.

v2 replaces the v1 approach of resolving inside `Config.get()` for one hard-coded
leaf path. That version returned different answers to `config get <path>` and
`dashmate config --format=json`, which was correctly rejected in review.

## Problem

dashmate config files store **resolved** values. Two of the ten images in the base
config are not authored values — they are derived from the package version in
`configs/defaults/getBaseConfigFactory.js`:

```js
const prereleaseTag = semver.prerelease(version) === null ? '' : `-${semver.prerelease(version)[0]}`;
const dockerImageVersion = `${semver.major(version)}${prereleaseTag}`;   // "4" stable, "4-rc" on an rc

image: `dashpay/rs-dapi:${dockerImageVersion}`,
image: `dashpay/drive:${dockerImageVersion}`,
```

Because the derived value is copied into every operator's config at install time, it
goes stale the moment the package version crosses a boundary — a major bump, or
stable ↔ prerelease. Nothing prompts anyone: the version bump alone changes what the
correct value is.

**53 of 73 migrations exist to re-pin an image.** For the eight hand-pinned images
(core, tenderdash, envoy, …) that is appropriate — a human edits a literal, and that
edit is the prompt. For the two derived ones there is no prompt, which is how a
release shipped with no migration moving operators onto the `4-rc` line.

The deeper problem is that a re-pinning migration must not overwrite an image the
operator chose, but both are plain strings in one field, so the only available test is
a heuristic on the string. A previous guard (`4(-[a-z]+)?`) silently overwrote
operator-built images such as `dashpay/drive:4-local`. No regex can fix this: a
locally built image tagged into the `dashpay/` namespace is indistinguishable by shape
from a published one. **The information needed — did the operator choose this? — is
not recorded.**

## Approach: record intent, resolve on read

A config stores `null` for the two derived images, meaning *"use the image line this
dashmate build ships"*. An explicit string means the operator pinned it, and nothing
ever overwrites it.

Reading is **effective by default**. Every ordinary read returns the resolved value;
stored intent comes back only through explicitly named accessors used by a short,
enforced allowlist.

That ordering is the point. With raw as the default, every present and future display
surface must remember to resolve, and v1 demonstrated how easily that is missed. With
effective as the default, new code is correct automatically and exposing `null`
requires deliberate effort — the failure mode becomes loud instead of silent.

### Why not the alternatives

**Shared helper, keep per-release migrations.** Saves a few lines; does not remove the
chore, and does not fix intent — an operator deliberately on a stock tag is still
moved. Migrations are frozen historical artifacts; a shared helper is live code, so
editing it retroactively changes what shipped migrations do.

**Normalise derived images on every config load.** Removes the chore but runs forever
rather than once, rewrites deliberate pins on every command, and leaves no audit
trail.

**Concrete image plus an `imageAutoUpdate` flag.** Rejected on evidence, not taste.
It creates two sources of truth that can disagree, and out-of-band writers are not
hypothetical — `.github/actions/local-network/action.yaml:85` rewrites images
directly in `~/.dashmate/config.json`:

```bash
sed -i -E "s/dashpay\/(drive|rs-dapi):[^\"]+/${image_org}\/\1:${SHA_TAG}/g" ~/.dashmate/config.json
```

With an untouched tracking flag, the next dashmate command silently reverts those
SHA-pinned images and CI tests the wrong build. Worse, `BaseCommand` writes changed
config on the normal command lifecycle (`src/oclif/command/BaseCommand.js:113`), so a
load-time normaliser turns `dashmate config get` into a write; with two dashmate
binaries sharing a home the stored image becomes "whichever ran last", destroying the
audit trail that was the flag's main advantage.

**Resolve only at template-render time.** `dashmate config get …image` would print
`null`, hiding what will actually run.

## Interface

### Config

Stored state is private: `null` or an explicit string.

**Effective (ordinary reads):**
- `get(path)` — both exact and parent-object paths
- `getOptions()`
- the existing `config.options` compatibility property
- `toJSON()` / `JSON.stringify(config)`

**Stored (explicit, allowlisted):**
- `getStored(path)` / `getStoredOptions()` — return cloned raw state

`getResolvedOptions()` from the spike is **deleted**. Two ordinary-looking read APIs
recreate the ambiguity this design removes; there must be one normal read and one
unmistakably special one.

The effective snapshot is cached and deeply read-only, rebuilt after `set()` and
`setOptions()`.

**Raw access allowlist** — everything else must use effective reads:

| call site | why raw |
| --- | --- |
| `src/config/configFile/ConfigFile.js:311` (`toObject`) | persistence must never materialise a resolved value |
| `src/config/configFile/ConfigFile.js:189` (clone-from) | cloning a tracking config must stay tracking |
| `src/config/Config.js:256` (`isEqual`) | equality compares intent, not effect |
| `configs/defaults/get{Testnet,Local,Mainnet}ConfigFactory.js` | base → network inheritance carries intent |
| `src/listr/tasks/resetNodeTaskFactory.js:190` | reset restores intent |

An architectural test enforces this list, so adding a raw read elsewhere fails CI.

### Mutation must go through `set()`

Because effective reads return a read-only snapshot, code that mutates the object
returned by `get()` silently loses its writes. Two live sites must be converted to
clone-and-`set()`:

- `src/listr/tasks/setup/setupRegularPresetTaskFactory.js:92`
- `src/listr/tasks/setup/setupLocalPresetTaskFactory.js:129`

Both do `Object.values(config.get('core.rpc.users')).forEach(o => { o.password = … })`
during setup. Left unconverted, generated RPC passwords are silently discarded.

### Derived defaults

```js
export const DERIVED_DEFAULTS = {
  'platform.drive.abci.docker.image': () => `dashpay/drive:${dockerImageVersion}`,
  'platform.dapi.rsDapi.docker.image': () => `dashpay/rs-dapi:${dockerImageVersion}`,
};
```

Schema: `image` becomes `type: ['string', 'null']` for these two paths only. Note
`drive.abci.docker` uses `$ref: '#/definitions/dockerWithBuild'` while
`dapi.rsDapi.docker` duplicates the same shape inline — there are four inline `image`
schemas, and only these two change.

An architectural test asserts the set of nullable image paths **equals** the
`DERIVED_DEFAULTS` keys, so making envoy or tenderdash nullable cannot slip through.

### CLI

- `dashmate config get <path>` — effective value (unchanged output for operators)
- `dashmate config get --raw <path>` — stored value, `null` when tracking
- `dashmate config` / `--format=json` — effective, with `--raw` for stored
- `dashmate config set <path> null` — return to tracking (already parses literal
  `null`; the widened schema accepts it, so no new flag is needed)

### Data flow

```
config.json  (image: null)
   |
   +-- getStoredOptions() --> persistence, clone, isEqual, inheritance, reset   (null)
   |
   +-- get() / getOptions() / toJSON()
          -> dashmate config, config get, doctor archives, templates            (resolved)
          -> convertObjectToEnvs -> PLATFORM_DRIVE_ABCI_DOCKER_IMAGE -> compose (resolved)
```

## Migration

One migration converts stored stock tags to `null`, using `stockImagePattern()` from
`packages/dashmate/src/config/stockImages.js` (already on `v4.1-dev`). Anything not
matching stays an explicit string.

This still needs the stock-tag heuristic **once**. That is accepted: confining the
guess to a single migration is strictly better than repeating it every release. One
ambiguity is accepted with it — an operator who deliberately pinned exactly
`dashpay/drive:4-rc` is indistinguishable from an inherited default and will be
converted to tracking.

### Two release gates — both must land first

1. **Guard the unconditional `'4.0.0'` migration**
   (`configs/getConfigFileMigrationsFactory.js:1549`). It re-pins both images with no
   guard, so a pre-4.x config carrying `registry.example.com/patched-drive:stable`
   loses it *before* any classification can inspect it. Reproduced in review. The null
   migration inherits that damage, so this cannot be deferred.

2. **Fix the migration runner** (`src/config/configFile/migrateConfigFileFactory.js:24`):

   ```js
   .reduce((migratedOptions, version) => {
     const migrationFunction = configFileMigrations[version];
     return migrationFunction(rawConfigFile);   // accumulator discarded
   }, rawConfigFile);
   ```

   It passes the original object rather than the accumulator, working only because
   every migration happens to mutate in place and return the same object — a migration
   returning a new object silently drops every prior step. The same line also selects
   migrations newer than `toVersion`; correct semantics are `from < key <= to`.

### CI action must change

`.github/actions/local-network/action.yaml:85` string-matches `dashpay/(drive|rs-dapi):…`
in `config.json`. Against `null` it silently matches nothing and CI would run stock
images instead of the build under test. It must become two
`dashmate config set --config=local …` calls. **This is a cost of this design, not an
incidental cleanup.**

## Compatibility with `dashmate update` and `status`

Verified against the current code, not assumed. Both reach images through the same
chain, and effective-by-default makes them correct with no change:

```
dashmate update / status
  -> getServiceList(config)            src/docker/getServiceListFactory.js:21
     -> generateEnvs(config)           -> config.getOptions()   (effective in v2)
     -> reads docker-compose.yml `image: ${PLATFORM_..._DOCKER_IMAGE:?err}`
        and resolves the env name against those envs
  -> docker.pull(image)                src/update/updateNodeFactory.js
```

Consequences:

- `generateEnvs` needs **no** change. The spike had to redirect it to
  `getResolvedOptions()`; under v2 the ordinary accessor is already effective, so that
  spike edit is reverted. Update and status inherit correctness rather than opting in.
- `dashmate update` never writes config — it pulls and reports — so running it cannot
  materialise a resolved value into a tracking config.
- `status` (and therefore the dashmate helper HTTP API, whose only method is `status`)
  reports the effective image via `src/status/scopes/services.js`.
- The intended behaviour emerges without a migration: upgrade the dashmate binary, run
  `dashmate update`, and a tracking config pulls the new line while a pinned config
  pulls exactly what the operator pinned.

## Multi-network contract

Tracking is defined as:

> Use the Drive/rs-dapi image line derived from the dashmate executable performing the
> operation.

Consequences, to be documented:
- every tracking network in that dashmate home follows the invoking build;
- switching binaries changes effective output with no stored change and no config diff;
- mixed stable/RC operation requires explicit per-config pins or separate
  `DASHMATE_HOME_DIR` values.

## Out of scope

**Downgrade.** dashmate does not support it and has no down migrations. An older
dashmate reading `image: null` will fail schema validation. Documented, not designed
around.

## Failure modes

| failure | consequence | mitigation |
| --- | --- | --- |
| a new display surface reads stored state | shows `null` | effective is the default; raw needs a differently-named call, enforced by the allowlist test |
| a non-derived image made nullable | null image, no fallback | schema test asserts nullable paths equal `DERIVED_DEFAULTS` keys |
| code mutates the object returned by `get()` | writes silently lost | read-only snapshot makes it throw; two known sites converted |
| one-time migration nulls a deliberate stock pin | operator silently moved to tracking | accepted; enumerated identifier guard limits it |
| `DERIVED_DEFAULTS` drifts from the schema | null reaches compose | compose uses `:?err` and fails loudly; plus the equality test above |
| out-of-band writer expects a string | no match, silent no-op | CI action converted to `config set`; documented for operators |

## Test plan

1. Effective reads: exact path, **parent object**, `getOptions()`, `config.options`,
   `toJSON()`.
2. Stored reads stay `null`; `toObject()` (what reaches disk) never materialises.
3. Compose envs resolve in both tracking and pinned cases.
4. Explicit override survives; `config set <path> null` returns to tracking.
5. Mutating a `get()` result throws rather than silently no-oping.
6. Raw-access allowlist: architectural test over the source tree.
7. Schema: nullable paths equal `DERIVED_DEFAULTS` keys; a non-derived image rejects
   `null`.
8. Stable vs RC binary resolve differently from identical stored state.
9. Migration: stock tags → `null`; the operator-image table from
   `should move only the stock version-derived tags and leave operator images alone`
   stays untouched; a pre-4.x custom image survives (requires gate 1).
10. Runner: `from < key <= to`, accumulator threaded, equal-version and future-key
    cases (requires gate 2).
11. The existing table-walk test is retired — once nothing stores a derived value there
    is no stale tag for it to detect.

## Open questions

- Should `dashmate config` annotate tracking values (e.g. `dashpay/drive:4-rc (default)`)
  or stay byte-identical to today's output?
- Should the four inline `image` schemas be collapsed onto the shared `$ref` as a
  separate cleanup first?
- Does the legacy `platform.dapi.api.docker.image` path still exist in any live config?
  It accounts for 20 historical re-pins but is absent from the current base config.
