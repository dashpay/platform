import childProcess from 'child_process';
import fs from 'fs';
import path from 'path';
import { promisify } from 'util';

import { PACKAGE_ROOT_DIR } from '../constants.js';

const execFileAsync = promisify(childProcess.execFile);

/**
 * How the running dashmate was put on disk.
 *
 * `tarball` covers both the standalone tarball and the macOS pkg: the pkg payload is the
 * tarball build, so they are indistinguishable at runtime and are updated the same way
 * (download and re-install by hand).
 */
export const INSTALL_METHOD_DEB = 'deb';
export const INSTALL_METHOD_TARBALL = 'tarball';
export const INSTALL_METHOD_NPM = 'npm';
export const INSTALL_METHOD_UNKNOWN = 'unknown';

export const INSTALL_METHODS = {
  DEB: INSTALL_METHOD_DEB,
  TARBALL: INSTALL_METHOD_TARBALL,
  NPM: INSTALL_METHOD_NPM,
  UNKNOWN: INSTALL_METHOD_UNKNOWN,
};

/**
 * Environment variables the oclif bin wrappers export.
 *
 * The deb wrapper exports UPDATE_INSTRUCTIONS and never assigns BINPATH; the tarball/pkg
 * wrapper assigns BINPATH and never mentions UPDATE_INSTRUCTIONS; an npm install runs
 * `bin/run.js` through npm's own shim and therefore sets neither.
 *
 * Both variables are `export`ed, so every child process dashmate spawns inherits them.
 * A spawned child must not re-derive the install method from its own environment — it would
 * report whatever installed its *parent*. Pass a detected result down explicitly instead.
 */
const UPDATE_INSTRUCTIONS_ENV = 'DASHMATE_UPDATE_INSTRUCTIONS';
const BINPATH_ENV = 'DASHMATE_BINPATH';

const DEB_PACKAGE_NAME = 'dashmate';
const DEB_ROOT_DIR = '/usr/lib/dashmate';
const DEB_BIN_PATH = '/usr/lib/dashmate/bin/dashmate';
const DPKG_QUERY_PATH = '/usr/bin/dpkg-query';

const NPM_BIN_NAME = 'dashmate';

const RELEASES_URL = 'https://github.com/dashpay/platform/releases/latest';

/**
 * Upgrade commands we print to the operator.
 *
 * Deliberately our own constants rather than the text of DASHMATE_UPDATE_INSTRUCTIONS:
 * the environment is inherited and writable by whoever launched us, and an instruction
 * string is something an operator is likely to paste into a root shell.
 */
const UPDATE_INSTRUCTIONS = {
  [INSTALL_METHOD_DEB]: 'sudo apt update && sudo apt install dashmate',
  [INSTALL_METHOD_NPM]: 'npm install -g dashmate@latest',
  [INSTALL_METHOD_TARBALL]: `Download the latest release from ${RELEASES_URL} and re-run the installer`,
  [INSTALL_METHOD_UNKNOWN]: `Update dashmate the same way you installed it, or download the latest release from ${RELEASES_URL}`,
};

/**
 * Environment handed to every probe.
 *
 * An allowlist, not a filtered copy of process.env: a probe that inherits PATH, LD_PRELOAD
 * or NODE_OPTIONS from the caller can be redirected into arbitrary code, and the answer this
 * module returns decides what gets installed later.
 */
const PROBE_ENV = Object.freeze({
  PATH: '/usr/sbin:/usr/bin:/sbin:/bin',
  // dpkg localises its output; C keeps the parsed lines stable
  LC_ALL: 'C',
  LANG: 'C',
});

const PROBE_TIMEOUT_MS = 5000;
const PROBE_MAX_BUFFER = 1024 * 1024;

/**
 * Whether a path is the given directory or lives under it.
 *
 * Compares resolved segments rather than string prefixes, so a sibling such as
 * `/usr/lib/dashmate-old` does not count as being under `/usr/lib/dashmate`.
 *
 * @param {string} parentDir
 * @param {string} childPath
 * @return {boolean}
 */
function isPathWithin(parentDir, childPath) {
  const relative = path.relative(parentDir, childPath);

  return !relative.startsWith('..') && !path.isAbsolute(relative);
}

/**
 * @param {string} filePath
 * @return {Promise<boolean>}
 */
async function defaultPathExists(filePath) {
  try {
    await fs.promises.access(filePath, fs.constants.F_OK);

    return true;
  } catch {
    return false;
  }
}

/**
 * Run a probe with an argv array and an allowlisted environment.
 *
 * execFile with an argv array, never exec and never shell: true, so nothing in a path or an
 * environment variable can become a shell token.
 *
 * @param {string} file - absolute path to the executable
 * @param {string[]} args
 * @return {Promise<string>} stdout
 */
async function defaultRunProbe(file, args) {
  const { stdout } = await execFileAsync(file, args, {
    env: { ...PROBE_ENV },
    timeout: PROBE_TIMEOUT_MS,
    maxBuffer: PROBE_MAX_BUFFER,
    windowsHide: true,
  });

  return stdout;
}

/**
 * Ask dpkg which package owns a path.
 *
 * @param {function} runProbe
 * @param {string} ownedPath
 * @return {Promise<boolean|null>} true or false when dpkg answered, null when it could not be asked
 */
async function isOwnedByDebPackage(runProbe, ownedPath) {
  let stdout;

  try {
    stdout = await runProbe(DPKG_QUERY_PATH, ['-S', ownedPath]);
  } catch {
    // dpkg-query exits non-zero when nothing matches, but it also fails when dpkg is absent
    // or its database is unreadable. Both land here, so the honest answer is "do not know".
    return null;
  }

  // Lines look like "dashmate: /usr/lib/dashmate/package.json", with a comma-separated
  // package list when several packages ship the same path.
  return stdout
    .split('\n')
    .some((line) => {
      const [packages] = line.split(': ');

      return (packages ?? '')
        .split(', ')
        .some((name) => name.trim() === DEB_PACKAGE_NAME);
    });
}

/**
 * Ask dpkg where the package put its executable.
 *
 * @param {function} runProbe
 * @return {Promise<string|null>}
 */
async function getDebBinPath(runProbe) {
  let stdout;

  try {
    stdout = await runProbe(DPKG_QUERY_PATH, ['-L', DEB_PACKAGE_NAME]);
  } catch {
    return null;
  }

  return stdout
    .split('\n')
    .map((line) => line.trim())
    .find((line) => isPathWithin(DEB_ROOT_DIR, line)
      && path.basename(line) === DEB_PACKAGE_NAME
      && path.basename(path.dirname(line)) === 'bin') ?? null;
}

/**
 * Locate the npm global prefix that owns a package root.
 *
 * A global install lives at `<prefix>/lib/node_modules/dashmate` for npm, nvm, asdf, volta
 * and Homebrew alike, so the prefix is derived from the root path instead of shelling out to
 * `npm prefix -g`: npm is not at a fixed absolute path, and resolving it through PATH is
 * exactly the lookup this module must not depend on.
 *
 * @param {string} packageRootDir
 * @return {string|null}
 */
function getNpmGlobalPrefix(packageRootDir) {
  const nodeModulesDir = path.dirname(packageRootDir);

  // dashmate is unscoped, so its root sits directly under node_modules
  if (path.basename(nodeModulesDir) !== 'node_modules') {
    return null;
  }

  const libDir = path.dirname(nodeModulesDir);

  if (path.basename(libDir) !== 'lib') {
    return null;
  }

  return path.dirname(libDir);
}

/**
 * @param {string} packageRootDir
 * @return {boolean}
 */
function isUnderNodeModules(packageRootDir) {
  return packageRootDir.split(path.sep).includes('node_modules');
}

/**
 * @param {string} method
 * @param {object} [properties]
 * @return {InstallMethodDetection}
 */
function result(method, properties = {}) {
  return {
    method,
    binPath: null,
    updateInstructions: UPDATE_INSTRUCTIONS[method],
    ...properties,
  };
}

/**
 * Detect how the running dashmate was installed.
 *
 * Never throws: an environment it cannot read returns `unknown`, because this runs on CLI
 * startup paths where an exception would take down unrelated commands.
 *
 * @typedef {object} InstallMethodDetection
 * @property {string} method - one of INSTALL_METHODS
 * @property {string|null} binPath - absolute path of the executable to re-exec after an
 *   upgrade, or null when it could not be established. Never resolved through PATH, so a
 *   shadowing install cannot turn an upgrade into an execution primitive.
 * @property {string} updateInstructions - human-readable upgrade command
 * @property {object} evidence - what the detection saw, for diagnostics
 *
 * @param {object} [options] - injection seam for tests; production callers pass nothing
 * @param {NodeJS.ProcessEnv} [options.env]
 * @param {string} [options.platform]
 * @param {string} [options.packageRootDir]
 * @param {function(string, string[]): Promise<string>} [options.runProbe]
 * @param {function(string): Promise<boolean>} [options.pathExists]
 * @return {Promise<InstallMethodDetection>}
 */
export default async function detectInstallMethod({
  env = process.env,
  platform = process.platform,
  packageRootDir = PACKAGE_ROOT_DIR,
  runProbe = defaultRunProbe,
  pathExists = defaultPathExists,
} = {}) {
  const evidence = {
    platform,
    packageRootDir,
    hasUpdateInstructionsEnv: false,
    hasBinPathEnv: false,
    debPackageConfirmed: null,
    npmGlobalPrefix: null,
    reason: '',
  };

  try {
    // Windows ships no dpkg and no POSIX layout. Every probe below is POSIX-only, so rather
    // than shelling out to commands that cannot exist there, Windows degrades to a manual
    // update.
    if (platform === 'win32') {
      evidence.reason = 'Windows has no package manager we can drive';

      return result(INSTALL_METHOD_UNKNOWN, { evidence });
    }

    const updateInstructionsEnv = env[UPDATE_INSTRUCTIONS_ENV];
    const binPathEnv = env[BINPATH_ENV];

    evidence.hasUpdateInstructionsEnv = typeof updateInstructionsEnv === 'string'
      && updateInstructionsEnv.length > 0;
    evidence.hasBinPathEnv = typeof binPathEnv === 'string' && binPathEnv.length > 0;

    // The filesystem layout is the strongest deb signal and holds even when the bin wrapper
    // was bypassed, so it is checked before the environment.
    if (platform === 'linux' && isPathWithin(DEB_ROOT_DIR, packageRootDir)) {
      evidence.debPackageConfirmed = await isOwnedByDebPackage(runProbe, packageRootDir);

      // A definite "dpkg does not own this" means something else was unpacked into the deb's
      // directory. Driving apt at it would upgrade a package that is not what is running.
      // dpkg being unavailable is a different answer, and the layout still stands on its own.
      if (evidence.debPackageConfirmed === false) {
        evidence.reason = `package root is under ${DEB_ROOT_DIR} but dpkg does not report it as owned by ${DEB_PACKAGE_NAME}`;

        return result(INSTALL_METHOD_UNKNOWN, { evidence });
      }

      evidence.reason = `package root is under ${DEB_ROOT_DIR}`;

      return result(INSTALL_METHOD_DEB, {
        binPath: await getDebBinPath(runProbe) ?? DEB_BIN_PATH,
        evidence,
      });
    }

    if (evidence.hasUpdateInstructionsEnv && !evidence.hasBinPathEnv) {
      // The deb wrapper's fingerprint without the deb layout. The variable is inherited by
      // every child, so trust it only if dpkg agrees this package root is the deb's.
      if (platform !== 'linux') {
        evidence.reason = `${UPDATE_INSTRUCTIONS_ENV} is set on a non-Linux platform, so it was inherited rather than exported by a deb install`;

        return result(INSTALL_METHOD_UNKNOWN, { evidence });
      }

      evidence.debPackageConfirmed = await isOwnedByDebPackage(runProbe, packageRootDir);

      if (evidence.debPackageConfirmed !== true) {
        evidence.reason = `${UPDATE_INSTRUCTIONS_ENV} is set but dpkg does not report ${packageRootDir} as owned by ${DEB_PACKAGE_NAME}`;

        return result(INSTALL_METHOD_UNKNOWN, { evidence });
      }

      evidence.reason = `dpkg reports ${packageRootDir} as owned by ${DEB_PACKAGE_NAME}`;

      return result(INSTALL_METHOD_DEB, {
        binPath: await getDebBinPath(runProbe) ?? DEB_BIN_PATH,
        evidence,
      });
    }

    if (evidence.hasBinPathEnv && !evidence.hasUpdateInstructionsEnv) {
      // A relative BINPATH cannot be re-executed safely: it would resolve against whatever
      // working directory the caller happened to have.
      const binPath = path.isAbsolute(binPathEnv) && await pathExists(binPathEnv)
        ? binPathEnv
        : null;

      evidence.reason = binPath === null
        ? `${BINPATH_ENV} is set but does not point at an existing absolute path`
        : `${BINPATH_ENV} is set by the tarball bin wrapper`;

      return result(INSTALL_METHOD_TARBALL, { binPath, evidence });
    }

    if (evidence.hasBinPathEnv && evidence.hasUpdateInstructionsEnv) {
      // No bin wrapper sets both. Something else in the process tree exported one of them,
      // so neither can be believed.
      evidence.reason = `both ${BINPATH_ENV} and ${UPDATE_INSTRUCTIONS_ENV} are set, which no bin wrapper does`;

      return result(INSTALL_METHOD_UNKNOWN, { evidence });
    }

    if (!isUnderNodeModules(packageRootDir)) {
      // No bin wrapper ran and we are not inside a dependency tree: a source checkout, a
      // yarn workspace, or something else with no upgrade path we can drive.
      evidence.reason = 'no bin wrapper signals and the package root is not inside node_modules';

      return result(INSTALL_METHOD_UNKNOWN, { evidence });
    }

    evidence.npmGlobalPrefix = getNpmGlobalPrefix(packageRootDir);
    evidence.reason = 'package root is inside node_modules and no bin wrapper signals are present';

    let binPath = null;

    if (evidence.npmGlobalPrefix !== null) {
      const candidate = path.join(evidence.npmGlobalPrefix, 'bin', NPM_BIN_NAME);

      binPath = await pathExists(candidate) ? candidate : null;
    }

    return result(INSTALL_METHOD_NPM, { binPath, evidence });
  } catch (e) {
    evidence.reason = `detection failed: ${e.message}`;

    return result(INSTALL_METHOD_UNKNOWN, { evidence });
  }
}
