import { promises as fsPromises } from 'fs';
import semver from 'semver';

import getConfigFormatVersion from '../config/configFile/getConfigFormatVersion.js';
import { DASHMATE_HELPER_DOCKER_IMAGE, DASHMATE_VERSION } from '../constants.js';

/**
 * Whether a node is activated at the dashmate version that is currently running.
 *
 * "Activated" is decided from what the host actually shows, never from a stored record: a
 * record can be copied with a home directory, restored from a snapshot or simply go stale,
 * and it would then claim an activation that never happened. Two facts are enough, and both
 * are already written down by reality:
 *
 * - the config format the file on disk was last migrated to, and
 * - the image tag of the running helper container.
 *
 * The helper is decisive because its image is pinned to the exact package version, so it is
 * the only image whose *name* changes across a dashmate upgrade. Every other service runs a
 * floating tag (`dashd:23`, `tenderdash:1.6.0`, drive and rs-dapi at their major), which moves
 * under the same name and therefore proves nothing about which dashmate brought the node up.
 * Compose recreates a container whose image reference changed, so a helper running the current
 * package's tag proves the node came up under this binary.
 *
 * Nothing here throws, for any input. Both probes run on diagnostic and update paths where a
 * probe failure must not be the reason a command fails, so an unreadable config, an absent
 * docker daemon, a malformed container list and a caller passing nonsense all degrade to a
 * labelled unknown.
 *
 * Three-valued on purpose, and it fails closed. A caller must be able to tell "definitely not
 * activated" from "could not determine", and anything this module does not positively
 * recognise as activated is reported as unknown - never as activated.
 *
 * Cost: the migration probe is one file read and is host-wide, so it is cheap enough for any
 * path. The helper probe spawns `docker compose ps` per config, which costs on the order of
 * 100 ms in CLI startup alone before the daemon round trip, and more as a config's compose
 * file list grows. It belongs on the update and doctor paths, and must not be added to a path
 * that only prints a line of status text. The two probes are exported separately so the cheap
 * one can be run on its own.
 */

export const ACTIVATION_STATE = {
  ACTIVATED: 'activated',
  PENDING: 'pending',
  UNKNOWN: 'unknown',
};

export const ACTIVATION_REASON = {
  // migration probe
  CONFIG_FORMAT_CURRENT: 'config-format-current',
  CONFIG_FORMAT_BEHIND: 'config-format-behind',
  CONFIG_FORMAT_AHEAD: 'config-format-ahead',
  CONFIG_FILE_MISSING: 'config-file-missing',
  CONFIG_FILE_UNREADABLE: 'config-file-unreadable',
  CONFIG_FILE_MALFORMED: 'config-file-malformed',
  CONFIG_FORMAT_VERSION_INVALID: 'config-format-version-invalid',
  // helper probe
  HELPER_IMAGE_MATCHES: 'helper-image-matches',
  HELPER_IMAGE_MISMATCH: 'helper-image-mismatch',
  HELPER_NOT_RUNNING: 'helper-not-running',
  HELPER_BUILT_LOCALLY: 'helper-built-locally',
  HELPER_IMAGE_UNREADABLE: 'helper-image-unreadable',
  DOCKER_UNAVAILABLE: 'docker-unavailable',
  // either probe
  PROBE_FAILED: 'probe-failed',
};

const VALID_STATES = new Set(Object.values(ACTIVATION_STATE));

/**
 * Compose service name of the helper, as declared in docker-compose.yml.
 */
const HELPER_SERVICE_NAME = 'dashmate_helper';

/**
 * Image the helper runs when it is built from local sources.
 *
 * Hardcoded in docker-compose.build.dashmate_helper.yml, which the build override adds to
 * COMPOSE_FILE. Its name never changes, so it cannot witness an upgrade. It is recognised by
 * the image the container actually runs rather than by the config option that asks for it,
 * which keeps this probe answering about the node as it is: the option can be turned on while
 * a previously released helper is still up, and it is the running container that decides.
 */
const LOCALLY_BUILT_HELPER_IMAGE = 'dashmate-helper:local';

/**
 * @param {string} state
 * @param {string} reason
 * @param {string} message
 * @param {Object} [details]
 * @return {ActivationProbeResult}
 */
function result(state, reason, message, details = {}) {
  return {
    state, reason, message, ...details,
  };
}

/**
 * Describe a caught value without assuming it is an Error.
 *
 * A rejected promise carries whatever it was rejected with, including null, a string or a
 * symbol, and reading `.message` or stringifying it blindly is itself a way to throw out of a
 * function that promises never to.
 *
 * @param {*} caught
 * @return {string}
 */
function describeError(caught) {
  try {
    const message = caught?.message;

    if (typeof message === 'string' && message.length > 0) {
      return message;
    }

    return String(caught);
  } catch {
    return 'unknown error';
  }
}

/**
 * @param {*} value
 * @return {boolean}
 */
function isNonEmptyString(value) {
  return typeof value === 'string' && value.length > 0;
}

/**
 * @param {string} filePath
 * @return {Promise<string>}
 */
function defaultReadFile(filePath) {
  return fsPromises.readFile(filePath, 'utf8');
}

/**
 * Whether the config file on disk carries the config format this build produces.
 *
 * Reads the raw JSON rather than asking the loaded ConfigFile, and that is not an
 * optimisation: `ConfigFileJsonRepository.read()` migrates the whole document on every read in
 * every command, so by the time any command body runs the in-memory `configFormatVersion` is
 * already the target and would report every host as migrated. Only the bytes on disk say
 * whether the migration was ever persisted.
 *
 * One scalar for the whole file, so this is host-wide and answers for every config at once.
 *
 * A version *ahead* of what this build targets is reported as unknown rather than pending:
 * the config was migrated by a newer dashmate, `migrateConfigFile` returns early instead of
 * touching it, and no amount of re-running this binary will change that. It is a downgrade to
 * surface, not work to schedule.
 *
 * Versions are compared by precedence, never as strings: 4.9.0 precedes 4.10.0, a prerelease
 * precedes the release it leads to, and build metadata does not make two versions differ.
 *
 * @typedef {Object} ActivationProbeResult
 * @property {string} state - one of ACTIVATION_STATE
 * @property {string} reason - one of ACTIVATION_REASON
 * @property {string} message - operator-readable explanation
 *
 * @param {Object} options
 * @param {string} options.configFilePath - path of `config.json`, i.e. homeDir.joinPath('config.json')
 * @param {function(): Object} options.getConfigFileMigrations
 * @param {string} [options.dashmateVersion] - injection seam for tests
 * @param {function(string): Promise<string>} [options.readFile] - injection seam for tests
 * @return {Promise<ActivationProbeResult>} with `onDiskVersion` and `expectedVersion`
 */
export async function probeConfigFormatMigrated(options) {
  const details = { onDiskVersion: null, expectedVersion: null };

  try {
    const {
      configFilePath,
      getConfigFileMigrations,
      dashmateVersion = DASHMATE_VERSION,
      readFile = defaultReadFile,
    } = options ?? {};

    if (!isNonEmptyString(configFilePath)) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.PROBE_FAILED,
        'No config file path was given to probe',
        details,
      );
    }

    if (typeof getConfigFileMigrations !== 'function' || typeof readFile !== 'function') {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.PROBE_FAILED,
        'No config file migrations, or no way to read the config file, was given to probe',
        details,
      );
    }

    const expectedVersion = getConfigFormatVersion(getConfigFileMigrations(), dashmateVersion);

    if (!isNonEmptyString(expectedVersion) || semver.valid(expectedVersion) === null) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.PROBE_FAILED,
        `This dashmate reports ${expectedVersion} as the config format it produces, which is not a version`,
        details,
      );
    }

    details.expectedVersion = expectedVersion;

    let rawConfigFile;

    try {
      rawConfigFile = await readFile(configFilePath);
    } catch (e) {
      if (e?.code === 'ENOENT') {
        return result(
          ACTIVATION_STATE.UNKNOWN,
          ACTIVATION_REASON.CONFIG_FILE_MISSING,
          `Config file ${configFilePath} does not exist`,
          details,
        );
      }

      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.CONFIG_FILE_UNREADABLE,
        `Could not read ${configFilePath}: ${describeError(e)}`,
        details,
      );
    }

    let configFileData;

    try {
      configFileData = JSON.parse(rawConfigFile);
    } catch (e) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.CONFIG_FILE_MALFORMED,
        `Could not parse ${configFilePath}: ${describeError(e)}`,
        details,
      );
    }

    const onDiskVersion = configFileData?.configFormatVersion;

    if (!isNonEmptyString(onDiskVersion) || semver.valid(onDiskVersion) === null) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.CONFIG_FORMAT_VERSION_INVALID,
        `${configFilePath} does not carry a valid configFormatVersion`,
        details,
      );
    }

    details.onDiskVersion = onDiskVersion;

    if (semver.eq(onDiskVersion, expectedVersion)) {
      return result(
        ACTIVATION_STATE.ACTIVATED,
        ACTIVATION_REASON.CONFIG_FORMAT_CURRENT,
        `Config file is migrated to ${expectedVersion}`,
        details,
      );
    }

    if (semver.gt(onDiskVersion, expectedVersion)) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.CONFIG_FORMAT_AHEAD,
        `Config file is in format ${onDiskVersion}, which is newer than the ${expectedVersion} this dashmate produces`,
        details,
      );
    }

    return result(
      ACTIVATION_STATE.PENDING,
      ACTIVATION_REASON.CONFIG_FORMAT_BEHIND,
      `Config file is in format ${onDiskVersion} and has not been migrated to ${expectedVersion}`,
      details,
    );
  } catch (e) {
    return result(
      ACTIVATION_STATE.UNKNOWN,
      ACTIVATION_REASON.PROBE_FAILED,
      `Could not probe the config format version: ${describeError(e)}`,
      details,
    );
  }
}

/**
 * Whether this config's helper container is running the image this dashmate pins.
 *
 * Costs a child process per config. See the note at the top of this module.
 *
 * Images are compared for exact equality. A tag that merely contains the expected one - a
 * prerelease of it, or the same tag behind a registry prefix - is a different image and is
 * reported as a mismatch, because what the probe needs is the image compose would create right
 * now, not one that resembles it.
 *
 * Definite evidence is never discarded in favour of an unknown: a helper left on a known older
 * tag is a mismatch even when a container alongside it reports no image at all, and a locally
 * built helper is only reported once no running container contradicts it.
 *
 * A helper that is not running lands on pending rather than unknown: the container list is
 * answering, and what it says is that no helper is up on the expected image. A stopped node
 * reads the same way, which is correct - it has not come up under this version.
 *
 * @param {Config} config
 * @param {Object} options
 * @param {DockerCompose} options.dockerCompose
 * @param {string} [options.expectedHelperImage] - injection seam for tests
 * @return {Promise<ActivationProbeResult>} with `expectedImage` and `runningImage`
 */
export async function probeHelperRestarted(config, options) {
  const details = { expectedImage: null, runningImage: null };

  try {
    const {
      dockerCompose,
      expectedHelperImage = DASHMATE_HELPER_DOCKER_IMAGE,
    } = options ?? {};

    if (isNonEmptyString(expectedHelperImage)) {
      details.expectedImage = expectedHelperImage;
    }

    if (typeof config?.get !== 'function'
      || typeof dockerCompose?.getContainersList !== 'function'
      || !isNonEmptyString(expectedHelperImage)) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.PROBE_FAILED,
        'No config, no docker compose or no expected helper image was given to probe',
        details,
      );
    }

    let containers;

    try {
      containers = await dockerCompose.getContainersList(config, {
        filterServiceNames: HELPER_SERVICE_NAME,
      });
    } catch (e) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.DOCKER_UNAVAILABLE,
        `Could not list containers: ${describeError(e)}`,
        details,
      );
    }

    if (!Array.isArray(containers)) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.DOCKER_UNAVAILABLE,
        'Docker returned no usable container list',
        details,
      );
    }

    // Stopped containers are left out of the list unless they are asked for, so anything here
    // should be running. State is checked anyway: a helper that exists but is down is not a
    // node that came back up.
    const runningHelpers = containers.filter((container) => container?.State === 'running');

    if (runningHelpers.length === 0) {
      return result(
        ACTIVATION_STATE.PENDING,
        ACTIVATION_REASON.HELPER_NOT_RUNNING,
        'No running helper container was found for this config',
        details,
      );
    }

    const images = runningHelpers.map((container) => container?.Image);
    const readableImages = images.filter(isNonEmptyString);

    [details.runningImage = null] = readableImages;

    // Every running helper must be on the expected image, not merely one of them: a leftover
    // container on an older tag is exactly the state this probe exists to catch, and it stays
    // conclusive whatever the containers alongside it report.
    const mismatched = readableImages
      .find((image) => image !== expectedHelperImage && image !== LOCALLY_BUILT_HELPER_IMAGE);

    if (mismatched !== undefined) {
      details.runningImage = mismatched;

      return result(
        ACTIVATION_STATE.PENDING,
        ACTIVATION_REASON.HELPER_IMAGE_MISMATCH,
        `Helper container is running ${mismatched} instead of ${expectedHelperImage}`,
        details,
      );
    }

    const locallyBuilt = readableImages.find((image) => image === LOCALLY_BUILT_HELPER_IMAGE);

    if (locallyBuilt !== undefined) {
      details.runningImage = locallyBuilt;

      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.HELPER_BUILT_LOCALLY,
        `Helper container is running ${LOCALLY_BUILT_HELPER_IMAGE}, a tag that cannot witness an upgrade`,
        details,
      );
    }

    if (readableImages.length !== images.length) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.HELPER_IMAGE_UNREADABLE,
        'A running helper container reported no image',
        details,
      );
    }

    return result(
      ACTIVATION_STATE.ACTIVATED,
      ACTIVATION_REASON.HELPER_IMAGE_MATCHES,
      `Helper container is running ${expectedHelperImage}`,
      details,
    );
  } catch (e) {
    return result(
      ACTIVATION_STATE.UNKNOWN,
      ACTIVATION_REASON.PROBE_FAILED,
      `Could not probe the helper container: ${describeError(e)}`,
      details,
    );
  }
}

/**
 * Reduce anything a probe or a caller hands over to a result this module recognises.
 *
 * Fails closed. A result whose shape or state is not recognised - a truncated cache, a
 * hand-built object, a probe result from a future version of this module - becomes unknown,
 * because the one thing this module must never do is report an unrecognised value as
 * activated.
 *
 * @param {*} probe
 * @param {string} source
 * @return {ActivationProbeResult}
 */
function normaliseProbeResult(probe, source) {
  if (probe !== null
    && typeof probe === 'object'
    && VALID_STATES.has(probe.state)
    && isNonEmptyString(probe.reason)) {
    return probe;
  }

  return result(
    ACTIVATION_STATE.UNKNOWN,
    ACTIVATION_REASON.PROBE_FAILED,
    `The ${source} probe returned a result this dashmate does not recognise`,
  );
}

/**
 * Both probes for one config, combined into a single verdict.
 *
 * Activated requires both probes to say so explicitly. Definite negative evidence then wins
 * over an unknown: a config file that is provably behind means the node is not activated,
 * whatever the helper probe managed to see.
 *
 * A host loop should run the migration probe once and pass its result in, since one scalar
 * covers the whole file and re-reading it per config buys nothing.
 *
 * @param {Config} config
 * @param {Object} options - as accepted by both probes
 * @param {ActivationProbeResult} [options.migration] - a migration probe result already taken
 * @return {Promise<ActivationState>}
 */
export default async function probeActivationState(config, options) {
  /**
   * @typedef {Object} ActivationState
   * @property {string} state - one of ACTIVATION_STATE
   * @property {string} reason - reason of the probe that decided the state
   * @property {string} message - operator-readable explanation
   * @property {ActivationProbeResult} migration - config format probe
   * @property {ActivationProbeResult} helper - helper container probe
   */
  let migration = normaliseProbeResult(undefined, 'config format');
  let helper = normaliseProbeResult(undefined, 'helper container');

  try {
    const probeOptions = options ?? {};

    migration = normaliseProbeResult(
      probeOptions.migration ?? await probeConfigFormatMigrated(probeOptions),
      'config format',
    );

    helper = normaliseProbeResult(
      await probeHelperRestarted(config, probeOptions),
      'helper container',
    );
  } catch (e) {
    return {
      state: ACTIVATION_STATE.UNKNOWN,
      reason: ACTIVATION_REASON.PROBE_FAILED,
      message: `Could not probe activation state: ${describeError(e)}`,
      migration,
      helper,
    };
  }

  const probes = [migration, helper];

  if (probes.every((probe) => probe.state === ACTIVATION_STATE.ACTIVATED)) {
    const messages = probes.map((probe) => probe.message).filter(isNonEmptyString);

    return {
      state: ACTIVATION_STATE.ACTIVATED,
      reason: ACTIVATION_REASON.HELPER_IMAGE_MATCHES,
      message: messages.length > 0 ? messages.join(', ') : 'Node is activated at this dashmate version',
      migration,
      helper,
    };
  }

  const deciding = probes.find((probe) => probe.state === ACTIVATION_STATE.PENDING)
    ?? probes.find((probe) => probe.state === ACTIVATION_STATE.UNKNOWN)
    ?? normaliseProbeResult(undefined, 'activation');

  return {
    state: deciding.state,
    reason: deciding.reason,
    message: deciding.message,
    migration,
    helper,
  };
}
