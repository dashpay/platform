import os from 'os';
import path from 'path';
import { DASHMATE_HELPER_DOCKER_IMAGE } from '../constants.js';
import convertObjectToEnvs from './convertObjectToEnvs.js';

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

      if (config.get('platform.dapi.api.docker.build.enabled')) {
        dockerComposeFiles.push('docker-compose.build.dapi_api.yml');
        dockerComposeFiles.push('docker-compose.build.dapi_core_streams.yml');
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
      ...convertObjectToEnvs(config.getOptions()),
    };

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

    return envs;
  }

  return generateEnvs;
}
