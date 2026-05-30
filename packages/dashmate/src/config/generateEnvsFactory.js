import os from 'os';
import path from 'path';
import { DASHMATE_HELPER_DOCKER_IMAGE, NETWORK_LOCAL } from '../constants.js';
import convertObjectToEnvs from './convertObjectToEnvs.js';

/**
 * Maps dashmate network name to Dash Core network name.
 * @param {string} network - dashmate network (local, devnet, testnet, mainnet)
 * @returns {string} Dash Core network name (regtest, devnet, testnet, mainnet)
 */
function getDashCoreNetwork(network) {
  if (network === NETWORK_LOCAL) {
    return 'regtest';
  }
  return network;
}

/**
 * @param {ConfigFile} configFile
 * @param {HomeDir} homeDir
 * @param {getConfigProfiles} getConfigProfiles
 * @return {generateEnvs}
 */
export default function generateEnvsFactory(configFile, homeDir, getConfigProfiles) {
  /**
   * @typedef {function} generateEnvs
   * @param {Config} config
   * @returns {{
   * COMPOSE_DOCKER_CLI_BUILD: number,
   * CONFIG_NAME: string,
   * DOCKER_BUILDKIT: number,
   * COMPOSE_PROJECT_NAME: string,
   * COMPOSE_FILE: string,
   * COMPOSE_PATH_SEPARATOR: string,
   * }}
   */
  function generateEnvs(config) {
    const dynamicComposePath = homeDir.joinPath(
      config.getName(),
      'dynamic-compose.yml',
    );

    const dockerComposeFiles = ['docker-compose.yml', dynamicComposePath];

    const profiles = getConfigProfiles(config);

    if (config.get('dashmate.helper.docker.build.enabled')) {
      dockerComposeFiles.push('docker-compose.build.dashmate_helper.yml');
    }

    if (config.get('platform.enable')) {
      if (config.get('platform.drive.abci.docker.build.enabled')) {
        dockerComposeFiles.push('docker-compose.build.drive_abci.yml');
      }

      if (config.get('platform.dapi.rsDapi.docker.build.enabled')) {
        dockerComposeFiles.push('docker-compose.build.rs-dapi.yml');
      }
    }

    if (config.get('core.insight.enabled')) {
      let insightComposeFile = 'docker-compose.insight_api.yml';
      if (config.get('core.insight.ui.enabled')) {
        insightComposeFile = 'docker-compose.insight_ui.yml';
      }
      dockerComposeFiles.push(insightComposeFile);
    }

    if (config.get('platform.gateway.rateLimiter.enabled')) {
      dockerComposeFiles.push('docker-compose.rate_limiter.yml');

      if (config.get('platform.gateway.rateLimiter.metrics.enabled')) {
        dockerComposeFiles.push('docker-compose.rate_limiter.metrics.yml');
      }
    }

    // we need this for compatibility with old configs
    const projectIdWithPrefix = configFile.getProjectId() ? `_${configFile.getProjectId()}` : '';

    const { uid, gid } = os.userInfo();

    let driveAbciMetricsUrl = '';
    if (config.get('platform.drive.abci.metrics.enabled')) {
      // IP and port inside container
      driveAbciMetricsUrl = 'http://0.0.0.0:29090';
    }

    const envs = {
      DASHMATE_HOME_DIR: homeDir.getPath(),
      LOCAL_UID: uid,
      LOCAL_GID: gid,
      COMPOSE_PROJECT_NAME: `dashmate${projectIdWithPrefix}_${config.getName()}`,
      CONFIG_NAME: config.getName(),
      COMPOSE_FILE: dockerComposeFiles.join(':'),
      COMPOSE_PROFILES: profiles.join(','),
      COMPOSE_PATH_SEPARATOR: ':',
      DOCKER_BUILDKIT: 1,
      COMPOSE_DOCKER_CLI_BUILD: 1,
      DASHMATE_HELPER_DOCKER_IMAGE,
      PLATFORM_GATEWAY_RATE_LIMITER_METRICS_DISABLED: !config.get('platform.gateway.rateLimiter.metrics.enabled'),
      PLATFORM_DRIVE_ABCI_METRICS_URL: driveAbciMetricsUrl,
      DASH_CORE_NETWORK: getDashCoreNetwork(config.get('network')),
      ...convertObjectToEnvs(config.getOptions()),
    };

    // Forward extra docker `build.args` declared per-image in dashmate config
    // (`platform.drive.abci.docker.build.buildArgs`,
    // `platform.dapi.rsDapi.docker.build.buildArgs`) as env vars under the
    // arg name. `docker-compose.build.*.yml` reads them via `${NAME}`
    // substitution and forwards them into the Dockerfile `ARG`.
    //
    // Dashmate config is the single source of truth for build args — do NOT
    // fall back to `process.env[key]`. Operators who need a per-invocation
    // override should `yarn dashmate config set ...` rather than `FOO=bar
    // yarn start`. Keeping this single-source matches the `yarn setup` flow
    // that writes SDK_TEST_DATA into the local config automatically.
    //
    // drive-abci and rs-dapi share the workspace so a shared key like
    // CARGO_BUILD_PROFILE typically wants the same value in both; the merge
    // order below means drive-abci's value wins on collision.
    const getBuildArgs = (configPath) => (
      config.has(configPath) ? (config.get(configPath) || {}) : {}
    );
    const mergedBuildArgs = {
      ...getBuildArgs('platform.dapi.rsDapi.docker.build.buildArgs'),
      ...getBuildArgs('platform.drive.abci.docker.build.buildArgs'),
    };
    const reservedEnvKeys = new Set([
      'COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES', 'COMPOSE_PATH_SEPARATOR',
      'DOCKER_BUILDKIT', 'COMPOSE_DOCKER_CLI_BUILD', 'CONFIG_NAME', 'DASHMATE_HOME_DIR', 'LOCAL_UID', 'LOCAL_GID',
    ]);
    for (const [key, value] of Object.entries(mergedBuildArgs)) {
      // don't let buildArgs clobber reserved compose/runtime envs
      if (reservedEnvKeys.has(key)) {
        continue;
      }
      envs[key] = value;
    }

    const configuredAccessLogPath = config.get('platform.dapi.rsDapi.logs.accessLogPath');
    const hasConfiguredPath = typeof configuredAccessLogPath === 'string'
      && configuredAccessLogPath.trim() !== '';

    const containerAccessLogDir = '/var/log/rs-dapi';
    let containerAccessLogPath = path.posix.join(containerAccessLogDir, 'access.log');
    let accessLogVolumeType = 'volume';
    let accessLogVolumeSource = 'rs-dapi-access-logs';

    envs.PLATFORM_DAPI_RS_DAPI_LOGS_ACCESS_LOG_HOST_PATH = '';
    envs.PLATFORM_DAPI_RS_DAPI_LOGS_ACCESS_LOG_HOST_DIR = '';

    if (hasConfiguredPath) {
      const homeDirPath = homeDir.getPath();

      const hostAccessLogPath = path.isAbsolute(configuredAccessLogPath)
        ? configuredAccessLogPath
        : path.resolve(homeDirPath, configuredAccessLogPath);

      const hostAccessLogDir = path.dirname(hostAccessLogPath);
      const hostAccessLogFile = path.basename(hostAccessLogPath);

      containerAccessLogPath = path.posix.join(containerAccessLogDir, hostAccessLogFile);
      accessLogVolumeType = 'bind';
      accessLogVolumeSource = hostAccessLogDir;

      envs.PLATFORM_DAPI_RS_DAPI_LOGS_ACCESS_LOG_HOST_PATH = hostAccessLogPath;
      envs.PLATFORM_DAPI_RS_DAPI_LOGS_ACCESS_LOG_HOST_DIR = hostAccessLogDir;
    }

    envs.PLATFORM_DAPI_RS_DAPI_LOGS_ACCESS_LOG_CONTAINER_DIR = containerAccessLogDir;
    envs.PLATFORM_DAPI_RS_DAPI_LOGS_ACCESS_LOG_CONTAINER_PATH = containerAccessLogPath;
    envs.PLATFORM_DAPI_RS_DAPI_LOGS_ACCESS_LOG_VOLUME_TYPE = accessLogVolumeType;
    envs.PLATFORM_DAPI_RS_DAPI_LOGS_ACCESS_LOG_VOLUME_SOURCE = accessLogVolumeSource;

    if (hasConfiguredPath) {
      envs.PLATFORM_DAPI_RS_DAPI_LOGS_ACCESS_LOG_PATH = containerAccessLogPath;
    } else {
      envs.PLATFORM_DAPI_RS_DAPI_LOGS_ACCESS_LOG_PATH = '';
    }

    if (
      config.has('platform.dapi.rsDapi.metrics.enabled')
      && !config.get('platform.dapi.rsDapi.metrics.enabled')
    ) {
      envs.PLATFORM_DAPI_RS_DAPI_METRICS_PORT = '0';
    }

    return envs;
  }

  return generateEnvs;
}
