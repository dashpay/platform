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
 * Nothing here throws. Both probes run on diagnostic and update paths where a probe failure
 * must not be the reason a command fails, so an unreadable config, an absent docker daemon
 * and a malformed container list all degrade to a labelled unknown.
 *
 * Three-valued on purpose. A caller must be able to tell "definitely not activated" from
 * "could not determine", because unknown must never be counted as activated.
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

/**
 * Compose service name of the helper, as declared in docker-compose.yml.
 */
const HELPER_SERVICE_NAME = 'dashmate_helper';

/**
 * Config option that replaces the released helper image with one built from local sources.
 */
const HELPER_BUILD_ENABLED_OPTION = 'dashmate.helper.docker.build.enabled';

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
export async function probeConfigFormatMigrated(options = {}) {
  const {
    configFilePath,
    getConfigFileMigrations,
    dashmateVersion = DASHMATE_VERSION,
    readFile = defaultReadFile,
  } = options;

  const details = { onDiskVersion: null, expectedVersion: null };

  try {
    if (typeof configFilePath !== 'string' || configFilePath.length === 0) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.PROBE_FAILED,
        'No config file path was given to probe',
        details,
      );
    }

    if (typeof getConfigFileMigrations !== 'function') {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.PROBE_FAILED,
        'No config file migrations were given to probe against',
        details,
      );
    }

    details.expectedVersion = getConfigFormatVersion(getConfigFileMigrations(), dashmateVersion);

    let rawConfigFile;

    try {
      rawConfigFile = await readFile(configFilePath);
    } catch (e) {
      if (e.code === 'ENOENT') {
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
        `Could not read ${configFilePath}: ${e.message}`,
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
        `Could not parse ${configFilePath}: ${e.message}`,
        details,
      );
    }

    const onDiskVersion = configFileData?.configFormatVersion;

    if (typeof onDiskVersion !== 'string' || semver.valid(onDiskVersion) === null) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.CONFIG_FORMAT_VERSION_INVALID,
        `${configFilePath} does not carry a valid configFormatVersion`,
        details,
      );
    }

    details.onDiskVersion = onDiskVersion;

    if (semver.eq(onDiskVersion, details.expectedVersion)) {
      return result(
        ACTIVATION_STATE.ACTIVATED,
        ACTIVATION_REASON.CONFIG_FORMAT_CURRENT,
        `Config file is migrated to ${details.expectedVersion}`,
        details,
      );
    }

    if (semver.gt(onDiskVersion, details.expectedVersion)) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.CONFIG_FORMAT_AHEAD,
        `Config file is in format ${onDiskVersion}, which is newer than the ${details.expectedVersion} this dashmate produces`,
        details,
      );
    }

    return result(
      ACTIVATION_STATE.PENDING,
      ACTIVATION_REASON.CONFIG_FORMAT_BEHIND,
      `Config file is in format ${onDiskVersion} and has not been migrated to ${details.expectedVersion}`,
      details,
    );
  } catch (e) {
    return result(
      ACTIVATION_STATE.UNKNOWN,
      ACTIVATION_REASON.PROBE_FAILED,
      `Could not probe the config format version: ${e.message}`,
      details,
    );
  }
}

/**
 * Whether this config's helper container is running the image this dashmate pins.
 *
 * Costs a child process per config. See the note at the top of this module.
 *
 * Skipped when the helper is built from local sources: that compose override replaces the
 * released image with `dashmate-helper:local`, whose name never changes, so it cannot witness
 * an upgrade. Reported as its own unknown so it is never mistaken for pending - there is
 * nothing for the operator to do about it.
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
export async function probeHelperRestarted(config, options = {}) {
  const {
    dockerCompose,
    expectedHelperImage = DASHMATE_HELPER_DOCKER_IMAGE,
  } = options;

  const details = { expectedImage: expectedHelperImage, runningImage: null };

  try {
    if (typeof config?.get !== 'function' || typeof dockerCompose?.getContainersList !== 'function') {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.PROBE_FAILED,
        'No config or docker compose was given to probe',
        details,
      );
    }

    if (config.get(HELPER_BUILD_ENABLED_OPTION) === true) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.HELPER_BUILT_LOCALLY,
        'The helper is built from local sources, so its image tag cannot witness an upgrade',
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
        `Could not list containers: ${e.message}`,
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

    const images = runningHelpers.map((container) => container.Image);

    if (images.some((image) => typeof image !== 'string' || image.length === 0)) {
      return result(
        ACTIVATION_STATE.UNKNOWN,
        ACTIVATION_REASON.HELPER_IMAGE_UNREADABLE,
        'A running helper container reported no image',
        details,
      );
    }

    // Every running helper must be on the expected image, not merely one of them: a leftover
    // container on the old tag is exactly the state this probe exists to catch.
    const mismatched = images.find((image) => image !== expectedHelperImage);

    [details.runningImage] = images;

    if (mismatched !== undefined) {
      details.runningImage = mismatched;

      return result(
        ACTIVATION_STATE.PENDING,
        ACTIVATION_REASON.HELPER_IMAGE_MISMATCH,
        `Helper container is running ${mismatched} instead of ${expectedHelperImage}`,
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
      `Could not probe the helper container: ${e.message}`,
      details,
    );
  }
}

/**
 * Both probes for one config, combined into a single verdict.
 *
 * Definite negative evidence wins over an unknown: a config file that is provably behind means
 * the node is not activated, whatever the helper probe managed to see. Unknown only survives
 * when no probe found anything definite, and it is never reported as activated.
 *
 * A host loop should run the migration probe once and pass its result in, since one scalar
 * covers the whole file and re-reading it per config buys nothing.
 *
 * @param {Config} config
 * @param {Object} options - as accepted by both probes
 * @param {ActivationProbeResult} [options.migration] - a migration probe result already taken
 * @return {Promise<ActivationState>}
 */
export default async function probeActivationState(config, options = {}) {
  /**
   * @typedef {Object} ActivationState
   * @property {string} state - one of ACTIVATION_STATE
   * @property {string} reason - reason of the probe that decided the state
   * @property {string} message - operator-readable explanation
   * @property {ActivationProbeResult} migration - config format probe
   * @property {ActivationProbeResult} helper - helper container probe
   */
  const migration = options.migration ?? await probeConfigFormatMigrated(options);
  const helper = await probeHelperRestarted(config, options);

  const probes = [migration, helper];

  const deciding = probes.find((probe) => probe.state === ACTIVATION_STATE.PENDING)
    ?? probes.find((probe) => probe.state === ACTIVATION_STATE.UNKNOWN);

  if (deciding === undefined) {
    return {
      state: ACTIVATION_STATE.ACTIVATED,
      reason: ACTIVATION_REASON.HELPER_IMAGE_MATCHES,
      message: `${migration.message}, ${helper.message}`,
      migration,
      helper,
    };
  }

  return {
    state: deciding.state,
    reason: deciding.reason,
    message: deciding.message,
    migration,
    helper,
  };
}
