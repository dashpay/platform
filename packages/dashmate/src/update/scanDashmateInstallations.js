import { promises as fsPromises } from 'fs';
import os from 'os';
import path from 'path';
import { fileURLToPath } from 'url';

/**
 * Discovery of every dashmate installation visible on the host.
 *
 * dashmate can end up installed more than once — a `.deb` unpacked into
 * /usr/lib/dashmate alongside an npm copy under a Node version manager, for
 * example. Only whichever one comes first on PATH actually runs, so upgrading
 * the other channel leaves the operator on unpatched code while both the
 * package manager and dashmate report success. This scan makes that visible.
 *
 * Two deliberate properties:
 *
 * - It never executes anything it finds. Every fact reported here comes from
 *   filesystem metadata and from reading an installation's own package.json,
 *   because running a binary discovered on PATH is precisely the risk the scan
 *   exists to describe.
 * - It never throws. Unreadable directories, dangling symlinks, permission
 *   errors and a missing PATH all degrade to a partial result with a warning,
 *   since this runs on diagnostic and update paths where it must not be the
 *   reason a command fails.
 *
 * It reports facts only. Whether a conflict is fatal or merely worth printing
 * is a policy decision left to the caller.
 */

const BIN_NAME = 'dashmate';
const PACKAGE_NAME = 'dashmate';

// The .deb unpacks the whole application here and points /usr/bin/dashmate at it.
const DEB_INSTALL_ROOT = '/usr/lib/dashmate';

// The oclif tarball and the macOS pkg both unpack under a prefix's lib directory.
const TARBALL_ROOT_SUFFIX = `${path.sep}lib${path.sep}${PACKAGE_NAME}`;

const HOMEBREW_PREFIXES = ['/opt/homebrew', '/usr/local/Cellar', '/home/linuxbrew/.linuxbrew'];

// Bounds. The scan runs on `status` and `doctor` paths, so it probes a fixed
// candidate list and reads at most one directory per version manager.
const MAX_NODE_VERSION_DIRS = 64;
const MAX_CANDIDATES = 512;
const MAX_PACKAGE_ROOT_DEPTH = 5;

// Errors that simply mean "nothing is installed here" rather than a problem
// worth telling the operator about.
const ABSENT_ERROR_CODES = ['ENOENT', 'ENOTDIR'];

export const ORIGIN = {
  DEB: 'deb',
  NPM_GLOBAL: 'npm-global',
  NVM: 'nvm',
  VOLTA: 'volta',
  ASDF: 'asdf',
  HOMEBREW: 'homebrew',
  TARBALL: 'tarball',
  SHIM: 'shim',
  UNKNOWN: 'unknown',
};

const DEFAULT_FS = {
  lstat: fsPromises.lstat,
  stat: fsPromises.stat,
  realpath: fsPromises.realpath,
  readFile: fsPromises.readFile,
  readdir: fsPromises.readdir,
};

/**
 * @typedef {Object} InstallationEntrypoint
 * @property {string} path - location the installation was found through, as probed
 * @property {string} realPath - the same location with every symlink resolved
 * @property {'path'|'known-root'} source - how the location was discovered
 * @property {number|null} pathIndex - position in PATH, or null when not on PATH
 */

/**
 * @typedef {Object} DashmateInstallation
 * @property {string} path - resolved package root, or the resolved binary when no root was found
 * @property {string|null} root - resolved package root
 * @property {InstallationEntrypoint[]} entrypoints - every location resolving to this installation
 * @property {string} origin - one of ORIGIN
 * @property {string|null} version - version read from the installation's package.json
 * @property {boolean} onPath - reachable through PATH
 * @property {number|null} pathIndex - lowest PATH position it is reachable at
 * @property {boolean} active - PATH resolves this installation, i.e. this is what runs
 * @property {boolean} running - this is the installation executing the current process
 * @property {boolean} isShim - a version manager dispatcher rather than an installation
 */

/**
 * @typedef {Object} ScanWarning
 * @property {string} code - errno-style code, or NO_PATH / INVALID_MANIFEST / SCAN_FAILED
 * @property {string|null} path - location the warning is about
 * @property {string} message - operator-readable explanation
 */

/**
 * @typedef {Object} DashmateInstallationScan
 * @property {boolean} supported - false when the platform cannot be scanned
 * @property {string} platform - process.platform the scan ran against
 * @property {string|null} unsupportedReason - why the platform cannot be scanned
 * @property {DashmateInstallation[]} installations - deduplicated, PATH order first
 * @property {string|null} activePath - the installation PATH resolves
 * @property {string|null} runningPath - the installation executing the current process
 * @property {boolean} hasConflict - more than one real installation was found
 * @property {boolean} truncated - a bound was hit, so the list may be incomplete
 * @property {ScanWarning[]} warnings - everything that could not be inspected
 * @property {string} scannedAt - ISO timestamp, for state files that cache the result
 */

/**
 * @param {string} location
 * @param {string} directory
 * @return {boolean}
 */
function isUnder(location, directory) {
  return location.startsWith(`${directory}${path.sep}`);
}

/**
 * @param {Object} env
 * @param {string} homeDir
 * @return {string}
 */
function nvmDir(env, homeDir) {
  return env.NVM_DIR || path.join(homeDir, '.nvm');
}

/**
 * @param {Object} env
 * @param {string} homeDir
 * @return {string}
 */
function voltaHome(env, homeDir) {
  return env.VOLTA_HOME || path.join(homeDir, '.volta');
}

/**
 * @param {Object} env
 * @param {string} homeDir
 * @return {string}
 */
function asdfDir(env, homeDir) {
  return env.ASDF_DATA_DIR || env.ASDF_DIR || path.join(homeDir, '.asdf');
}

/**
 * Directories holding a file named `dashmate` that dispatches to an
 * installation rather than being one.
 *
 * @param {Object} env
 * @param {string} homeDir
 * @return {string[]}
 */
function shimDirectories(env, homeDir) {
  return [
    path.join(voltaHome(env, homeDir), 'bin'),
    path.join(asdfDir(env, homeDir), 'shims'),
  ];
}

/**
 * @param {Object} found
 * @param {Object} context
 * @return {boolean}
 */
function isShimLocation(found, context) {
  const directories = shimDirectories(context.env, context.homeDir);

  return directories.includes(path.dirname(found.realPath))
    || directories.includes(path.dirname(found.candidate.target));
}

/**
 * Name the packaging channel an installation came from, from its layout. This
 * is a best effort label for the operator, never a decision input.
 *
 * @param {string} location - the installation's package root, or its binary
 * @param {Object} found
 * @param {Object} context
 * @return {string}
 */
function classifyOrigin(location, found, context) {
  const { env, homeDir } = context;

  if (found.root === null && isShimLocation(found, context)) {
    return ORIGIN.SHIM;
  }

  if (location === DEB_INSTALL_ROOT || isUnder(location, DEB_INSTALL_ROOT)) {
    return ORIGIN.DEB;
  }

  if (isUnder(location, nvmDir(env, homeDir))) {
    return ORIGIN.NVM;
  }

  if (isUnder(location, voltaHome(env, homeDir))) {
    return ORIGIN.VOLTA;
  }

  if (isUnder(location, asdfDir(env, homeDir))) {
    return ORIGIN.ASDF;
  }

  if (HOMEBREW_PREFIXES.some((prefix) => isUnder(location, prefix))) {
    return ORIGIN.HOMEBREW;
  }

  if (location.includes(`${path.sep}node_modules${path.sep}${PACKAGE_NAME}`)) {
    return ORIGIN.NPM_GLOBAL;
  }

  if (location.endsWith(TARBALL_ROOT_SUFFIX)) {
    return ORIGIN.TARBALL;
  }

  return ORIGIN.UNKNOWN;
}

/**
 * Read a directory's package.json, but only accept it when it is dashmate's.
 *
 * @param {Object} fs
 * @param {string} directory
 * @param {ScanWarning[]} warnings - collected, so a partial scan still reports what it saw
 * @return {Promise<{version: string|null}|null>}
 */
async function readDashmateManifest(fs, directory, warnings) {
  const manifestPath = path.join(directory, 'package.json');

  let raw;

  try {
    raw = await fs.readFile(manifestPath, 'utf8');
  } catch (e) {
    if (!ABSENT_ERROR_CODES.includes(e.code)) {
      warnings.push({
        code: e.code ?? 'UNKNOWN',
        path: manifestPath,
        message: `Cannot read ${manifestPath}: ${e.message}`,
      });
    }

    return null;
  }

  let parsed;

  try {
    parsed = JSON.parse(raw);
  } catch (e) {
    warnings.push({
      code: 'INVALID_MANIFEST',
      path: manifestPath,
      message: `Cannot parse ${manifestPath}: ${e.message}`,
    });

    return null;
  }

  if (parsed === null || typeof parsed !== 'object' || parsed.name !== PACKAGE_NAME) {
    return null;
  }

  return { version: typeof parsed.version === 'string' ? parsed.version : null };
}

/**
 * Walk up from a binary towards the package root that owns it.
 *
 * @param {Object} fs
 * @param {string} startDirectory
 * @param {ScanWarning[]} warnings
 * @return {Promise<{path: string, version: string|null}|null>}
 */
async function findPackageRoot(fs, startDirectory, warnings) {
  let directory = startDirectory;

  for (let depth = 0; depth < MAX_PACKAGE_ROOT_DEPTH; depth += 1) {
    const manifest = await readDashmateManifest(fs, directory, warnings);

    if (manifest !== null) {
      return { path: directory, version: manifest.version };
    }

    const parent = path.dirname(directory);

    if (parent === directory) {
      break;
    }

    directory = parent;
  }

  return null;
}

/**
 * @param {Object} fs
 * @param {string} target
 * @param {ScanWarning[]} warnings
 * @return {Promise<{realPath: string}|null>}
 */
async function resolveRealPath(fs, target, warnings) {
  let linkStats;

  try {
    linkStats = await fs.lstat(target);
  } catch (e) {
    if (!ABSENT_ERROR_CODES.includes(e.code)) {
      warnings.push({
        code: e.code ?? 'UNKNOWN',
        path: target,
        message: `Cannot inspect ${target}: ${e.message}`,
      });
    }

    return null;
  }

  try {
    return { realPath: await fs.realpath(target) };
  } catch (e) {
    warnings.push({
      code: e.code ?? 'UNKNOWN',
      path: target,
      message: linkStats.isSymbolicLink() && ABSENT_ERROR_CODES.includes(e.code)
        ? `Ignoring ${target}: it is a symlink whose target no longer exists`
        : `Cannot resolve ${target}: ${e.message}`,
    });

    return null;
  }
}

/**
 * Resolve one candidate location into an installation, or null when there is
 * nothing usable there.
 *
 * @param {Object} fs
 * @param {Object} candidate
 * @param {ScanWarning[]} warnings
 * @return {Promise<Object|null>}
 */
async function probeCandidate(fs, candidate, warnings) {
  const resolved = await resolveRealPath(fs, candidate.target, warnings);

  if (resolved === null) {
    return null;
  }

  const { realPath } = resolved;

  let stats;

  try {
    stats = await fs.stat(realPath);
  } catch (e) {
    warnings.push({
      code: e.code ?? 'UNKNOWN',
      path: candidate.target,
      message: `Cannot inspect ${realPath}: ${e.message}`,
    });

    return null;
  }

  if (candidate.kind === 'root') {
    if (!stats.isDirectory()) {
      return null;
    }

    const manifest = await readDashmateManifest(fs, realPath, warnings);

    if (manifest === null) {
      return null;
    }

    return {
      candidate, realPath, root: realPath, version: manifest.version,
    };
  }

  if (!stats.isFile()) {
    return null;
  }

  const root = await findPackageRoot(fs, path.dirname(realPath), warnings);

  return {
    candidate,
    realPath,
    root: root === null ? null : root.path,
    version: root === null ? null : root.version,
  };
}

/**
 * @param {Object} fs
 * @param {string} directory
 * @param {ScanWarning[]} warnings
 * @return {Promise<{names: string[], truncated: boolean}>}
 */
async function readVersionDirectory(fs, directory, warnings) {
  let entries;

  try {
    entries = await fs.readdir(directory);
  } catch (e) {
    if (!ABSENT_ERROR_CODES.includes(e.code)) {
      warnings.push({
        code: e.code ?? 'UNKNOWN',
        path: directory,
        message: `Cannot list ${directory}: ${e.message}`,
      });
    }

    return { names: [], truncated: false };
  }

  const names = entries
    .map((entry) => (typeof entry === 'string' ? entry : entry.name))
    .filter((name) => typeof name === 'string' && !name.startsWith('.'))
    .sort();

  return {
    names: names.slice(0, MAX_NODE_VERSION_DIRS),
    truncated: names.length > MAX_NODE_VERSION_DIRS,
  };
}

/**
 * Package roots each packaging channel is known to use.
 *
 * @param {Object} env
 * @param {string} homeDir
 * @return {string[]}
 */
function packageRootCandidates(env, homeDir) {
  return [
    // The .deb.
    DEB_INSTALL_ROOT,
    // The oclif tarball and the macOS pkg.
    path.join('/usr/local', 'lib', PACKAGE_NAME),
    path.join('/opt', PACKAGE_NAME),
    // Volta keeps a package's own image rather than a shared global prefix.
    path.join(
      voltaHome(env, homeDir),
      'tools',
      'image',
      'packages',
      PACKAGE_NAME,
      'lib',
      'node_modules',
      PACKAGE_NAME,
    ),
  ];
}

/**
 * Prefixes whose `lib/node_modules` an npm global install could live under.
 *
 * @param {Object} env
 * @param {string} homeDir
 * @param {string} execPath
 * @return {string[]}
 */
function prefixCandidates(env, homeDir, execPath) {
  const prefixes = [
    '/usr',
    '/usr/local',
    '/opt/homebrew',
    env.npm_config_prefix,
    env.NPM_CONFIG_PREFIX,
    path.join(homeDir, '.npm-global'),
    path.join(homeDir, '.local'),
    path.join(homeDir, '.node'),
  ];

  // The prefix of the Node running this process, which is where a version
  // manager's currently selected global installs live.
  if (typeof execPath === 'string' && execPath.length > 0) {
    prefixes.push(path.dirname(path.dirname(execPath)));
  }

  return prefixes.filter((prefix) => typeof prefix === 'string' && prefix.length > 0);
}

/**
 * Build the fixed list of locations to probe: every PATH entry, then the roots
 * each packaging channel is known to use.
 *
 * @param {Object} context
 * @param {ScanWarning[]} warnings
 * @return {Promise<{candidates: Object[], truncated: boolean}>}
 */
async function collectCandidates(context, warnings) {
  const {
    env, fs, homeDir, execPath, cwd,
  } = context;

  const candidates = [];
  const seen = new Set();

  let truncated = false;

  const add = (target, kind, source, pathIndex = null) => {
    if (typeof target !== 'string' || target.length === 0) {
      return;
    }

    if (candidates.length >= MAX_CANDIDATES) {
      truncated = true;

      return;
    }

    const key = `${kind}:${target}`;

    if (seen.has(key)) {
      return;
    }

    seen.add(key);
    candidates.push({
      target, kind, source, pathIndex,
    });
  };

  // PATH, in order — the Node equivalent of `which -a dashmate`.
  const rawPath = env.PATH;

  if (typeof rawPath !== 'string' || rawPath.length === 0) {
    warnings.push({
      code: 'NO_PATH',
      path: null,
      message: 'PATH is empty or unset, so installations reachable through PATH could not be enumerated',
    });
  } else {
    const directories = rawPath.split(path.delimiter);

    for (let index = 0; index < directories.length; index += 1) {
      const directory = directories[index];

      if (directory.length > 0) {
        add(path.join(path.resolve(cwd, directory), BIN_NAME), 'bin', 'path', index);
      }
    }
  }

  for (const root of packageRootCandidates(env, homeDir)) {
    add(root, 'root', 'known-root');
  }

  for (const prefix of prefixCandidates(env, homeDir, execPath)) {
    add(path.join(prefix, 'lib', 'node_modules', PACKAGE_NAME), 'root', 'known-root');
    add(path.join(prefix, 'bin', BIN_NAME), 'bin', 'known-root');
  }

  for (const shimDirectory of shimDirectories(env, homeDir)) {
    add(path.join(shimDirectory, BIN_NAME), 'bin', 'known-root');
  }

  // Version managers keep one Node installation per version, each with its own
  // global node_modules, so a stale dashmate can sit under a version the
  // operator no longer uses.
  const versionDirectories = [
    path.join(nvmDir(env, homeDir), 'versions', 'node'),
    path.join(asdfDir(env, homeDir), 'installs', 'nodejs'),
  ];

  const versionListings = await Promise.all(
    versionDirectories.map((directory) => readVersionDirectory(fs, directory, warnings)),
  );

  for (let index = 0; index < versionDirectories.length; index += 1) {
    const versionsDirectory = versionDirectories[index];
    const listing = versionListings[index];

    truncated = truncated || listing.truncated;

    for (const version of listing.names) {
      add(path.join(versionsDirectory, version, 'lib', 'node_modules', PACKAGE_NAME), 'root', 'known-root');
      add(path.join(versionsDirectory, version, 'bin', BIN_NAME), 'bin', 'known-root');
    }
  }

  return { candidates, truncated };
}

/**
 * The installation the current process is executing from, so a caller can tell
 * the operator whether the dashmate they just ran is the one PATH resolves.
 *
 * @param {Object} context
 * @param {ScanWarning[]} warnings
 * @return {Promise<string|null>}
 */
async function findRunningRoot(context, warnings) {
  const { fs, selfPath } = context;

  if (typeof selfPath !== 'string' || selfPath.length === 0) {
    return null;
  }

  const resolved = await resolveRealPath(fs, selfPath, warnings);

  if (resolved === null) {
    return null;
  }

  const root = await findPackageRoot(fs, path.dirname(resolved.realPath), warnings);

  return root === null ? null : root.path;
}

/**
 * Group probed locations into distinct installations.
 *
 * @param {Object[]} probed
 * @param {string|null} runningRoot
 * @param {Object} context
 * @return {DashmateInstallation[]}
 */
function groupInstallations(probed, runningRoot, context) {
  const byLocation = new Map();

  for (const found of probed) {
    if (found === null) {
      continue;
    }

    // Symlinks are resolved before this point, so an installation reachable
    // through several paths collapses into one entry instead of looking like
    // a conflict with itself.
    const key = found.root ?? found.realPath;

    let installation = byLocation.get(key);

    if (installation === undefined) {
      installation = {
        path: key,
        root: found.root,
        entrypoints: [],
        origin: classifyOrigin(key, found, context),
        version: found.version,
        onPath: false,
        pathIndex: null,
        active: false,
        running: found.root !== null && found.root === runningRoot,
        isShim: found.root === null && isShimLocation(found, context),
      };

      byLocation.set(key, installation);
    }

    installation.entrypoints.push({
      path: found.candidate.target,
      realPath: found.realPath,
      source: found.candidate.source,
      pathIndex: found.candidate.pathIndex,
    });

    const { pathIndex } = found.candidate;

    if (pathIndex !== null && (installation.pathIndex === null
      || pathIndex < installation.pathIndex)) {
      installation.pathIndex = pathIndex;
      installation.onPath = true;
    }
  }

  const installations = [...byLocation.values()];

  for (const installation of installations) {
    installation.entrypoints.sort((a, b) => a.path.localeCompare(b.path));
  }

  // PATH order first, because that is the order a shell resolves them in.
  installations.sort((a, b) => {
    if (a.pathIndex !== b.pathIndex) {
      if (a.pathIndex === null) {
        return 1;
      }

      if (b.pathIndex === null) {
        return -1;
      }

      return a.pathIndex - b.pathIndex;
    }

    return a.path.localeCompare(b.path);
  });

  return installations;
}

/**
 * @param {Object} context
 * @return {Promise<Object>}
 */
async function scan(context) {
  const warnings = [];

  const { candidates, truncated } = await collectCandidates(context, warnings);

  const [probed, runningRoot] = await Promise.all([
    Promise.all(candidates.map((candidate) => probeCandidate(context.fs, candidate, warnings))),
    findRunningRoot(context, warnings),
  ]);

  const installations = groupInstallations(probed, runningRoot, context);

  const active = installations.find((installation) => installation.onPath) ?? null;

  if (active !== null) {
    active.active = true;
  }

  const running = installations.find((installation) => installation.running) ?? null;

  return {
    installations,
    activePath: active === null ? null : active.path,
    runningPath: running === null ? runningRoot : running.path,
    // A version manager shim dispatches to an installation rather than being
    // one, so counting it would report a conflict where there is a single
    // install.
    hasConflict: installations.filter((installation) => !installation.isShim).length > 1,
    truncated,
    warnings,
  };
}

/**
 * Enumerate every dashmate installation discoverable on this host.
 *
 * @param {Object} [options] - seams, overridden by tests; defaults describe the real host
 * @param {string} [options.platform]
 * @param {Object} [options.env]
 * @param {Object} [options.fs] - lstat, stat, realpath, readFile, readdir
 * @param {string} [options.homeDir]
 * @param {string} [options.execPath] - the Node binary running this process
 * @param {string} [options.selfPath] - the file this module was loaded from
 * @param {string} [options.cwd] - resolves relative PATH entries
 * @return {Promise<DashmateInstallationScan>}
 */
export default async function scanDashmateInstallations(options = {}) {
  const {
    platform = process.platform,
    env = process.env,
    fs = DEFAULT_FS,
    homeDir = os.homedir(),
    execPath = process.execPath,
    selfPath = fileURLToPath(import.meta.url),
    cwd = process.cwd(),
  } = options;

  const empty = {
    supported: true,
    platform,
    unsupportedReason: null,
    installations: [],
    activePath: null,
    runningPath: null,
    hasConflict: false,
    truncated: false,
    warnings: [],
    scannedAt: new Date().toISOString(),
  };

  // Windows resolves executables through %PATH% with .cmd shims and lays version
  // managers out differently, so the POSIX probes above would report nonsense.
  // Say so explicitly instead, so a caller can tell "no conflict" apart from
  // "not checked" and does not record an unsupported platform as a defect.
  if (platform === 'win32') {
    return {
      ...empty,
      supported: false,
      unsupportedReason: 'Scanning for conflicting dashmate installations is not supported on Windows',
    };
  }

  try {
    const scanned = await scan({
      env, fs, homeDir, execPath, selfPath, cwd,
    });

    return { ...empty, ...scanned };
  } catch (e) {
    // Defence in depth: the helpers above already contain their own failures,
    // and this command must never fail because the scan did.
    return {
      ...empty,
      warnings: [{
        code: 'SCAN_FAILED',
        path: null,
        message: `Could not complete the installation scan: ${e.message}`,
      }],
    };
  }
}
