import childProcess from 'child_process';
import fs, { promises as fsPromises } from 'fs';
import semver from 'semver';
import { promisify } from 'util';

import { DASHMATE_VERSION } from '../constants.js';
import detectInstallMethod, { INSTALL_METHODS } from './detectInstallMethod.js';

/**
 * Which version the channel that installed dashmate would install next, and whether it is
 * worth installing.
 *
 * The candidate always comes from the channel that will actually perform the upgrade: apt for
 * a `.deb` install, the npm registry for an npm install. Asking a third party — the GitHub
 * API, say — would mean the version an operator approves and the version the package manager
 * then installs are unrelated numbers, which makes the confirmation decorative, and it would
 * put an unauthenticated HTTP response in the decision path of the one channel whose whole
 * point is that it is signed.
 *
 * Two orderings are involved and they are deliberately kept apart. apt reports Debian versions
 * (`4.1.0~rc.3-1`), which order under dpkg's rules; dashmate has an application version
 * (`4.1.0-rc.3`), which orders under semver. Debian versions are translated to application
 * versions here for one purpose only — deciding whether the candidate is newer than what is
 * running — and no Debian version is ever compared with semver, nor an application version
 * with dpkg. Package ordering stays the package manager's job.
 *
 * Every version string is validated at the boundary before it is returned, printed or could
 * reach a package-manager argument. npm accepts `git+https://…`, `file:…`, `https://….tgz` and
 * `npm:` aliases wherever it accepts a version, so nothing but a strict version shape may reach
 * `npm install -g dashmate@<spec>`, whatever the registry returned. Passing arguments with
 * execFile and no shell does not change that: the specifier is one argument either way.
 *
 * Never throws. A missing apt, an unreachable registry, a rate limit, malformed output — all
 * of them degrade to a labelled unknown carrying a machine-readable reason. Unknown is its own
 * state precisely because it must never be read as "no update available".
 *
 * Read-only and unprivileged: this resolves what *would* happen, it never installs anything.
 * See the note on `apt-get update` below for the root decision.
 */

export const CANDIDATE_STATE = {
  UPDATE_AVAILABLE: 'update-available',
  UP_TO_DATE: 'up-to-date',
  UNKNOWN: 'unknown',
};

export const CANDIDATE_REASON = {
  // decided
  NEWER_VERSION_AVAILABLE: 'newer-version-available',
  PRERELEASE_AVAILABLE: 'prerelease-available',
  PACKAGE_REBUILD_AVAILABLE: 'package-rebuild-available',
  ALREADY_CURRENT: 'already-current',
  CANDIDATE_OLDER: 'candidate-older',
  // channel
  UNSUPPORTED_INSTALL_METHOD: 'unsupported-install-method',
  AMBIGUOUS_INSTALL_METHOD: 'ambiguous-install-method',
  UNSUPPORTED_PLATFORM: 'unsupported-platform',
  // apt
  APT_MISSING: 'apt-missing',
  APT_SOURCES_MISSING: 'apt-sources-missing',
  APT_FAILED: 'apt-failed',
  APT_OUTPUT_MALFORMED: 'apt-output-malformed',
  APT_NO_CANDIDATE: 'apt-no-candidate',
  DEB_VERSION_UNMAPPABLE: 'deb-version-unmappable',
  PACKAGE_REBUILD_UNDETERMINED: 'package-rebuild-undetermined',
  PACKAGE_HELD: 'package-held',
  // npm registry
  REGISTRY_UNREACHABLE: 'registry-unreachable',
  REGISTRY_ERROR: 'registry-error',
  REGISTRY_RATE_LIMITED: 'registry-rate-limited',
  REGISTRY_PACKAGE_MISSING: 'registry-package-missing',
  REGISTRY_RESPONSE_MALFORMED: 'registry-response-malformed',
  REGISTRY_RESPONSE_TOO_LARGE: 'registry-response-too-large',
  // either channel
  INVALID_VERSION: 'invalid-version',
  CURRENT_VERSION_INVALID: 'current-version-invalid',
  RESOLUTION_FAILED: 'resolution-failed',
};

/**
 * Where a candidate came from.
 */
export const CANDIDATE_SOURCE = {
  APT: 'apt',
  NPM_REGISTRY: 'npm-registry',
};

const PACKAGE_NAME = 'dashmate';

const APT_GET_PATH = '/usr/bin/apt-get';
const APT_CACHE_PATH = '/usr/bin/apt-cache';
const DPKG_PATH = '/usr/bin/dpkg';
const DPKG_QUERY_PATH = '/usr/bin/dpkg-query';

/**
 * dpkg's own report of what the operator asked for: three characters, the first of which is
 * `h` when the package is held.
 */
// eslint-disable-next-line no-template-curly-in-string
const DPKG_STATUS_FORMAT = '-f=${db:Status-Abbrev}';

/**
 * Our own deb822 sources file, shipped by the package.
 *
 * Every apt invocation is scoped to it, so an unrelated third-party source that is broken,
 * unreachable or unsigned cannot abort the check, and so a "dashmate" package from some other
 * repository can never become our candidate.
 */
const APT_SOURCES_PATH = '/etc/apt/sources.list.d/dashmate.sources';

const NPM_REGISTRY_URL = 'https://registry.npmjs.org';
const NPM_DIST_TAG = 'latest';

const APT_UPDATE_TIMEOUT_MS = 60000;
const APT_CACHE_TIMEOUT_MS = 15000;
const DPKG_TIMEOUT_MS = 5000;
const COMMAND_MAX_BUFFER = 1024 * 1024;

const REGISTRY_TIMEOUT_MS = 5000;
const MAX_RESPONSE_SIZE = 1024 * 1024;

const MAX_LOG_LENGTH = 200;

/**
 * The only version shape accepted from a channel.
 *
 * Anchored, with character classes that leave no room for control or ANSI escape characters,
 * nor for anything a package manager would read as a location to install from.
 */
const VERSION_REGEX = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/;

/**
 * Characters Debian policy allows in a version, so anything else in apt's output is not a
 * version at all and is refused before it is stored or printed.
 */
const DEB_VERSION_REGEX = /^[0-9][A-Za-z0-9.+~:-]*$/;

// C0 and C1 controls, zero-width characters, line and paragraph separators, and the
// bidirectional overrides that reorder what an operator reads. Matching them is the point.
// eslint-disable-next-line no-control-regex
const UNSAFE_LOG_CHARACTERS_REGEX = /[\u0000-\u001f\u007f-\u009f\u200b-\u200f\u2028-\u2029\u202a-\u202e\u2066-\u2069\ufeff]/g;

/**
 * Environment handed to every subprocess.
 *
 * An allowlist built from nothing, not a filtered copy of process.env. APT_CONFIG alone can
 * relocate the sources apt reads and switch on APT::Get::AllowUnauthenticated, which would
 * silently disable the signature verification this whole channel rests on; NODE_OPTIONS,
 * NODE_PATH and LD_* redirect a child into arbitrary code.
 */
const COMMAND_ENV = Object.freeze({
  PATH: '/usr/sbin:/usr/bin:/sbin:/bin',
  // apt localises its output, and both its output and its exit behaviour are parsed here
  LC_ALL: 'C',
  LANG: 'C',
  // apt must never try to open a dialog on a path that runs unattended
  DEBIAN_FRONTEND: 'noninteractive',
});

/**
 * apt options that scope an invocation to our own sources file.
 *
 * `Dir::Etc::sourceparts=/dev/null` empties the drop-in directory and `Dir::Etc::sourcelist`
 * replaces the main list with ours, so a broken or unreachable third-party source cannot abort
 * the check and a "dashmate" package from some other repository can never become the candidate.
 *
 * Two options exist to keep that scoping from having side effects on the host. Without
 * `APT::Get::List-Cleanup=0` a scoped `apt-get update` sees every other repository's index as
 * belonging to no configured source and deletes it. Without the empty cache paths, apt writes
 * `pkgcache.bin` from a cache built out of our source alone, so the next unrelated apt command
 * has to rebuild it; empty values keep the cache in memory instead.
 *
 * The isolation is real but partial, and worth stating rather than assuming: apt's lock, the
 * downloaded lists, `apt.conf.d` and — when this runs as root — `Update::Post-Invoke` hooks are
 * all still shared with the rest of the system.
 *
 * @param {string} sourcesPath
 * @return {string[]}
 */
function aptScopeArgs(sourcesPath) {
  return [
    '-o', `Dir::Etc::sourcelist=${sourcesPath}`,
    '-o', 'Dir::Etc::sourceparts=/dev/null',
    '-o', 'APT::Get::List-Cleanup=0',
    '-o', 'Dir::Cache::pkgcache=',
    '-o', 'Dir::Cache::srcpkgcache=',
  ];
}

const execFileAsync = promisify(childProcess.execFile);

/**
 * Statuses the registry seam reports, so a caller can tell a rate limit from an unreachable
 * host without an exception carrying that meaning in a string.
 */
export const FETCH_STATUS = {
  OK: 'ok',
  HTTP_ERROR: 'http-error',
  NETWORK_ERROR: 'network-error',
  TOO_LARGE: 'too-large',
  MALFORMED: 'malformed',
};

/**
 * Make text received from a remote host or a subprocess safe to print.
 *
 * Diagnostics quote the payload that failed to parse, so foreign bytes reach the terminal
 * through messages even when the value itself is rejected.
 *
 * @param {*} text
 * @return {string}
 */
function sanitize(text) {
  return typeof text === 'string'
    ? text.replace(UNSAFE_LOG_CHARACTERS_REGEX, '').slice(0, MAX_LOG_LENGTH)
    : '[unprintable]';
}

/**
 * Accept a version string only if it is a version.
 *
 * The shape check runs first and the semver parse second: the regex refuses what semver would
 * accept in a package-manager context (a `git+https://` specifier is not a version but is a
 * perfectly good argument to `npm install`), and semver refuses what the shape allows but
 * cannot order, such as the leading zeroes in `01.2.3`. The normalised result drops build
 * metadata, which comparison ignores and a package manager would refuse to resolve.
 *
 * @param {*} version
 * @return {string|null}
 */
function parseVersion(version) {
  if (typeof version !== 'string' || !VERSION_REGEX.test(version)) {
    return null;
  }

  return semver.valid(version);
}

/**
 * Translate a Debian version into the application version it packages.
 *
 * apt speaks Debian versions and dashmate speaks semver, and they are different strings under
 * different orderings: `4.1.0~rc.3-1` is the package, `4.1.0-rc.3` is the application. The
 * translation is mechanical and one-way:
 *
 * - an epoch (`1:`) is a packaging device with no application meaning, so it is dropped;
 * - the Debian revision is everything after the last hyphen — it identifies a rebuild of the
 *   same application version, so it is dropped too;
 * - `~` is Debian's prerelease idiom, sorting below the release it precedes, which is exactly
 *   what a semver prerelease tag means, so the two map onto each other.
 *
 * Anything the result cannot be validated as is refused rather than approximated. In
 * particular the legacy scheme oclif emitted, `4.1.0.bfc80249b9-1`, has no application version
 * to recover: truncating it to `4.1.0` would silently equate a release with a random build of
 * it. Callers get an unknown carrying the raw string instead of a wrong number.
 *
 * Because the revision is dropped, a rebuild of the same application version (`4.1.0-1` →
 * `4.1.0-2`) maps to the version already running and is therefore reported as up to date.
 * apt still sees it as an upgrade and `apt-get install --only-upgrade` would apply it; there
 * is no application-version difference for this comparison to find.
 *
 * @param {string} debVersion
 * @return {string|null} application version, or null when the package version does not carry one
 */
export function debVersionToApplicationVersion(debVersion) {
  if (typeof debVersion !== 'string' || !DEB_VERSION_REGEX.test(debVersion)) {
    return null;
  }

  let remainder = debVersion;

  const epochSeparatorIndex = remainder.indexOf(':');

  if (epochSeparatorIndex !== -1) {
    const epoch = remainder.slice(0, epochSeparatorIndex);

    if (!/^\d+$/.test(epoch)) {
      return null;
    }

    remainder = remainder.slice(epochSeparatorIndex + 1);
  }

  const revisionSeparatorIndex = remainder.lastIndexOf('-');
  const upstreamVersion = revisionSeparatorIndex === -1
    ? remainder
    : remainder.slice(0, revisionSeparatorIndex);

  // Only the first tilde is translated. A version carrying more than one does not follow the
  // scheme this maps, and the leftover tilde makes the validation below refuse it.
  return parseVersion(upstreamVersion.replace('~', '-'));
}

/**
 * @param {string} filePath
 * @return {Promise<boolean>}
 */
async function defaultPathExists(filePath) {
  try {
    await fsPromises.access(filePath, fs.constants.F_OK);

    return true;
  } catch {
    return false;
  }
}

/**
 * The options every subprocess is spawned with.
 *
 * Separate from the call so the guarantees can be asserted on their own: the environment is an
 * allowlist rather than an inherited one, there is no shell, and both the time and the output
 * are bounded.
 *
 * @param {number} timeout
 * @return {Object}
 */
export function commandOptions(timeout) {
  return {
    env: { ...COMMAND_ENV },
    timeout,
    maxBuffer: COMMAND_MAX_BUFFER,
    windowsHide: true,
  };
}

/**
 * Run a command with an argv array and an allowlisted environment.
 *
 * execFile with an argv array, never exec and never shell: true, so nothing in a path, an
 * environment variable or a package version can become a shell token. Bounded in both time and
 * output, because this runs on paths an operator is waiting on.
 *
 * @param {string} file - absolute path to the executable
 * @param {string[]} args
 * @param {Object} [options]
 * @param {number} [options.timeout]
 * @param {function} [options.execFile] - injection seam for tests
 * @return {Promise<string>} stdout
 */
export async function defaultRunCommand(file, args, options = {}) {
  const { timeout = APT_CACHE_TIMEOUT_MS, execFile = execFileAsync } = options;

  const { stdout } = await execFile(file, args, commandOptions(timeout));

  return stdout;
}

/**
 * Fetch JSON from the registry with a bounded response.
 *
 * @param {string} url
 * @return {Promise<{status: string, body?: Object, statusCode?: number, message?: string}>}
 */
export async function defaultFetchJson(url) {
  let response;

  try {
    response = await fetch(url, {
      headers: { accept: 'application/json' },
      signal: AbortSignal.timeout(REGISTRY_TIMEOUT_MS),
    });
  } catch (e) {
    return { status: FETCH_STATUS.NETWORK_ERROR, message: e?.message };
  }

  // Until the body is cancelled the connection stays checked out of the pool
  const discard = () => response.body?.cancel().catch(() => {});

  if (!response.ok) {
    discard();

    return { status: FETCH_STATUS.HTTP_ERROR, statusCode: response.status };
  }

  const declaredSize = Number(response.headers.get('content-length'));

  if (Number.isFinite(declaredSize) && declaredSize > MAX_RESPONSE_SIZE) {
    discard();

    return { status: FETCH_STATUS.TOO_LARGE };
  }

  if (!response.body) {
    return { status: FETCH_STATUS.MALFORMED, message: 'response carried no body' };
  }

  const chunks = [];
  let size = 0;

  try {
    // The declared size is only a hint, so the body is measured while it is read
    for await (const chunk of response.body) {
      size += chunk.length;

      if (size > MAX_RESPONSE_SIZE) {
        discard();

        return { status: FETCH_STATUS.TOO_LARGE };
      }

      chunks.push(chunk);
    }
  } catch (e) {
    return { status: FETCH_STATUS.MALFORMED, message: e?.message };
  }

  try {
    return { status: FETCH_STATUS.OK, body: JSON.parse(Buffer.concat(chunks).toString('utf8')) };
  } catch (e) {
    return { status: FETCH_STATUS.MALFORMED, message: e?.message };
  }
}

/**
 * The one place a result is built.
 *
 * `installableVersion` is derived here rather than passed in, and that is the point: it is the
 * only field a caller should ever hand to a package manager, and deriving it from the state
 * makes it structurally impossible for a version the module refused to act on — a downgrade, a
 * held package, an unknown — to leave through it. `candidateVersion` still reports whatever the
 * channel offers, for display.
 *
 * @param {Object} base - method, currentVersion, updateInstructions and evidence of the resolution
 * @param {string} state
 * @param {string} reason
 * @param {string} message
 * @param {Object} [properties]
 * @return {UpdateCandidate}
 */
function result(base, state, reason, message, properties = {}) {
  const candidate = {
    state,
    reason,
    message,
    method: base.method,
    source: null,
    currentVersion: base.currentVersion,
    candidateVersion: null,
    packageVersion: null,
    prerelease: null,
    updateInstructions: base.updateInstructions,
    ...properties,
    details: {
      installMethodEvidence: base.installMethodEvidence,
      ...properties.details,
    },
  };

  return {
    ...candidate,
    installableVersion: state === CANDIDATE_STATE.UPDATE_AVAILABLE
      ? candidate.candidateVersion
      : null,
  };
}

/**
 * Decide what a validated candidate version means for the version that is running.
 *
 * `semver.gt` against the running version is the whole rule. There is no record of the highest
 * version ever seen: it would matter only once a downgrade had already become the baseline,
 * while permanently blocking an operator who deliberately installs an older release after a
 * broken one.
 *
 * A candidate lower than the running version is an answer, not a failure — the channel has
 * nothing to offer, so the state is up to date, with its own reason so the caller can say why.
 *
 * @param {Object} base
 * @param {string} candidateVersion - already validated
 * @param {Object} properties - source, packageVersion and details of the resolution
 * @return {UpdateCandidate}
 */
function compareWithRunningVersion(base, candidateVersion, properties) {
  const common = {
    ...properties,
    candidateVersion,
    prerelease: semver.prerelease(candidateVersion) !== null,
  };

  if (semver.gt(candidateVersion, base.currentVersion)) {
    // A prerelease gets its own reason rather than a flag on the same one. Prereleases and
    // stable releases share a flat repository, so whatever sorts highest is what the channel
    // offers, and a stable node is one upgrade away from sitting on a release candidate. A
    // caller decides on `reason`, so the distinction has to live there to be actionable.
    const isPrerelease = common.prerelease;

    return result(
      base,
      CANDIDATE_STATE.UPDATE_AVAILABLE,
      isPrerelease
        ? CANDIDATE_REASON.PRERELEASE_AVAILABLE
        : CANDIDATE_REASON.NEWER_VERSION_AVAILABLE,
      isPrerelease
        ? `dashmate ${candidateVersion} is available, this node runs ${base.currentVersion}. It is a prerelease`
        : `dashmate ${candidateVersion} is available, this node runs ${base.currentVersion}`,
      common,
    );
  }

  if (semver.eq(candidateVersion, base.currentVersion)) {
    return result(
      base,
      CANDIDATE_STATE.UP_TO_DATE,
      CANDIDATE_REASON.ALREADY_CURRENT,
      `dashmate ${base.currentVersion} is the version this channel offers`,
      common,
    );
  }

  return result(
    base,
    CANDIDATE_STATE.UP_TO_DATE,
    CANDIDATE_REASON.CANDIDATE_OLDER,
    `This channel offers dashmate ${candidateVersion}, which is older than the ${base.currentVersion} running here, so it is not an update`,
    common,
  );
}

/**
 * Read one field out of `apt-cache policy` output.
 *
 * @param {string} stdout
 * @param {string} field - Installed or Candidate
 * @return {string|null}
 */
function readPolicyField(stdout, field) {
  const line = stdout
    .split('\n')
    .find((candidate) => candidate.trim().startsWith(`${field}:`));

  if (line === undefined) {
    return null;
  }

  const value = line.trim().slice(`${field}:`.length).trim();

  return value.length === 0 ? null : value;
}

/**
 * Refresh apt's package lists, when this process can.
 *
 * `apt-get update` writes to /var/lib/apt/lists and takes apt's lock, so it needs root. A
 * version *check* that demanded sudo would be unusable — it runs on notification paths, and
 * asking a masternode operator for a password to answer "is there an update?" trains exactly
 * the habit this design should not create. So: refresh when this process already is root,
 * otherwise skip it and answer from the lists the host already has, reporting which of the two
 * happened. The caller that goes on to upgrade holds root by then and refreshes as part of the
 * install, so nothing downstream depends on this having run.
 *
 * A refresh failure is never fatal either. Stale lists still answer the question, they just
 * answer it as of the last refresh.
 *
 * @param {Object} options
 * @return {Promise<{refreshed: boolean, skippedReason: string|null}>}
 */
async function refreshAptLists({ runCommand, sourcesPath, isRoot }) {
  if (!isRoot) {
    return {
      refreshed: false,
      skippedReason: 'not running as root, so apt package lists were not refreshed',
    };
  }

  try {
    // `--error-on=any` makes a source that failed to fetch an error rather than a warning, so
    // a refresh that half worked is not reported as a refresh that worked
    await runCommand(
      APT_GET_PATH,
      [...aptScopeArgs(sourcesPath), '--error-on=any', 'update'],
      { timeout: APT_UPDATE_TIMEOUT_MS },
    );

    return { refreshed: true, skippedReason: null };
  } catch (e) {
    return {
      refreshed: false,
      skippedReason: `apt-get update failed: ${sanitize(e?.message)}`,
    };
  }
}

/**
 * Say so when apt answered from lists nobody refreshed.
 *
 * "apt has no candidate" and "apt has not looked since before the release" read identically to
 * an operator otherwise, and only one of them is worth acting on.
 *
 * @param {Object} details
 * @return {string}
 */
function staleListsNote(details) {
  return details.listsRefreshed
    ? ''
    : ` (${details.listsNotRefreshedReason})`;
}

/**
 * Whether one Debian version sorts above another, decided by dpkg.
 *
 * Debian ordering is not semver ordering and this module does not attempt it: `dpkg
 * --compare-versions` is the implementation of the rules, so the question is asked there. Both
 * arguments have already been validated as Debian versions, so nothing else can reach the argv.
 *
 * @param {function} runCommand
 * @param {string} greater
 * @param {string} lesser
 * @return {Promise<boolean|null>} null when dpkg could not answer
 */
async function isDebVersionGreater(runCommand, greater, lesser) {
  try {
    await runCommand(
      DPKG_PATH,
      ['--compare-versions', greater, 'gt', lesser],
      { timeout: DPKG_TIMEOUT_MS },
    );

    return true;
  } catch (e) {
    // dpkg exits 1 for "the comparison is false" and 2 for a problem, so only the first is an
    // answer. A missing dpkg lands here too.
    return e?.code === 1 ? false : null;
  }
}

/**
 * Whether the operator has pinned the package with `apt-mark hold`.
 *
 * A hold does not change what apt reports as the candidate, so without this a held host would
 * be told an update is available forever, and a consented install would exit 0 having kept the
 * package back.
 *
 * @param {function} runCommand
 * @return {Promise<boolean|null>} null when dpkg could not answer
 */
async function isPackageHeld(runCommand) {
  try {
    const stdout = await runCommand(
      DPKG_QUERY_PATH,
      ['-W', DPKG_STATUS_FORMAT, PACKAGE_NAME],
      { timeout: DPKG_TIMEOUT_MS },
    );

    return typeof stdout === 'string' ? stdout.trim().startsWith('h') : null;
  } catch {
    return null;
  }
}

/**
 * Whether apt is offering a rebuild of the application version that is already running.
 *
 * The Debian revision (`-1` → `-2`) identifies a rebuild of the same application version, so it
 * maps to the version running here and the application-version comparison finds nothing. A
 * rebuild is how a fix to anything packaged alongside dashmate ships — the bundled Node runtime
 * above all — so reporting it as up to date tells an operator they are patched while a patch is
 * waiting.
 *
 * Only the ordering of the two package versions is delegated to dpkg; the decision of what that
 * means stays here. An unorderable pair is reported as unknown rather than guessed, because the
 * candidate can legitimately sort *below* what is installed once a pool prunes old versions.
 *
 * @param {Object} base
 * @param {string} candidateVersion - the application version both package versions carry
 * @param {string} candidatePackageVersion
 * @param {Object} properties
 * @param {function} runCommand
 * @return {Promise<UpdateCandidate|null>} null when there is no rebuild to report
 */
async function resolvePackageRebuild(
  base,
  candidateVersion,
  candidatePackageVersion,
  properties,
  runCommand,
) {
  const { installedPackageVersion } = properties.details;

  if (installedPackageVersion === null || installedPackageVersion === candidatePackageVersion) {
    return null;
  }

  const common = { ...properties, candidateVersion, prerelease: semver.prerelease(candidateVersion) !== null };

  const isNewer = await isDebVersionGreater(
    runCommand,
    candidatePackageVersion,
    installedPackageVersion,
  );

  if (isNewer === true) {
    return result(
      base,
      CANDIDATE_STATE.UPDATE_AVAILABLE,
      CANDIDATE_REASON.PACKAGE_REBUILD_AVAILABLE,
      `apt offers package ${candidatePackageVersion}, a rebuild of the dashmate ${candidateVersion} installed as ${installedPackageVersion}`,
      common,
    );
  }

  if (isNewer === false) {
    return null;
  }

  return result(
    base,
    CANDIDATE_STATE.UNKNOWN,
    CANDIDATE_REASON.PACKAGE_REBUILD_UNDETERMINED,
    `apt offers package ${candidatePackageVersion} where ${installedPackageVersion} is installed, and dpkg could not be asked which is newer`,
    common,
  );
}

/**
 * Ask apt which version it would install.
 *
 * @param {Object} base
 * @param {Object} options
 * @return {Promise<UpdateCandidate>}
 */
async function resolveFromApt(base, options) {
  const {
    runCommand,
    pathExists,
    aptSourcesPath,
    isRoot,
  } = options;

  const details = {
    aptSourcesPath,
    listsRefreshed: false,
    listsNotRefreshedReason: null,
    installedPackageVersion: null,
    // set when apt reported an Installed field that is not a package version, since that field
    // is what a rebuild is detected against and losing it silently would hide the detection
    installedPackageVersionReported: null,
    held: null,
  };

  // A dashmate installed before the apt repository existed has no sources file, so apt has
  // nothing to answer from. Reported as its own reason: the operator has a one-time setup to
  // do, which is a different situation from apt being broken.
  if (!await pathExists(aptSourcesPath)) {
    return result(
      base,
      CANDIDATE_STATE.UNKNOWN,
      CANDIDATE_REASON.APT_SOURCES_MISSING,
      `${aptSourcesPath} does not exist, so apt has no dashmate repository to offer a version from`,
      { source: CANDIDATE_SOURCE.APT, details },
    );
  }

  const refresh = await refreshAptLists({ runCommand, sourcesPath: aptSourcesPath, isRoot });

  details.listsRefreshed = refresh.refreshed;
  details.listsNotRefreshedReason = refresh.skippedReason;

  let stdout;

  try {
    stdout = await runCommand(
      APT_CACHE_PATH,
      [...aptScopeArgs(aptSourcesPath), 'policy', PACKAGE_NAME],
      { timeout: APT_CACHE_TIMEOUT_MS },
    );
  } catch (e) {
    if (e?.code === 'ENOENT') {
      return result(
        base,
        CANDIDATE_STATE.UNKNOWN,
        CANDIDATE_REASON.APT_MISSING,
        `${APT_CACHE_PATH} is not installed, so this host has no apt to ask`,
        { source: CANDIDATE_SOURCE.APT, details },
      );
    }

    return result(
      base,
      CANDIDATE_STATE.UNKNOWN,
      CANDIDATE_REASON.APT_FAILED,
      `Could not ask apt which version it would install: ${sanitize(e?.message)}`,
      { source: CANDIDATE_SOURCE.APT, details },
    );
  }

  if (typeof stdout !== 'string' || stdout.trim().length === 0) {
    return result(
      base,
      CANDIDATE_STATE.UNKNOWN,
      CANDIDATE_REASON.APT_NO_CANDIDATE,
      `apt knows no ${PACKAGE_NAME} package from ${aptSourcesPath}${staleListsNote(details)}`,
      { source: CANDIDATE_SOURCE.APT, details },
    );
  }

  const installedField = readPolicyField(stdout, 'Installed');

  if (installedField !== null && DEB_VERSION_REGEX.test(installedField)) {
    details.installedPackageVersion = installedField;
  } else if (installedField !== null && installedField !== '(none)') {
    details.installedPackageVersionReported = sanitize(installedField);
  }

  const candidateField = readPolicyField(stdout, 'Candidate');

  if (candidateField === null) {
    return result(
      base,
      CANDIDATE_STATE.UNKNOWN,
      CANDIDATE_REASON.APT_OUTPUT_MALFORMED,
      `apt-cache policy reported no candidate line for ${PACKAGE_NAME}`,
      { source: CANDIDATE_SOURCE.APT, details },
    );
  }

  // apt says "(none)" when the package is known but no configured source offers it, which is
  // what a host sees between a release being announced and the pool serving its architecture.
  if (candidateField === '(none)') {
    return result(
      base,
      CANDIDATE_STATE.UNKNOWN,
      CANDIDATE_REASON.APT_NO_CANDIDATE,
      `apt has no candidate version of ${PACKAGE_NAME} from ${aptSourcesPath}${staleListsNote(details)}`,
      { source: CANDIDATE_SOURCE.APT, details },
    );
  }

  if (!DEB_VERSION_REGEX.test(candidateField)) {
    return result(
      base,
      CANDIDATE_STATE.UNKNOWN,
      CANDIDATE_REASON.APT_OUTPUT_MALFORMED,
      `apt-cache policy reported "${sanitize(candidateField)}" where a package version was expected`,
      { source: CANDIDATE_SOURCE.APT, details },
    );
  }

  const properties = {
    source: CANDIDATE_SOURCE.APT,
    packageVersion: candidateField,
    details,
  };

  const candidateVersion = debVersionToApplicationVersion(candidateField);

  if (candidateVersion === null) {
    return result(
      base,
      CANDIDATE_STATE.UNKNOWN,
      CANDIDATE_REASON.DEB_VERSION_UNMAPPABLE,
      `apt offers package version ${candidateField}, which does not carry an application version this dashmate can compare`,
      properties,
    );
  }

  const rebuild = semver.eq(candidateVersion, base.currentVersion)
    ? await resolvePackageRebuild(base, candidateVersion, candidateField, properties, runCommand)
    : null;

  const candidate = rebuild ?? compareWithRunningVersion(base, candidateVersion, properties);

  if (candidate.state !== CANDIDATE_STATE.UPDATE_AVAILABLE) {
    return candidate;
  }

  // Only asked when there is something to hold back, so a host with nothing to install pays
  // one child process rather than two
  details.held = await isPackageHeld(runCommand);

  if (details.held !== true) {
    return candidate;
  }

  // A hold is the operator saying "not this host". Reported as unknown rather than up to date,
  // because a newer version does exist, and not as an update, because nothing may act on it:
  // apt would keep the package back and exit 0, which reads as a successful upgrade.
  return result(
    base,
    CANDIDATE_STATE.UNKNOWN,
    CANDIDATE_REASON.PACKAGE_HELD,
    `apt offers ${candidateField} but ${PACKAGE_NAME} is held, so it will not be installed until the hold is removed`,
    { ...properties, candidateVersion, prerelease: candidate.prerelease },
  );
}

/**
 * Which registry to ask.
 *
 * npm reads `npm_config_registry` from the environment for its own resolution, so a host
 * pointed at a private mirror or a caching proxy installs from there — and asking the public
 * registry would then answer about a different set of packages than the upgrade would install,
 * which is the same-source rule the deb path keeps by scoping to our own sources file.
 * Honouring it grants nothing new to whoever sets it: they already control what `npm install`
 * resolves. The version string coming back is validated exactly as strictly either way.
 *
 * https only, and no credentials in the URL: a value that cannot be one is ignored rather than
 * followed, and reported so the answer's provenance is visible.
 *
 * The gap this does not close: npm also reads a registry from `.npmrc`, which is a file this
 * module deliberately does not parse, so a host configured that way is still answered for by
 * the public registry.
 *
 * @param {string|null} explicitUrl - injected by a caller, wins over the environment
 * @param {NodeJS.ProcessEnv} env
 * @return {{url: string, source: string, ignored: string|null}}
 */
function resolveRegistryUrl(explicitUrl, env) {
  if (typeof explicitUrl === 'string' && explicitUrl.length > 0) {
    return { url: explicitUrl, source: 'caller', ignored: null };
  }

  const configured = env?.npm_config_registry ?? env?.NPM_CONFIG_REGISTRY;

  if (typeof configured !== 'string' || configured.length === 0) {
    return { url: NPM_REGISTRY_URL, source: 'default', ignored: null };
  }

  let parsed;

  try {
    parsed = new URL(configured);
  } catch {
    return { url: NPM_REGISTRY_URL, source: 'default', ignored: sanitize(configured) };
  }

  if (parsed.protocol !== 'https:' || parsed.username !== '' || parsed.password !== '') {
    return { url: NPM_REGISTRY_URL, source: 'default', ignored: sanitize(configured) };
  }

  return {
    url: `${parsed.origin}${parsed.pathname}`.replace(/\/+$/, ''),
    source: 'environment',
    ignored: null,
  };
}

/**
 * Whether a detection that produced no usable channel did so because the signals disagreed.
 *
 * A source checkout has no channel and nothing is wrong; two bin wrappers claiming the same
 * process, or a deb layout dpkg does not own, mean the host has more than one dashmate or one
 * that is not what it appears to be. A caller acting on non-interactive consent must be able to
 * refuse the second case while merely printing instructions for the first.
 *
 * @param {Object} [evidence] - as reported by detectInstallMethod
 * @return {boolean}
 */
function isAmbiguousDetection(evidence) {
  if (evidence === undefined || evidence === null) {
    return false;
  }

  return (evidence.hasBinPathEnv === true && evidence.hasUpdateInstructionsEnv === true)
    || evidence.debPackageConfirmed === false
    || (evidence.hasUpdateInstructionsEnv === true && evidence.debPackageConfirmed !== true);
}

/**
 * Ask the npm registry which version the dist-tag points at.
 *
 * The registry is queried for the tag rather than the whole packument: the tag is what
 * `npm install -g dashmate@latest` resolves, so it is the same answer the upgrade would get.
 *
 * @param {Object} base
 * @param {Object} options
 * @return {Promise<UpdateCandidate>}
 */
async function resolveFromNpmRegistry(base, options) {
  const {
    fetchJson, registryUrl, distTag, env,
  } = options;

  const registry = resolveRegistryUrl(registryUrl, env);

  const url = `${registry.url}/${PACKAGE_NAME}/${distTag}`;
  const details = {
    registryUrl: registry.url,
    registryUrlSource: registry.source,
    registryUrlIgnored: registry.ignored,
    distTag,
  };

  const response = await fetchJson(url);

  if (response?.status !== FETCH_STATUS.OK) {
    if (response?.status === FETCH_STATUS.NETWORK_ERROR) {
      return result(
        base,
        CANDIDATE_STATE.UNKNOWN,
        CANDIDATE_REASON.REGISTRY_UNREACHABLE,
        `Could not reach the npm registry: ${sanitize(response.message)}`,
        { source: CANDIDATE_SOURCE.NPM_REGISTRY, details },
      );
    }

    if (response?.status === FETCH_STATUS.HTTP_ERROR) {
      // Three answers with three consequences: a rate limit resolves itself and a caller
      // caching negative results should retry sooner; a 404 means this registry does not carry
      // the package or the tag yet, which is what an npm operator sees in the window between a
      // GitHub release and the publish; anything else is a fault.
      const byStatus = {
        429: [
          CANDIDATE_REASON.REGISTRY_RATE_LIMITED,
          'The npm registry rate-limited this host, so the latest version is not known right now',
        ],
        404: [
          CANDIDATE_REASON.REGISTRY_PACKAGE_MISSING,
          `${registry.url} does not carry ${PACKAGE_NAME}@${distTag} yet`,
        ],
      };

      const [reason, message] = byStatus[response.statusCode] ?? [
        CANDIDATE_REASON.REGISTRY_ERROR,
        `The npm registry answered with status ${response.statusCode}`,
      ];

      return result(
        base,
        CANDIDATE_STATE.UNKNOWN,
        reason,
        message,
        { source: CANDIDATE_SOURCE.NPM_REGISTRY, details },
      );
    }

    if (response?.status === FETCH_STATUS.TOO_LARGE) {
      // Worth its own reason: the registry's answer for one version is a small document, so an
      // oversized one is a more suspicious event than a truncated or unparsable one.
      return result(
        base,
        CANDIDATE_STATE.UNKNOWN,
        CANDIDATE_REASON.REGISTRY_RESPONSE_TOO_LARGE,
        'The npm registry answered with a document too large to be a version manifest',
        { source: CANDIDATE_SOURCE.NPM_REGISTRY, details },
      );
    }

    return result(
      base,
      CANDIDATE_STATE.UNKNOWN,
      CANDIDATE_REASON.REGISTRY_RESPONSE_MALFORMED,
      `The npm registry did not answer with a usable document${response?.message ? `: ${sanitize(response.message)}` : ''}`,
      { source: CANDIDATE_SOURCE.NPM_REGISTRY, details },
    );
  }

  const manifest = response.body;

  // The document is addressed by package name, so one naming a different package means the
  // answer is about something else entirely
  if (manifest === null || typeof manifest !== 'object'
    || (manifest.name !== undefined && manifest.name !== PACKAGE_NAME)) {
    return result(
      base,
      CANDIDATE_STATE.UNKNOWN,
      CANDIDATE_REASON.REGISTRY_RESPONSE_MALFORMED,
      `The npm registry answered with a document that does not describe ${PACKAGE_NAME}`,
      { source: CANDIDATE_SOURCE.NPM_REGISTRY, details },
    );
  }

  const rawVersion = manifest.version;
  const candidateVersion = parseVersion(rawVersion);

  // Everything a registry can put in this field that is not a version is refused here, at the
  // boundary. Downstream this string becomes `dashmate@<version>` on an npm command line,
  // where a specifier such as git+https://…, file:…, a .tgz URL or an npm: alias installs code
  // of the responder's choosing.
  if (candidateVersion === null) {
    return result(
      base,
      CANDIDATE_STATE.UNKNOWN,
      CANDIDATE_REASON.INVALID_VERSION,
      `The npm registry reported "${sanitize(rawVersion)}" where a version was expected`,
      { source: CANDIDATE_SOURCE.NPM_REGISTRY, details },
    );
  }

  return compareWithRunningVersion(base, candidateVersion, {
    source: CANDIDATE_SOURCE.NPM_REGISTRY,
    packageVersion: rawVersion,
    details,
  });
}

/**
 * Resolve which version the channel that installed this dashmate would install next.
 *
 * @typedef {Object} UpdateCandidate
 * @property {string} state - one of CANDIDATE_STATE; unknown is never "no update available"
 * @property {string} reason - one of CANDIDATE_REASON
 * @property {string} message - operator-readable explanation
 * @property {string} method - install method the candidate was resolved for
 * @property {string|null} source - one of CANDIDATE_SOURCE, when a channel was asked
 * @property {string} currentVersion - the version running now
 * @property {string|null} candidateVersion - validated application version the channel offers,
 *   for display; it is populated on states that must not be acted on, such as a downgrade
 * @property {string|null} installableVersion - the only field a package-manager argument may be
 *   built from; non-null exactly when the state is update-available
 * @property {string|null} packageVersion - the channel's own version string, as it reported it
 * @property {boolean|null} prerelease - whether the candidate is a prerelease
 * @property {string} updateInstructions - how this channel is upgraded by hand
 * @property {Object} details - what the channel reported, for diagnostics
 *
 * @param {Object} [options] - injection seams for tests; production callers pass nothing
 * @param {InstallMethodDetection} [options.installMethod] - a detection already taken
 * @param {string} [options.currentVersion]
 * @param {string} [options.platform]
 * @param {NodeJS.ProcessEnv} [options.env]
 * @param {boolean} [options.isRoot] - whether apt lists may be refreshed
 * @param {function(string, string[], Object=): Promise<string>} [options.runCommand]
 * @param {function(string): Promise<boolean>} [options.pathExists]
 * @param {function(string): Promise<Object>} [options.fetchJson]
 * @param {function(Object): Promise<InstallMethodDetection>} [options.detect]
 * @param {string} [options.aptSourcesPath]
 * @param {string} [options.registryUrl] - overrides both the environment and the default
 * @param {string} [options.distTag]
 * @return {Promise<UpdateCandidate>}
 */
export default async function resolveUpdateCandidate(options = {}) {
  // Enough of a result to answer with even if reading the options themselves fails, which is
  // what makes the never-throws property hold for every input rather than for well-formed ones
  const base = {
    method: INSTALL_METHODS.UNKNOWN,
    currentVersion: DASHMATE_VERSION,
    updateInstructions: '',
    installMethodEvidence: null,
  };

  try {
    const {
      currentVersion = DASHMATE_VERSION,
      platform = process.platform,
      env = process.env,
      runCommand = defaultRunCommand,
      pathExists = defaultPathExists,
      fetchJson = defaultFetchJson,
      detect = detectInstallMethod,
      aptSourcesPath = APT_SOURCES_PATH,
      registryUrl = null,
      distTag = NPM_DIST_TAG,
    } = options;

    base.currentVersion = currentVersion;

    // A caller that has already detected passes the detection in: detection costs a dpkg
    // probe, and running it twice would also allow the two answers to disagree.
    const installMethod = options.installMethod ?? await detect({ platform, env });

    base.method = installMethod.method;
    base.updateInstructions = installMethod.updateInstructions;
    // Carried through so a caller can tell an install with no channel from one whose signals
    // contradict each other, and report which installation it decided about
    base.installMethodEvidence = installMethod.evidence ?? null;

    // Refusing a downgrade means comparing against the running version, so an unreadable
    // running version leaves nothing to compare against. It cannot be assumed to be old.
    if (parseVersion(currentVersion) === null) {
      return result(
        base,
        CANDIDATE_STATE.UNKNOWN,
        CANDIDATE_REASON.CURRENT_VERSION_INVALID,
        `This dashmate reports version "${sanitize(currentVersion)}", which cannot be compared with a released version`,
        {},
      );
    }

    if (installMethod.method === INSTALL_METHODS.DEB) {
      // Every deb probe below is a POSIX absolute path, and a deb install cannot exist
      // elsewhere, but the guard keeps a mis-detection from spawning nonsense
      if (platform !== 'linux') {
        return result(
          base,
          CANDIDATE_STATE.UNKNOWN,
          CANDIDATE_REASON.UNSUPPORTED_PLATFORM,
          `A deb install cannot be resolved on ${platform}`,
          {},
        );
      }

      return await resolveFromApt(base, {
        runCommand, pathExists, aptSourcesPath, isRoot: options.isRoot ?? process.geteuid?.() === 0,
      });
    }

    if (installMethod.method === INSTALL_METHODS.NPM) {
      return await resolveFromNpmRegistry(base, {
        fetchJson, registryUrl, distTag, env,
      });
    }

    // Two different situations, and a caller acting on non-interactive consent must treat them
    // differently: a tarball, a pkg or a source checkout simply has no channel to ask, while
    // contradictory detection signals mean the host may hold more than one dashmate and an
    // upgrade could patch the one that is not running.
    if (isAmbiguousDetection(installMethod.evidence)) {
      return result(
        base,
        CANDIDATE_STATE.UNKNOWN,
        CANDIDATE_REASON.AMBIGUOUS_INSTALL_METHOD,
        `How this dashmate was installed could not be established: ${sanitize(installMethod.evidence?.reason)}`,
        {},
      );
    }

    // Whatever version exists elsewhere is not a version *this* installation would receive.
    // Saying so is the honest answer, and the caller has the manual instructions.
    return result(
      base,
      CANDIDATE_STATE.UNKNOWN,
      CANDIDATE_REASON.UNSUPPORTED_INSTALL_METHOD,
      `This dashmate was not installed through a channel that can be queried (${installMethod.method})`,
      {},
    );
  } catch (e) {
    // Defence in depth: every branch above contains its own failures, and a version check must
    // never be the reason a command fails. Nothing here may assume it was handed an Error
    // either — a rejected promise carries whatever the thrower chose, including null.
    return result(
      base,
      CANDIDATE_STATE.UNKNOWN,
      CANDIDATE_REASON.RESOLUTION_FAILED,
      `Could not resolve an update candidate: ${sanitize(e?.message)}`,
      {},
    );
  }
}
