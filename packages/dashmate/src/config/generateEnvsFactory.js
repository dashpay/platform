const nodePath = require('path');
const os = require('os');
const convertObjectToEnvs = require('./convertObjectToEnvs');
const { DASHMATE_HELPER_DOCKER_IMAGE } = require('../constants');

/**
 * @param {ConfigFile} configFile
 * @param {HomeDir} homeDir
 * @return {generateEnvs}
 */
function generateEnvsFactory(configFile, homeDir) {
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
    const dockerComposeFiles = ['docker-compose.yml'];
    const profiles = [];

    profiles.push('core');

    if (config.get('core.masternode.enable')) {
      profiles.push('masternode');
    }

    if (config.get('dashmate.helper.docker.build.enabled')) {
      dockerComposeFiles.push('docker-compose.build.dashmate_helper.yml');
    }

    if (config.get('platform.enable')) {
      profiles.push('platform');

      if (config.get('platform.drive.abci.docker.build.enabled')) {
        dockerComposeFiles.push('docker-compose.build.drive_abci.yml');
      }

      if (config.get('platform.dapi.api.docker.build.enabled')) {
        dockerComposeFiles.push('docker-compose.build.dapi_api.yml');
        dockerComposeFiles.push('docker-compose.build.dapi_tx_filter_stream.yml');
      }
    }

    // we need this for compatibility with old configs
    const projectIdWithPrefix = configFile.getProjectId() ? `_${configFile.getProjectId()}` : '';

    const { uid, gid } = os.userInfo();

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
      CORE_LOG_DIRECTORY_PATH: nodePath.dirname(
        config.get('core.log.file.path'),
      ),
      DASHMATE_HELPER_DOCKER_IMAGE,
      PLATFORM_DRIVE_ABCI_LOG_PRETTY_DIRECTORY_PATH: nodePath.dirname(
        config.get('platform.drive.abci.log.prettyFile.path'),
      ),
      PLATFORM_DRIVE_ABCI_LOG_JSON_DIRECTORY_PATH: nodePath.dirname(
        config.get('platform.drive.abci.log.jsonFile.path'),
      ),
      PLATFORM_DRIVE_ABCI_LOG_PRETTY_FILE_NAME: nodePath.basename(
        config.get('platform.drive.abci.log.prettyFile.path'),
      ),
      PLATFORM_DRIVE_ABCI_LOG_JSON_FILE_NAME: nodePath.basename(
        config.get('platform.drive.abci.log.jsonFile.path'),
      ),
      ...convertObjectToEnvs(config.getOptions()),
    };
  }

  return generateEnvs;
}

module.exports = generateEnvsFactory;
