import childProcess from 'child_process';
import { promises as fsPromises, constants as fsConstants } from 'fs';
import path from 'path';
import { promisify } from 'util';
import semver from 'semver';

import AbstractError from '../errors/AbstractError.js';
import { INSTALL_METHOD_DEB, INSTALL_METHOD_NPM } from './detectInstallMethod.js';

/**
 * Replacement of the dashmate package itself, through the package manager that installed it.
 *
 * These are the first subprocesses dashmate runs with root privileges, on hosts running
 * masternodes, so every input is treated as hostile and every invocation is built from
 * constants plus values that were re-validated here:
 *
 * - `execFile` with an argv array only. No `exec`, no `shell`, no command string is ever
 *   assembled, so nothing in a version, a path or an environment variable can become a shell
 *   token.
 * - Absolute paths for every executable. A package manager resolved through PATH would turn a
 *   shadowing install into an execution primitive, which is the opposite of what an upgrade
 *   is for.
 * - An explicit environment allowlist rather than a filtered copy of the caller's environment.
 *   `APT_CONFIG` alone can relocate the sources file or set `APT::Get::AllowUnauthenticated`,
 *   which would silently disable the signature verification the whole update design rests on;
 *   `NODE_OPTIONS`, `NODE_PATH` and `LD_*` are code injection into anything we spawn. A
 *   denylist would have to be kept in step with every tool we call, so nothing is inherited.
 * - `sudo -n` for every privileged invocation. A sudo timestamp acquired before the run can
 *   expire while apt is downloading, and `-n` turns that into a loud failure instead of a
 *   password prompt hidden behind a progress renderer.
 *
 * This module decides nothing about *which* version to install. It is given one and installs
 * exactly that, after validating it again: this is the last place a value becomes an argv
 * token, and npm re-parses that token as a package specifier which also accepts
 * `git+https://`, `file:` and `npm:` aliases.
 *
 * It also does not activate the node. Migrating the config, pulling images and restarting
 * services happen afterwards, in the newly installed binary.
 *
 * Every failure is a `PackageUpgradeError` carrying the step it failed at, the package
 * manager's own output, and a `packageState` that is always one of PACKAGE_STATE. That field is
 * total on purpose: the caller decides from it whether to carry on, and an unrecognised value
 * landing on the optimistic branch would mean activating a node after a failed root-privileged
 * operation. A rejection from this module that is *not* a PackageUpgradeError is a bug, and a
 * caller seeing one must treat it as PACKAGE_STATE.UNCERTAIN.
 *
 * Success is only ever returned after the installed version has been read back and compared
 * against the exact version that was approved: apt exits 0 for "0 upgraded" as readily as for a
 * real upgrade, so an exit code proves nothing on its own.
 */

const execFileAsync = promisify(childProcess.execFile);

const SUDO_PATH = '/usr/bin/sudo';
const ENV_PATH = '/usr/bin/env';
const APT_GET_PATH = '/usr/bin/apt-get';
const DPKG_PATH = '/usr/bin/dpkg';
const DPKG_QUERY_PATH = '/usr/bin/dpkg-query';

const PACKAGE_NAME = 'dashmate';

/**
 * The only directory an apt sources file may live in.
 *
 * The path reaches apt as `-o Dir::Etc::sourcelist=`, which decides which repository may answer
 * for this package - i.e. which signing key the upgrade trusts. Pinning the directory keeps
 * that decision inside this module even if a future caller starts passing the value in.
 * It must never be sourced from `config.json`, the environment, or anything else an
 * unprivileged operator can write: that would let an operator-level write influence a root apt
 * invocation, from a position that grants no root today.
 */
const SOURCES_DIR = '/etc/apt/sources.list.d';

const DEFAULT_SOURCES_FILE_PATH = `${SOURCES_DIR}/dashmate.sources`;

/**
 * How long apt waits for the dpkg lock before giving up.
 *
 * unattended-upgrades holds that lock for as long as its own run takes. Without a wait, an
 * upgrade started while it is running fails instantly for a reason that has nothing to do with
 * dashmate.
 */
const DEFAULT_LOCK_TIMEOUT_SECONDS = 600;

/**
 * PATH handed to everything we spawn.
 *
 * Every executable we invoke is named by absolute path, so this exists for the children of
 * those processes: dpkg, and any maintainer script the package runs.
 */
const CHILD_PATH = '/usr/sbin:/usr/bin:/sbin:/bin';

/**
 * HOME for the privileged apt invocation.
 *
 * Fixed rather than inherited: maintainer scripts run as root, and handing them the invoking
 * operator's home directory lets an unprivileged dotfile influence a root process.
 */
const ROOT_HOME = '/root';

const QUICK_TIMEOUT_MS = 30000;
const VERSION_CHECK_TIMEOUT_MS = 60000;

/**
 * No timeout on the package manager itself.
 *
 * Killing apt part way through a dpkg transaction produces exactly the half-configured state
 * this module exists to avoid, and a download over a slow link is not a hang. The lock wait is
 * bounded separately, `DEBIAN_FRONTEND=noninteractive` and `--force-confold` remove the
 * invisible-prompt hang, and a caller that needs a bound can pass one.
 */
const DEFAULT_INSTALL_TIMEOUT_MS = 0;

const MAX_BUFFER = 8 * 1024 * 1024;

/**
 * How much package manager output is carried into an error message.
 *
 * The tail is kept, since that is where apt, dpkg and npm put the reason they failed.
 */
const MAX_REPORTED_OUTPUT = 4000;

/**
 * Accepted application version strings.
 *
 * Deliberately narrower than semver: no build metadata, no leading `v`, nothing that is not a
 * digit, a dot, a hyphen or an alphanumeric. Everything npm would otherwise accept as a
 * package specifier - a git URL, a tarball URL, a `file:` path, an `npm:` alias, a dist-tag -
 * fails this before it can reach an argv array.
 */
const VERSION_PATTERN = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;

const MAX_VERSION_LENGTH = 64;

/**
 * Accepted Debian package version strings, per Debian policy's character set.
 */
const DEB_VERSION_PATTERN = /^(\d+:)?[A-Za-z0-9][A-Za-z0-9.+~:-]*$/;

/**
 * What sudo says when the operator may run the command but must prove who they are.
 */
const SUDO_NEEDS_PASSWORD_PATTERN = /password is required|a terminal is required|no askpass|no tty present/i;

/**
 * What sudo says when sudoers does not permit the command at all.
 */
const SUDO_REFUSED_PATTERN = /not allowed to execute|is not allowed to run|not in the sudoers file|may not run/i;

/**
 * What apt says when the transaction cannot be resolved from the sources it was scoped to.
 */
const APT_UNMET_DEPENDENCY_PATTERN = /unmet dependencies|unable to correct problems|has no installation candidate|depends:/i;

const APT_LOCK_HELD_PATTERN = /could not get lock|unable to (?:acquire|lock)|is another process using it/i;

export const UPGRADE_STEP = {
  VALIDATE_INPUT: 'validate-input',
  ROOT_ACCESS: 'root-access',
  DPKG_AUDIT: 'dpkg-audit',
  QUERY_INSTALLED_VERSION: 'query-installed-version',
  APT_INSTALL: 'apt-install',
  VERIFY_INSTALLED_VERSION: 'verify-installed-version',
  LOCATE_NPM: 'locate-npm',
  NPM_INSTALL: 'npm-install',
  VERIFY_BINARY: 'verify-binary',
  UNEXPECTED: 'unexpected',
};

/**
 * What is known about the installed package.
 *
 * Total by construction: every rejection from this module carries one of these, because the
 * caller's next decision - activate the node, halt, or ask the operator to intervene - is taken
 * from it, and an unrecognised value would land on whichever branch the caller wrote first.
 *
 * `unchanged` is only used where nothing that could modify the package has run yet. Once a
 * package manager has started, neither apt nor npm is atomic - a dpkg unpack can fail part way
 * and `npm install -g` removes before it writes - so the honest answers are `intact` (the
 * installed dashmate was executed and works), `uncertain` (it was executed and did not work),
 * or `unverified` (there was no executable path to try, so nothing is claimed either way).
 * `unapproved` is its own state because it is the one failure where the binary really was
 * replaced: that node needs activation, unlike every other failure here.
 */
export const PACKAGE_STATE = {
  UPGRADED: 'upgraded',
  UNCHANGED: 'unchanged',
  INTACT: 'intact',
  UNVERIFIED: 'unverified',
  UNCERTAIN: 'uncertain',
  UNAPPROVED: 'unapproved',
};

export const ROOT_ACCESS = {
  ALREADY_ROOT: 'already-root',
  SUDO: 'sudo',
  NONE: 'none',
};

/**
 * Only the package replacement may run as root.
 *
 * Under `sudo dashmate ...` the home directory resolves to /root/.dashmate, so config
 * migration, image pulls and restarts would all run against an empty configuration that is not
 * the operator's node.
 *
 * A command must refuse this with `isRunningAsRoot()` at the very start of `init()`, before the
 * DI container is built: resolving `HomeDir` creates the directory and a default config file,
 * and `finally()` writes `config.json` and re-renders templates. By the time this module could
 * report anything, /root/.dashmate has already been created and written to.
 */
export const RUNNING_AS_ROOT_WARNING = 'dashmate is running as root, so only the package upgrade may run in this process:'
  + ' every other step resolves the home directory to /root/.dashmate rather than the operator\'s configuration.';

/**
 * @return {number|null}
 */
function readEuid() {
  return typeof process.geteuid === 'function' ? process.geteuid() : null;
}

/**
 * Whether this process has root privileges.
 *
 * Exported so a command can refuse before it builds anything. Platforms without an euid answer
 * false, which is correct: they have no sudo path either.
 *
 * @param {Object} [options]
 * @param {number|null} [options.euid]
 * @return {boolean}
 */
export function isRunningAsRoot({ euid = readEuid() } = {}) {
  return euid === 0;
}

export class PackageUpgradeError extends AbstractError {
  /**
   * @param {Object} properties
   * @param {string} properties.step - one of UPGRADE_STEP
   * @param {string} properties.summary - what failed, in the operator's terms
   * @param {string} properties.packageState - one of PACKAGE_STATE
   * @param {string} [properties.output] - package manager output, already sanitized
   * @param {string[]} [properties.instructions] - exact commands for the operator
   * @param {number|null} [properties.exitCode]
   * @param {string|null} [properties.installedVersion] - version found installed after the
   *   failure, where it could be read
   * @param {string|null} [properties.elevation] - one of ROOT_ACCESS
   * @param {Array} [properties.commands] - every invocation made, in order
   */
  constructor({
    step,
    summary,
    packageState,
    output = '',
    instructions = [],
    exitCode = null,
    installedVersion = null,
    elevation = null,
    commands = [],
  }) {
    const sections = [summary];

    if (output.length > 0) {
      sections.push(`Output:\n${output}`);
    }

    if (instructions.length > 0) {
      sections.push(`Run by hand:\n${instructions.map((line) => `  ${line}`).join('\n')}`);
    }

    super(sections.join('\n\n'));

    this.step = step;
    this.summary = summary;
    this.packageState = packageState;
    this.output = output;
    this.instructions = instructions;
    this.exitCode = exitCode;
    this.installedVersion = installedVersion;
    this.elevation = elevation;
    this.commands = commands;
  }
}

const ESCAPE_CHARACTER = String.fromCharCode(27);

/**
 * ANSI control sequences. Built from the escape character's code point rather than written as
 * an escape, so the pattern stays readable and the source stays free of control bytes.
 */
const ANSI_SEQUENCE_PATTERN = new RegExp(`${ESCAPE_CHARACTER}\\[[0-9;?]*[ -/]*[@-~]`, 'g');

const TAB_CODE = 9;
const NEWLINE_CODE = 10;

/**
 * @param {number} code
 * @return {boolean}
 */
function isPrintableCode(code) {
  return code >= 32 && code !== 127 && (code < 128 || code > 159);
}

/**
 * Keep the characters a report can be written in, and nothing else.
 *
 * Tab and newline are the only control characters that survive. A carriage return in particular
 * must not: `dashd is half-installed<CR>Everything is fine.` prints as "Everything is fine." on
 * a terminal, with the first clause overwritten and gone.
 *
 * @param {string} text
 * @return {string}
 */
function stripControlCharacters(text) {
  return Array.from(text)
    .filter((character) => {
      const code = character.codePointAt(0);

      return code === TAB_CODE || code === NEWLINE_CODE || isPrintableCode(code);
    })
    .join('');
}

/**
 * Remove anything that could rewrite the operator's terminal, and keep the tail.
 *
 * Package manager output ends up printed verbatim, and it is not all ours: a package name, a
 * repository description or a maintainer script can carry escape sequences.
 *
 * @param {...string} chunks
 * @return {string}
 */
function sanitizeOutput(...chunks) {
  const joined = chunks
    .filter((chunk) => typeof chunk === 'string' && chunk.length > 0)
    .join('\n');

  // Sequences first, so removing the escape character cannot leave a bare "[0m" behind
  const sanitized = stripControlCharacters(joined.replace(ANSI_SEQUENCE_PATTERN, '')).trim();

  if (sanitized.length <= MAX_REPORTED_OUTPUT) {
    return sanitized;
  }

  return `[truncated]\n${sanitized.slice(-MAX_REPORTED_OUTPUT)}`;
}

/**
 * @param {string[]} lines
 * @return {string[]}
 */
function dedupe(lines) {
  return [...new Set(lines)];
}

/**
 * @param {*} version
 * @return {boolean}
 */
function isValidVersion(version) {
  return typeof version === 'string'
    && version.length > 0
    && version.length <= MAX_VERSION_LENGTH
    && VERSION_PATTERN.test(version)
    && semver.valid(version) === version;
}

/**
 * @param {*} debVersion
 * @return {boolean}
 */
function isValidDebVersion(debVersion) {
  return typeof debVersion === 'string'
    && debVersion.length > 0
    && debVersion.length <= MAX_VERSION_LENGTH
    && DEB_VERSION_PATTERN.test(debVersion);
}

/**
 * Whether every character of a value is printable ASCII.
 *
 * Applied to values that become argv option tokens. Written as the range that is accepted
 * rather than a class of control characters to reject, so it cannot quietly let through
 * something nobody thought to list.
 *
 * @param {string} value
 * @return {boolean}
 */
function isPrintableAscii(value) {
  return !/[^ -~]/.test(value);
}

/**
 * Whether a value is usable as a filesystem path we will execute or read.
 *
 * Non-ASCII is allowed, unlike in an option token: an operator's home directory is a legitimate
 * place for an npm prefix and is not ours to reject. Control characters are not allowed, and
 * neither is a path that is not already normalized.
 *
 * @param {*} value
 * @return {boolean}
 */
function isUsableAbsolutePath(value) {
  return typeof value === 'string'
    && value.length > 0
    && path.isAbsolute(value)
    && value === path.normalize(value)
    && !Array.from(value).some((character) => !isPrintableCode(character.codePointAt(0)));
}

/**
 * The `major.minor.patch` a Debian version was built from.
 *
 * Debian version ordering and application semver are different orderings over different
 * strings - `4.1.0~rc.3-1` against `4.1.0-rc.3` - so they are never compared directly. This
 * extracts the one part both spellings share, which is enough to reject a caller that paired a
 * Debian version with an application version from a different release.
 *
 * @param {string} debVersion
 * @return {string|null}
 */
function debUpstreamCore(debVersion) {
  const withoutEpoch = debVersion.replace(/^\d+:/, '');
  const revisionAt = withoutEpoch.lastIndexOf('-');
  const upstream = revisionAt === -1 ? withoutEpoch : withoutEpoch.slice(0, revisionAt);

  const matched = upstream.match(/^(\d+)\.(\d+)\.(\d+)/);

  return matched === null ? null : `${matched[1]}.${matched[2]}.${matched[3]}`;
}

/**
 * @param {string} version
 * @return {string}
 */
function semverCore(version) {
  return `${semver.major(version)}.${semver.minor(version)}.${semver.patch(version)}`;
}

/**
 * Refuse anything that is not an absolute path or is not a plain string argument.
 *
 * A last barrier in front of the process boundary. Every caller below builds its argv from
 * constants, so this only ever fires on a programming mistake - which is exactly when it
 * matters, because the mistake would be a subprocess invocation nobody intended.
 *
 * Exported because it is part of what this module promises, not an implementation detail.
 *
 * @param {string} file
 * @param {string[]} args
 */
export function assertArgv(file, args) {
  if (typeof file !== 'string' || !path.isAbsolute(file)) {
    throw new TypeError(`Refusing to run "${file}": executables must be named by absolute path`);
  }

  if (!Array.isArray(args)
    || args.some((arg) => typeof arg !== 'string' || !isPrintableAscii(arg))) {
    throw new TypeError(`Refusing to run ${file}: arguments must be plain single-line strings`);
  }
}

/**
 * @param {string} filePath
 * @return {Promise<boolean>}
 */
async function defaultPathExists(filePath) {
  try {
    await fsPromises.access(filePath, fsConstants.F_OK);

    return true;
  } catch {
    return false;
  }
}

/**
 * @param {string} filePath
 * @return {Promise<string>}
 */
function defaultReadFile(filePath) {
  return fsPromises.readFile(filePath, 'utf8');
}

/**
 * Run a command and describe how it ended, without throwing.
 *
 * `execFile` with an argv array and no shell. Every outcome - a non-zero exit, a signal, a
 * failure to spawn at all - is returned as data, so no decision below is taken on the shape of
 * an exception.
 *
 * @param {string} file
 * @param {string[]} args
 * @param {Object} options
 * @param {Object} options.env
 * @param {number} [options.timeout]
 * @return {Promise<CommandResult>}
 */
async function defaultRunCommand(file, args, { env, timeout = 0 }) {
  /**
   * @typedef {Object} CommandResult
   * @property {number|null} exitCode - null when the process never exited normally
   * @property {string} stdout
   * @property {string} stderr
   * @property {string|null} signal
   * @property {Error|null} spawnError - set when the process could not be started at all
   */
  try {
    const { stdout, stderr } = await execFileAsync(file, args, {
      env,
      timeout,
      maxBuffer: MAX_BUFFER,
      windowsHide: true,
    });

    return {
      exitCode: 0, stdout, stderr, signal: null, spawnError: null,
    };
  } catch (e) {
    const stdout = typeof e.stdout === 'string' ? e.stdout : '';
    const stderr = typeof e.stderr === 'string' ? e.stderr : '';

    if (typeof e.code === 'number') {
      return {
        exitCode: e.code, stdout, stderr, signal: null, spawnError: null,
      };
    }

    if (e.killed === true || typeof e.signal === 'string') {
      return {
        exitCode: null, stdout, stderr, signal: e.signal ?? null, spawnError: null,
      };
    }

    return {
      exitCode: null, stdout, stderr, signal: null, spawnError: e,
    };
  }
}

/**
 * Run a command attached to the operator's terminal.
 *
 * Only `sudo -v` uses this, and only outside a listr renderer: a spinner redrawing the line
 * garbles or swallows the password prompt, and an operator who cannot see what is being asked
 * types their password into whatever is listening.
 *
 * @param {string} file
 * @param {string[]} args
 * @param {Object} options
 * @param {Object} options.env
 * @return {Promise<CommandResult>}
 */
function defaultRunInteractiveCommand(file, args, { env }) {
  return new Promise((resolve) => {
    const child = childProcess.spawn(file, args, {
      env,
      stdio: 'inherit',
      windowsHide: true,
    });

    child.on('error', (e) => resolve({
      exitCode: null, stdout: '', stderr: '', signal: null, spawnError: e,
    }));

    child.on('close', (exitCode, signal) => resolve({
      exitCode, stdout: '', stderr: '', signal, spawnError: null,
    }));
  });
}

/**
 * @param {Object} env
 * @return {string|null}
 */
function homeFromEnv(env) {
  const home = env?.HOME;

  return isUsableAbsolutePath(home) ? home : null;
}

/**
 * Environment for the unprivileged probes: reading the dpkg database and running dashmate.
 *
 * The locale is pinned so parsed output does not change with the operator's language.
 *
 * @param {Object} env
 * @return {Object}
 */
function buildProbeEnv(env) {
  const home = homeFromEnv(env);

  return {
    PATH: CHILD_PATH,
    LANG: 'C',
    LC_ALL: 'C',
    ...(home === null ? {} : { HOME: home }),
  };
}

/**
 * Environment for apt and everything it runs.
 *
 * @param {string} home
 * @return {Object}
 */
function buildAptEnv(home) {
  return {
    PATH: CHILD_PATH,
    HOME: home,
    LANG: 'C',
    LC_ALL: 'C',
    // Maintainer scripts must never wait on a prompt nobody can see
    DEBIAN_FRONTEND: 'noninteractive',
  };
}

/**
 * Environment for npm.
 *
 * npm's launcher falls back to whatever `node` PATH resolves when it cannot find one beside
 * itself, so the directory of the node currently running dashmate is prepended - a value we
 * computed, never one we inherited. HOME is passed through because it is where npm's own
 * configuration and the global prefix an operator may have set both live.
 *
 * @param {Object} env
 * @param {string} nodeExecPath
 * @return {Object}
 */
function buildNpmEnv(env, nodeExecPath) {
  const home = homeFromEnv(env);
  const nodeDir = path.dirname(nodeExecPath);

  return {
    PATH: `${nodeDir}:${CHILD_PATH}`,
    LANG: 'C',
    LC_ALL: 'C',
    ...(home === null ? {} : { HOME: home }),
  };
}

/**
 * The apt invocation, without any decision about how it becomes root.
 *
 * @param {Object} options
 * @param {string} options.sourcesFilePath
 * @param {number} options.lockTimeoutSeconds
 * @param {string} options.expectedDebVersion
 * @return {{file: string, args: string[]}}
 */
function buildAptCommand({ sourcesFilePath, lockTimeoutSeconds, expectedDebVersion }) {
  return {
    file: APT_GET_PATH,
    args: [
      'install',
      '--only-upgrade',
      // --only-upgrade means "the package must already be installed", not "touch nothing else":
      // resolution may still remove a conflicting package, and -y would approve that
      '--no-remove',
      '-y',
      // Scope apt to our own sources file, and to nothing else on the host. An unrelated
      // third-party source that is broken, unreachable or hostile then cannot abort an upgrade
      // of ours, and no other repository can answer for this package.
      // Each option is a separate argv token, which is the form apt documents.
      '-o', `Dir::Etc::sourcelist=${sourcesFilePath}`,
      '-o', 'Dir::Etc::sourceparts=/dev/null',
      // Wait for unattended-upgrades rather than failing the moment it holds the lock
      '-o', `DPkg::Lock::Timeout=${lockTimeoutSeconds}`,
      // A changed conffile must not stop and wait for an answer nobody can see
      '-o', 'Dpkg::Options::=--force-confold',
      // Pin the exact version. Without it apt installs whatever the pool sorts highest, and
      // prereleases and stable share one flat pool - so an operator who approved a stable
      // release would get a release candidate, and every check downstream would call it success.
      `${PACKAGE_NAME}=${expectedDebVersion}`,
    ],
  };
}

/**
 * The command sudo is asked to run: apt, wrapped so its environment is ours.
 *
 * `env -i` rebuilds the environment from nothing rather than handing it to `execFile`, because
 * sudoers decides what survives sudo - a host configured with `!env_reset` or a generous
 * `env_keep` would otherwise let `APT_CONFIG` through from the caller, and that one variable
 * can disable the signature verification this design rests on.
 *
 * The cost is that sudoers must permit `/usr/bin/env`, not `/usr/bin/apt-get`, which is why a
 * refusal prints `sudoersRuleFor` rather than leaving the operator to guess.
 *
 * @param {Object} childEnv
 * @param {{file: string, args: string[]}} command
 * @return {{file: string, args: string[]}}
 */
function wrapInEnv(childEnv, command) {
  const envTokens = Object.entries(childEnv).map(([name, value]) => `${name}=${value}`);

  return {
    file: ENV_PATH,
    args: ['-i', ...envTokens, command.file, ...command.args],
  };
}

/**
 * The sudoers line that permits exactly this invocation.
 *
 * @param {{file: string, args: string[]}} command
 * @return {string}
 */
function sudoersRuleFor(command) {
  return `<user> ALL=(root) NOPASSWD: ${command.file} ${command.args.join(' ')}`;
}

/**
 * @param {Object} rootAccess
 * @param {{file: string, args: string[]}} command - already wrapped by wrapInEnv
 * @param {Object} childEnv
 * @return {{file: string, args: string[], env: Object}}
 */
function buildPrivilegedInvocation(rootAccess, command, childEnv) {
  if (rootAccess.elevation === ROOT_ACCESS.ALREADY_ROOT) {
    // Already root, so the environment can simply be handed to execFile
    return { file: command.file, args: command.args, env: childEnv };
  }

  return {
    // `-n` never prompts - a sudo timestamp that expired between the pre-flight and here fails
    // loudly instead of stalling behind a renderer - and `--` stops sudo parsing our arguments
    file: SUDO_PATH,
    args: ['-n', '--', command.file, ...command.args],
    env: childEnv,
  };
}

/**
 * @param {*} rootAccess
 * @return {boolean}
 */
function isValidRootAccess(rootAccess) {
  return rootAccess !== null
    && typeof rootAccess === 'object'
    && Object.values(ROOT_ACCESS).includes(rootAccess.elevation)
    && (rootAccess.euid === null || Number.isInteger(rootAccess.euid))
    && typeof rootAccess.authenticated === 'boolean'
    && Array.isArray(rootAccess.warnings)
    && rootAccess.warnings.every((warning) => typeof warning === 'string');
}

/**
 * Classify what sudo said about a command it would not run.
 *
 * The two answers call for completely different action - authenticate, or edit sudoers - and
 * they are indistinguishable by exit code.
 *
 * @param {string} output
 * @return {'password'|'refused'|'unknown'}
 */
function classifySudoRefusal(output) {
  if (SUDO_REFUSED_PATTERN.test(output)) {
    return 'refused';
  }

  if (SUDO_NEEDS_PASSWORD_PATTERN.test(output)) {
    return 'password';
  }

  return 'unknown';
}

/**
 * Obtain the root privileges the deb path needs, or explain why it cannot.
 *
 * Runs before anything is touched, so a host where root is unattainable is told so instead of
 * finding out half way through. Interactive authentication is opt-in and must be requested by a
 * caller that is not inside a listr renderer; the default is non-interactive, which can fail but
 * can never hang.
 *
 * Where the deb parameters are supplied, the pre-flight asks sudo about the *exact* command that
 * will run (`sudo -n -l -- ...`) rather than about `true`. That distinction decides what the
 * operator is told on a host with a narrow sudoers rule: asking about `true` there is refused,
 * and reporting "sudo would ask for a password" would send them to fix the wrong thing.
 *
 * A successful pre-flight is not a guarantee. The sudo timestamp can expire while apt is
 * downloading, so the privileged call is the authoritative check and classifies its own refusal
 * the same way.
 *
 * @param {Object} [options]
 * @param {number|null} [options.euid]
 * @param {boolean} [options.interactive] - allow a password prompt
 * @param {boolean} [options.isTTY]
 * @param {NodeJS.ProcessEnv} [options.env]
 * @param {string[]} [options.instructions] - commands printed when root cannot be obtained
 * @param {string} [options.sourcesFilePath] - deb parameters, so the pre-flight can ask about
 * @param {number} [options.lockTimeoutSeconds]   the exact command rather than about `true`
 * @param {string} [options.expectedDebVersion]
 * @param {function} [options.runCommand]
 * @param {function} [options.runInteractiveCommand]
 * @param {function} [options.pathExists]
 * @return {Promise<RootAccess>}
 */
export async function ensureRootAccess(options = {}) {
  /**
   * @typedef {Object} RootAccess
   * @property {string} elevation - one of ROOT_ACCESS
   * @property {number|null} euid
   * @property {boolean} authenticated - sudo will run the command without asking for a password
   * @property {string[]} warnings
   */
  const {
    euid = readEuid(),
    interactive = false,
    isTTY = Boolean(process.stdin?.isTTY && process.stderr?.isTTY),
    env = process.env,
    instructions = [],
    sourcesFilePath = null,
    lockTimeoutSeconds = DEFAULT_LOCK_TIMEOUT_SECONDS,
    expectedDebVersion = null,
    runCommand = defaultRunCommand,
    runInteractiveCommand = defaultRunInteractiveCommand,
    pathExists = defaultPathExists,
  } = options;

  if (isRunningAsRoot({ euid })) {
    return {
      elevation: ROOT_ACCESS.ALREADY_ROOT,
      euid,
      authenticated: true,
      warnings: [RUNNING_AS_ROOT_WARNING],
    };
  }

  const probeEnv = buildProbeEnv(env);

  const fail = (summary, extraInstructions = []) => new PackageUpgradeError({
    step: UPGRADE_STEP.ROOT_ACCESS,
    summary,
    packageState: PACKAGE_STATE.UNCHANGED,
    instructions: dedupe([...instructions, ...extraInstructions]),
    elevation: ROOT_ACCESS.SUDO,
  });

  if (!await pathExists(SUDO_PATH)) {
    throw fail(`Replacing the dashmate package needs root privileges and ${SUDO_PATH} is not installed.`);
  }

  // `sudo -l <command>` matches sudoers against the command *and its arguments*, so a narrow
  // rule answers accurately instead of being asked about something unrelated.
  const privilegedCommand = sourcesFilePath !== null && expectedDebVersion !== null
    ? wrapInEnv(
      buildAptEnv(ROOT_HOME),
      buildAptCommand({ sourcesFilePath, lockTimeoutSeconds, expectedDebVersion }),
    )
    : null;

  const probeArgs = privilegedCommand === null
    // `true` is the one command named without a path, and sudo resolves it for itself: either
    // from sudoers' secure_path, which is root's to set, or from the PATH below, which is ours.
    ? ['-n', 'true']
    : ['-n', '-l', '--', privilegedCommand.file, ...privilegedCommand.args];

  const probe = await runCommand(SUDO_PATH, probeArgs, {
    env: probeEnv,
    timeout: QUICK_TIMEOUT_MS,
  });

  if (probe.spawnError !== null) {
    throw fail(`Could not run ${SUDO_PATH}: ${probe.spawnError.message}`);
  }

  if (probe.exitCode === 0) {
    return {
      elevation: ROOT_ACCESS.SUDO, euid, authenticated: true, warnings: [],
    };
  }

  const probeOutput = sanitizeOutput(probe.stdout, probe.stderr);
  const refusal = classifySudoRefusal(probeOutput);

  if (refusal === 'refused') {
    throw fail(
      'sudo does not permit this account to run the package upgrade. That is a sudoers policy rather than'
      + ` a password prompt, so authenticating will not help.\n${probeOutput}`,
      privilegedCommand === null ? [] : [`Add to sudoers: ${sudoersRuleFor(privilegedCommand)}`],
    );
  }

  if (refusal === 'unknown') {
    throw fail(`sudo refused the pre-flight check for a reason dashmate does not recognise.\n${probeOutput}`);
  }

  if (!interactive) {
    throw fail('Replacing the dashmate package needs root privileges, and sudo would ask for a password.'
      + ' Nothing here can prompt for one without a terminal to prompt on.');
  }

  if (!isTTY) {
    throw fail('Replacing the dashmate package needs root privileges, and sudo would ask for a password,'
      + ' but this process has no terminal to ask on.');
  }

  // Authenticate on the terminal, then go back to -n for the work itself
  const prompted = await runInteractiveCommand(SUDO_PATH, ['-v'], { env: probeEnv });

  if (prompted.exitCode !== 0) {
    throw fail('sudo authentication failed or was cancelled.');
  }

  const afterPrompt = await runCommand(SUDO_PATH, probeArgs, {
    env: probeEnv,
    timeout: QUICK_TIMEOUT_MS,
  });

  if (afterPrompt.exitCode !== 0) {
    throw fail('sudo accepted the password but still refuses to run the upgrade without one.');
  }

  return {
    elevation: ROOT_ACCESS.SUDO, euid, authenticated: true, warnings: [],
  };
}

/**
 * Run one command, recording it, so a failure can name what ran.
 *
 * @param {Object} context
 * @param {string} step
 * @param {Object} invocation
 * @return {Promise<CommandResult>}
 */
async function runStep(context, step, {
  file, args, env, timeout = QUICK_TIMEOUT_MS,
}) {
  assertArgv(file, args);

  const result = await context.runCommand(file, args, { env, timeout });

  context.commands.push({
    step, file, args: [...args], exitCode: result.exitCode,
  });

  return result;
}

/**
 * Whether the installed dashmate still executes.
 *
 * Run after any package manager failure. apt is transactional per package, but a dpkg unpack
 * that failed part way leaves a half-configured package, and `npm install -g` is not atomic at
 * all: it removes before it writes. The difference between "your upgrade did not happen" and
 * "your dashmate is broken" is the difference between reading the message later and acting on
 * it now.
 *
 * The executable is named by the absolute path detection resolved, never through PATH. When
 * there is no such path the check is *not attempted*, which is a third answer: saying a package
 * may be broken because we had nowhere to look is how a benign failure becomes a
 * middle-of-the-night incident.
 *
 * @param {Object} context
 * @return {Promise<{attempted: boolean, ran: boolean, version: string|null, reason: string}>}
 */
async function verifyDashmateRuns(context) {
  const { binPath } = context;

  if (binPath === null) {
    return {
      attempted: false,
      ran: false,
      version: null,
      reason: 'the path of the installed dashmate executable is not known',
    };
  }

  const result = await runStep(context, UPGRADE_STEP.VERIFY_BINARY, {
    file: binPath,
    args: ['--version'],
    env: context.probeEnv,
    timeout: VERSION_CHECK_TIMEOUT_MS,
  });

  if (result.spawnError !== null) {
    return {
      attempted: true,
      ran: false,
      version: null,
      reason: `${binPath} could not be executed: ${result.spawnError.message}`,
    };
  }

  if (result.exitCode !== 0) {
    return {
      attempted: true,
      ran: false,
      version: null,
      reason: `${binPath} --version exited with code ${result.exitCode}`,
    };
  }

  // oclif prints "dashmate/4.1.0 linux-x64 node-v20.11.0"
  const matched = `${result.stdout} ${result.stderr}`.match(/(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)/);

  return {
    attempted: true,
    ran: true,
    version: matched === null ? null : matched[1],
    reason: '',
  };
}

/**
 * Fail after the package manager has run, saying what is known about the package.
 *
 * @param {Object} context
 * @param {Object} failure
 * @return {Promise<never>}
 */
async function failAfterPackageManager(context, {
  step, summary, output = '', exitCode = null, instructions = [], installedVersion = null,
}) {
  const health = await verifyDashmateRuns(context);

  const common = {
    step,
    output,
    exitCode,
    installedVersion,
    elevation: context.elevation,
    commands: context.commands,
  };

  if (!health.attempted) {
    throw new PackageUpgradeError({
      ...common,
      summary: `${summary}\nWhether the installed dashmate still works was not checked, because ${health.reason}.`,
      packageState: PACKAGE_STATE.UNVERIFIED,
      instructions: dedupe([`${PACKAGE_NAME} --version`, ...instructions]),
    });
  }

  if (health.ran) {
    throw new PackageUpgradeError({
      ...common,
      summary: `${summary}\nThe installed dashmate still runs${health.version === null ? '' : ` and reports ${health.version}`}.`,
      packageState: PACKAGE_STATE.INTACT,
      installedVersion: installedVersion ?? health.version,
      instructions: dedupe(instructions),
    });
  }

  throw new PackageUpgradeError({
    ...common,
    summary: `${summary}\nThe installed dashmate could not be verified to work: ${health.reason}. The package is in an uncertain state.`,
    packageState: PACKAGE_STATE.UNCERTAIN,
    instructions: dedupe([...context.recoveryInstructions, ...instructions]),
  });
}

/**
 * Read the version dpkg records for the package.
 *
 * @param {Object} context
 * @return {Promise<{version: string|null, result: CommandResult}>}
 */
async function queryInstalledDebVersion(context) {
  const result = await runStep(context, UPGRADE_STEP.QUERY_INSTALLED_VERSION, {
    file: DPKG_QUERY_PATH,
    // A dpkg-query format string, not a shell expansion and not a template literal:
    // dpkg-query substitutes it itself, and nothing here ever reaches a shell
    // eslint-disable-next-line no-template-curly-in-string
    args: ['-W', '-f=${Version}', PACKAGE_NAME],
    env: context.probeEnv,
    timeout: QUICK_TIMEOUT_MS,
  });

  if (result.spawnError !== null || result.exitCode !== 0) {
    return { version: null, result };
  }

  const version = result.stdout.trim();

  return { version: isValidDebVersion(version) ? version : null, result };
}

/**
 * @param {Object} context
 * @param {number} exitCode
 * @param {string} output
 * @param {{file: string, args: string[]}} privilegedCommand
 * @return {string}
 */
function describeAptFailure(context, exitCode, output, privilegedCommand) {
  const refusal = classifySudoRefusal(output);

  if (refusal === 'refused') {
    return 'sudo refused to run the package upgrade. That is a sudoers policy rather than a password prompt.'
      + ` The rule has to permit ${privilegedCommand.file}, not ${APT_GET_PATH}: the environment is rebuilt`
      + ' by env so that nothing can be inherited into a root apt run.';
  }

  if (refusal === 'password') {
    return 'sudo asked for a password during the upgrade. The authentication taken before the upgrade started'
      + ' has expired, and nothing here can prompt for another one from inside a progress renderer.';
  }

  if (APT_LOCK_HELD_PATTERN.test(output)) {
    return `apt-get gave up waiting ${context.lockTimeoutSeconds}s for the dpkg lock.`
      + ' Another package manager - usually unattended-upgrades - is still holding it.';
  }

  if (APT_UNMET_DEPENDENCY_PATTERN.test(output)) {
    return `apt-get could not resolve the transaction from ${context.sourcesFilePath} alone (exit code ${exitCode}).`
      + ' The upgrade is deliberately scoped to the dashmate repository so that an unrelated broken source cannot'
      + ' abort it, which also means a dependency living in the distribution archives cannot be seen from here.';
  }

  return `apt-get exited with code ${exitCode}.`;
}

/**
 * @param {string} output
 * @param {{file: string, args: string[]}} privilegedCommand
 * @return {string[]}
 */
function aptFailureInstructions(output, privilegedCommand) {
  if (classifySudoRefusal(output) === 'refused') {
    return [`Add to sudoers: ${sudoersRuleFor(privilegedCommand)}`];
  }

  return [];
}

/**
 * Replace the package with apt.
 *
 * @param {Object} context
 * @return {Promise<Object>}
 */
async function upgradeDebPackage(context) {
  const { expectedDebVersion, rootAccess } = context;

  // A dpkg database that was already broken produces apt failures that have nothing to do with
  // us, and blaming the upgrade for them sends the operator looking in the wrong place.
  const audit = await runStep(context, UPGRADE_STEP.DPKG_AUDIT, {
    file: DPKG_PATH,
    args: ['--audit'],
    env: context.probeEnv,
    timeout: QUICK_TIMEOUT_MS,
  });

  if (audit.spawnError !== null) {
    throw new PackageUpgradeError({
      step: UPGRADE_STEP.DPKG_AUDIT,
      summary: `Could not run ${DPKG_PATH} --audit: ${audit.spawnError.message}`,
      packageState: PACKAGE_STATE.UNCHANGED,
      elevation: context.elevation,
      commands: context.commands,
    });
  }

  // dpkg reports its findings on stdout and still exits 0 on some versions, so the report
  // itself is the signal rather than the exit code alone.
  const auditReport = sanitizeOutput(audit.stdout, audit.stderr);

  if (audit.exitCode !== 0 || auditReport.length > 0) {
    throw new PackageUpgradeError({
      step: UPGRADE_STEP.DPKG_AUDIT,
      summary: 'dpkg reports packages in a broken state on this host. That is not caused by the upgrade,'
        + ' and upgrading now would fail in a way that looks like it was.',
      packageState: PACKAGE_STATE.UNCHANGED,
      output: auditReport,
      exitCode: audit.exitCode,
      instructions: ['sudo dpkg --configure -a', 'sudo apt-get --fix-broken install'],
      elevation: context.elevation,
      commands: context.commands,
    });
  }

  const before = await queryInstalledDebVersion(context);

  if (before.version === null) {
    throw new PackageUpgradeError({
      step: UPGRADE_STEP.QUERY_INSTALLED_VERSION,
      summary: `dpkg does not report an installed version of ${PACKAGE_NAME}, so there is nothing to upgrade.`,
      packageState: PACKAGE_STATE.UNCHANGED,
      output: sanitizeOutput(before.result.stdout, before.result.stderr),
      exitCode: before.result.exitCode,
      elevation: context.elevation,
      commands: context.commands,
    });
  }

  const aptEnv = buildAptEnv(rootAccess.elevation === ROOT_ACCESS.ALREADY_ROOT
    ? homeFromEnv(context.env) ?? ROOT_HOME
    : ROOT_HOME);

  const privilegedCommand = wrapInEnv(aptEnv, buildAptCommand(context));
  const invocation = buildPrivilegedInvocation(rootAccess, privilegedCommand, aptEnv);

  context.packageManagerRan = true;

  const install = await runStep(context, UPGRADE_STEP.APT_INSTALL, {
    ...invocation,
    timeout: context.installTimeoutMs,
  });

  if (install.spawnError !== null) {
    throw new PackageUpgradeError({
      step: UPGRADE_STEP.APT_INSTALL,
      summary: `Could not run ${invocation.file}: ${install.spawnError.message}`,
      packageState: PACKAGE_STATE.UNCHANGED,
      instructions: context.manualInstructions,
      elevation: context.elevation,
      commands: context.commands,
    });
  }

  if (install.exitCode !== 0) {
    const output = sanitizeOutput(install.stdout, install.stderr);

    await failAfterPackageManager(context, {
      step: UPGRADE_STEP.APT_INSTALL,
      summary: describeAptFailure(context, install.exitCode, output, privilegedCommand),
      output,
      exitCode: install.exitCode,
      instructions: dedupe([
        ...aptFailureInstructions(output, privilegedCommand),
        ...context.manualInstructions,
      ]),
    });
  }

  // apt exits 0 for "0 upgraded" exactly as it does for a real upgrade, so what dpkg records
  // now is the only evidence that anything happened.
  const after = await queryInstalledDebVersion(context);

  if (after.version === null) {
    await failAfterPackageManager(context, {
      step: UPGRADE_STEP.VERIFY_INSTALLED_VERSION,
      summary: 'apt-get reported success but dpkg no longer reports an installed version of dashmate.',
      output: sanitizeOutput(after.result.stdout, after.result.stderr),
      exitCode: after.result.exitCode,
    });
  }

  if (after.version === expectedDebVersion && after.version === before.version) {
    await failAfterPackageManager(context, {
      step: UPGRADE_STEP.VERIFY_INSTALLED_VERSION,
      summary: `apt-get exited 0 but dashmate is still ${after.version}, the version that was already installed:`
        + ' nothing was upgraded.',
      installedVersion: after.version,
      instructions: ['sudo apt-get update', `apt-cache policy ${PACKAGE_NAME}`, 'apt-mark showhold'],
    });
  }

  if (after.version !== expectedDebVersion) {
    // The version is pinned in the argv, so apt should have refused rather than reach here.
    // Getting here means the package on disk is not the approved one, and unlike every other
    // failure in this function the binary really was replaced.
    throw new PackageUpgradeError({
      step: UPGRADE_STEP.VERIFY_INSTALLED_VERSION,
      summary: `apt-get exited 0 but dashmate is now ${after.version}, not the ${expectedDebVersion} that was approved.`
        + ' The installed package is not the approved one, and the node has not been activated for it.',
      packageState: PACKAGE_STATE.UNAPPROVED,
      installedVersion: after.version,
      instructions: [
        `sudo apt-get install --only-upgrade ${PACKAGE_NAME}=${expectedDebVersion}`,
        `apt-cache policy ${PACKAGE_NAME}`,
      ],
      elevation: context.elevation,
      commands: context.commands,
    });
  }

  return {
    previousVersion: before.version,
    installedVersion: after.version,
    versionScheme: 'debian',
    verifiedBy: 'dpkg-query',
  };
}

/**
 * Find npm without going through PATH.
 *
 * A global install puts npm in the same prefix it puts dashmate, and the node running us lives
 * beside its own npm. Either is the npm that owns this installation; PATH is whatever the
 * caller's shell happened to be, which is the ambiguity an upgrade must not inherit.
 *
 * @param {Object} context
 * @return {Promise<string>}
 */
async function locateNpm(context) {
  const candidates = [
    context.npmGlobalPrefix === null ? null : path.join(context.npmGlobalPrefix, 'bin', 'npm'),
    path.join(path.dirname(context.nodeExecPath), 'npm'),
  ].filter((candidate) => candidate !== null);

  const found = await candidates.reduce(async (previousPromise, candidate) => {
    const previous = await previousPromise;

    if (previous !== null) {
      return previous;
    }

    return await context.pathExists(candidate) ? candidate : null;
  }, Promise.resolve(null));

  if (found === null) {
    throw new PackageUpgradeError({
      step: UPGRADE_STEP.LOCATE_NPM,
      summary: 'npm could not be found next to this installation, and it is deliberately not looked up on PATH.',
      packageState: PACKAGE_STATE.UNCHANGED,
      instructions: context.manualInstructions,
      elevation: context.elevation,
      commands: context.commands,
    });
  }

  return found;
}

/**
 * Read the version npm has installed globally.
 *
 * The package manifest is read first because it is what npm just wrote, and executing the
 * freshly installed binary is the fallback for layouts where the global prefix could not be
 * derived.
 *
 * @param {Object} context
 * @return {Promise<{version: string|null, verifiedBy: string|null, reason: string}>}
 */
async function readNpmInstalledVersion(context) {
  if (context.npmGlobalPrefix !== null) {
    const manifestPath = path.join(
      context.npmGlobalPrefix,
      'lib',
      'node_modules',
      PACKAGE_NAME,
      'package.json',
    );

    try {
      const manifest = JSON.parse(await context.readFile(manifestPath));

      if (isValidVersion(manifest?.version)) {
        return { version: manifest.version, verifiedBy: manifestPath, reason: '' };
      }
    } catch (e) {
      context.warnings.push(`Could not read ${manifestPath}: ${e.message}`);
    }
  }

  const health = await verifyDashmateRuns(context);

  if (health.ran && health.version !== null) {
    return { version: health.version, verifiedBy: `${context.binPath} --version`, reason: '' };
  }

  return {
    version: null,
    verifiedBy: null,
    reason: health.ran ? 'dashmate --version printed no version' : health.reason,
  };
}

/**
 * Replace the package with npm.
 *
 * One attempt, never a retry: a global install that failed part way has already removed files,
 * and running it again on a hunch is how a recoverable failure becomes an unrecoverable one.
 *
 * @param {Object} context
 * @return {Promise<Object>}
 */
async function upgradeNpmPackage(context) {
  const { version } = context;

  const npmPath = await locateNpm(context);

  context.packageManagerRan = true;

  const install = await runStep(context, UPGRADE_STEP.NPM_INSTALL, {
    file: npmPath,
    args: ['install', '-g', `${PACKAGE_NAME}@${version}`],
    env: buildNpmEnv(context.env, context.nodeExecPath),
    timeout: context.installTimeoutMs,
  });

  if (install.spawnError !== null) {
    throw new PackageUpgradeError({
      step: UPGRADE_STEP.NPM_INSTALL,
      summary: `Could not run ${npmPath}: ${install.spawnError.message}`,
      packageState: PACKAGE_STATE.UNCHANGED,
      instructions: context.manualInstructions,
      elevation: context.elevation,
      commands: context.commands,
    });
  }

  if (install.exitCode !== 0) {
    const output = sanitizeOutput(install.stdout, install.stderr);

    // A global prefix owned by root is the usual reason an npm upgrade fails, and it is the one
    // case where the command to run by hand is not the command we just ran.
    const permissionDenied = /\bEACCES\b|\bEPERM\b|permission denied/i.test(output);

    await failAfterPackageManager(context, {
      step: UPGRADE_STEP.NPM_INSTALL,
      summary: permissionDenied
        ? `npm could not write to its global directory (exit code ${install.exitCode}).`
        + ' The global prefix is owned by another user, usually root, which is why the command below'
        + ' is the one to run rather than the one that just failed.'
        + '\nIf this dashmate came from nvm, volta or asdf, do not use sudo: installing as root into a'
        + ' version manager\'s prefix leaves files the operator can no longer replace. Fix the'
        + ' ownership of the prefix instead.'
        : `npm exited with code ${install.exitCode}.`,
      output,
      exitCode: install.exitCode,
      instructions: permissionDenied
        ? [`sudo npm install -g ${PACKAGE_NAME}@${version}`]
        : context.manualInstructions,
    });
  }

  const installed = await readNpmInstalledVersion(context);

  if (installed.version === null) {
    throw new PackageUpgradeError({
      step: UPGRADE_STEP.VERIFY_INSTALLED_VERSION,
      summary: 'npm exited 0, so the upgrade probably did happen, but which version is now installed could not'
        + ` be read back: ${installed.reason}. An unverified upgrade is not one this command will report as done.`,
      packageState: PACKAGE_STATE.UNVERIFIED,
      instructions: dedupe([`${PACKAGE_NAME} --version`, ...context.manualInstructions]),
      elevation: context.elevation,
      commands: context.commands,
    });
  }

  if (installed.version !== version) {
    throw new PackageUpgradeError({
      step: UPGRADE_STEP.VERIFY_INSTALLED_VERSION,
      summary: `npm exited 0 but the installed dashmate is ${installed.version}, not the ${version} that was approved.`,
      packageState: PACKAGE_STATE.UNAPPROVED,
      installedVersion: installed.version,
      instructions: context.manualInstructions,
      elevation: context.elevation,
      commands: context.commands,
    });
  }

  return {
    previousVersion: null,
    installedVersion: installed.version,
    versionScheme: 'semver',
    verifiedBy: installed.verifiedBy,
  };
}

/**
 * Replace the dashmate package with the given version.
 *
 * @param {Object} options
 * @param {string} options.installMethod - one of INSTALL_METHODS, from detectInstallMethod
 * @param {string} options.version - application version to install, re-validated here
 * @param {string} options.expectedDebVersion - required on the deb path: the exact Debian
 *   version apt must install. Debian ordering and application semver are different orderings
 *   over different strings, so only the caller that resolved the candidate from apt knows it.
 *   It is pinned in the argv so apt refuses up front rather than installing whatever the pool
 *   sorts highest - which, with prereleases and stable sharing one flat pool, is how a node
 *   whose operator approved a stable release ends up on a release candidate.
 * @param {string|null} [options.binPath] - absolute path of the installed dashmate executable,
 *   from detectInstallMethod. Used to prove dashmate still runs after a failure. It is executed,
 *   so it must come from detection and never from the environment: `DASHMATE_BINPATH` is the
 *   tarball wrapper's variable, and the only thing keeping an environment-supplied path out of
 *   here is the install-method gate below refusing tarball installs.
 * @param {string|null} [options.npmGlobalPrefix] - from detectInstallMethod's evidence, which
 *   derives it from the running package root. Never from config or the environment: npm is
 *   executed out of it.
 * @param {RootAccess} [options.rootAccess] - from ensureRootAccess. When omitted, root access is
 *   established here without a prompt, so a caller that forgot cannot end up asking for a
 *   password from inside a progress renderer.
 * @param {string} [options.sourcesFilePath] - must be inside /etc/apt/sources.list.d
 * @param {number} [options.lockTimeoutSeconds]
 * @param {number} [options.installTimeoutMs] - 0 disables, which is the default
 * @param {number|null} [options.euid]
 * @param {string} [options.platform]
 * @param {NodeJS.ProcessEnv} [options.env]
 * @param {string} [options.nodeExecPath]
 * @param {function} [options.runCommand] - injection seam for tests
 * @param {function} [options.runInteractiveCommand] - injection seam for tests
 * @param {function} [options.pathExists] - injection seam for tests
 * @param {function} [options.readFile] - injection seam for tests
 * @return {Promise<PackageUpgradeResult>}
 * @throws {PackageUpgradeError} on every failure. A rejection of any other type is a bug in this
 *   module, and a caller that sees one must treat it as PACKAGE_STATE.UNCERTAIN.
 */
export default async function upgradePackage(options = {}) {
  /**
   * @typedef {Object} PackageUpgradeResult
   * @property {string} method - install method the upgrade went through
   * @property {string} version - application version that was requested
   * @property {string} packageState - always PACKAGE_STATE.UPGRADED
   * @property {string} elevation - one of ROOT_ACCESS
   * @property {string|null} previousVersion - package version before, where the manager records one
   * @property {string} installedVersion - package version read back afterwards
   * @property {string} versionScheme - `debian` or `semver`. On the deb path installedVersion is
   *   a Debian version (`4.2.0~rc.3-1`) and is NOT comparable with what `dashmate --version`
   *   prints; `version` is the application version to compare against that.
   * @property {string} verifiedBy - what the installed version was read from
   * @property {string[]} warnings
   * @property {Array} commands - every invocation made, in order
   */
  const {
    installMethod,
    version,
    expectedDebVersion = null,
    binPath = null,
    npmGlobalPrefix = null,
    rootAccess = null,
    sourcesFilePath = DEFAULT_SOURCES_FILE_PATH,
    lockTimeoutSeconds = DEFAULT_LOCK_TIMEOUT_SECONDS,
    installTimeoutMs = DEFAULT_INSTALL_TIMEOUT_MS,
    euid = readEuid(),
    platform = process.platform,
    env = process.env,
    nodeExecPath = process.execPath,
    runCommand = defaultRunCommand,
    runInteractiveCommand = defaultRunInteractiveCommand,
    pathExists = defaultPathExists,
    readFile = defaultReadFile,
  } = options;

  const invalidInput = (summary) => new PackageUpgradeError({
    step: UPGRADE_STEP.VALIDATE_INPUT,
    summary,
    packageState: PACKAGE_STATE.UNCHANGED,
    elevation: isRunningAsRoot({ euid }) ? ROOT_ACCESS.ALREADY_ROOT : ROOT_ACCESS.NONE,
  });

  // Validated again even though the caller validated it. This is the last point before the
  // value becomes an argv token, and npm parses that token as a package specifier: `git+https`,
  // `file:`, a tarball URL and an `npm:` alias are all things npm would happily install.
  if (!isValidVersion(version)) {
    throw invalidInput(`"${sanitizeOutput(String(version)).slice(0, MAX_VERSION_LENGTH)}" is not a dashmate version.`);
  }

  if (installMethod !== INSTALL_METHOD_DEB && installMethod !== INSTALL_METHOD_NPM) {
    throw invalidInput(`dashmate cannot upgrade a "${sanitizeOutput(String(installMethod)).slice(0, 32)}" installation for you.`
      + ' Only the deb and npm channels can be driven; anything else is replaced by hand.');
  }

  if (installMethod === INSTALL_METHOD_DEB) {
    if (platform !== 'linux') {
      throw invalidInput(`A deb installation cannot be upgraded on ${sanitizeOutput(String(platform)).slice(0, 32)}.`);
    }

    if (!isValidDebVersion(expectedDebVersion)) {
      throw invalidInput('Upgrading a deb installation needs the exact Debian version to install.'
        + ' It cannot be derived from application semver here: the caller resolved the candidate from apt and is'
        + ' the only thing that knows which Debian version was approved.');
    }

    if (debUpstreamCore(expectedDebVersion) !== semverCore(version)) {
      throw invalidInput(`The Debian version "${expectedDebVersion}" was not built from dashmate ${version}.`);
    }

    if (!isUsableAbsolutePath(sourcesFilePath)
      || !isPrintableAscii(sourcesFilePath)
      || /\s/.test(sourcesFilePath)
      || path.dirname(sourcesFilePath) !== SOURCES_DIR) {
      throw invalidInput(`The apt sources file must be a plain absolute path inside ${SOURCES_DIR}.`);
    }
  }

  if (binPath !== null && !isUsableAbsolutePath(binPath)) {
    throw invalidInput('The dashmate executable path must be a normalized absolute path.');
  }

  if (npmGlobalPrefix !== null && !isUsableAbsolutePath(npmGlobalPrefix)) {
    throw invalidInput('The npm global prefix must be a normalized absolute path.');
  }

  if (!isUsableAbsolutePath(nodeExecPath)) {
    throw invalidInput('The node executable path must be a normalized absolute path.');
  }

  if (!Number.isInteger(lockTimeoutSeconds) || lockTimeoutSeconds < 0) {
    throw invalidInput('The dpkg lock timeout must be a whole number of seconds.');
  }

  if (!Number.isInteger(installTimeoutMs) || installTimeoutMs < 0) {
    throw invalidInput('The package manager timeout must be a whole number of milliseconds, or 0 for none.');
  }

  if (rootAccess !== null && !isValidRootAccess(rootAccess)) {
    throw invalidInput('The root access record is not one this module produced.'
      + ' Pass the result of ensureRootAccess unchanged, or omit it.');
  }

  const manualInstructions = installMethod === INSTALL_METHOD_DEB
    ? [`sudo apt-get install --only-upgrade ${PACKAGE_NAME}=${expectedDebVersion}`]
    : [`npm install -g ${PACKAGE_NAME}@${version}`];

  const context = {
    installMethod,
    version,
    expectedDebVersion,
    binPath,
    npmGlobalPrefix,
    sourcesFilePath,
    lockTimeoutSeconds,
    installTimeoutMs,
    env,
    nodeExecPath,
    runCommand,
    pathExists,
    readFile,
    probeEnv: buildProbeEnv(env),
    manualInstructions,
    recoveryInstructions: installMethod === INSTALL_METHOD_DEB
      ? ['sudo dpkg --configure -a', `sudo apt-get install --reinstall ${PACKAGE_NAME}`]
      : [`npm install -g ${PACKAGE_NAME}@${version}`],
    commands: [],
    warnings: [],
    packageManagerRan: false,
    rootAccess: null,
    // Read whatever the install method: an npm upgrade run under sudo puts the rest of the
    // command in /root/.dashmate just as surely as a deb one does, and this module's own EACCES
    // advice is what sends an operator to run it that way.
    elevation: isRunningAsRoot({ euid }) ? ROOT_ACCESS.ALREADY_ROOT : ROOT_ACCESS.NONE,
  };

  if (isRunningAsRoot({ euid })) {
    context.warnings.push(RUNNING_AS_ROOT_WARNING);
  }

  try {
    if (installMethod === INSTALL_METHOD_DEB) {
      context.rootAccess = rootAccess ?? await ensureRootAccess({
        euid,
        env,
        interactive: false,
        instructions: manualInstructions,
        sourcesFilePath,
        lockTimeoutSeconds,
        expectedDebVersion,
        runCommand,
        runInteractiveCommand,
        pathExists,
      });

      context.elevation = context.rootAccess.elevation;
      context.warnings = dedupe([...context.warnings, ...context.rootAccess.warnings]);
    }

    const outcome = installMethod === INSTALL_METHOD_DEB
      ? await upgradeDebPackage(context)
      : await upgradeNpmPackage(context);

    return {
      method: installMethod,
      version,
      packageState: PACKAGE_STATE.UPGRADED,
      elevation: context.elevation,
      warnings: context.warnings,
      commands: context.commands,
      ...outcome,
    };
  } catch (e) {
    if (e instanceof PackageUpgradeError) {
      throw e;
    }

    // Nothing may leave this module without a package state. A caller deciding whether to
    // activate a node reads that field, and an undefined one reads as "not uncertain".
    throw new PackageUpgradeError({
      step: UPGRADE_STEP.UNEXPECTED,
      summary: `dashmate could not complete the package upgrade: ${sanitizeOutput(String(e?.message ?? e))}`,
      packageState: context.packageManagerRan
        ? PACKAGE_STATE.UNCERTAIN
        : PACKAGE_STATE.UNCHANGED,
      instructions: context.packageManagerRan
        ? dedupe([...context.recoveryInstructions, ...manualInstructions])
        : manualInstructions,
      elevation: context.elevation,
      commands: context.commands,
    });
  }
}
