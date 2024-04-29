import path from 'path';
import os from 'os';
import convertObjectToEnvs from './convertObjectToEnvs.js';
import { DASHMATE_HELPER_DOCKER_IMAGE } from '../constants.js';

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
   * CORE_LOG_DIRECTORY_PATH: string
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

    // Determine logs directory to mount into tenderdash container
    let tenderdashLogDirectoryPath = homeDir.joinPath('logs', config.get('network'));
    const tenderdashLogFilePath = config.get('platform.drive.tenderdash.log.path');
    if (tenderdashLogFilePath !== null) {
      tenderdashLogDirectoryPath = path.dirname(tenderdashLogFilePath);
    }

    let driveAbciMetricsUrl = '';
    if (config.get('platform.drive.abci.metrics.enabled')) {
      driveAbciMetricsUrl = 'http://0.0.0.0:29090';
    }

    return {
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
      CORE_LOG_DIRECTORY_PATH: path.dirname(
        config.get('core.log.file.path'),
      ),
      DASHMATE_HELPER_DOCKER_IMAGE,
      PLATFORM_DRIVE_TENDERDASH_LOG_DIRECTORY_PATH: tenderdashLogDirectoryPath,
      PLATFORM_GATEWAY_RATE_LIMITER_METRICS_DISABLED: !config.get('platform.gateway.rateLimiter.metrics.enabled'),
      PLATFORM_DRIVE_ABCI_METRICS_URL: driveAbciMetricsUrl,
      ...convertObjectToEnvs(config.getOptions()),
    };
  }

  return generateEnvs;
}
