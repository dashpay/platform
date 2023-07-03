const nodePath = require('path');
const convertObjectToEnvs = require('../config/convertObjectToEnvs');

/**
 *
 * @param {ConfigFile} configFile
 * @param {Config} config
 * @param {Object} [options={}]
 * @param {boolean} [options.platformOnly=false]
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
function generateEnvs(configFile, config, options = {}) {
  const dockerComposeFiles = [];

  dockerComposeFiles.push('docker-compose.yml');

  if (config.get('dashmate.helper.dockerBuild.context') !== null) {
    dockerComposeFiles.push('docker-compose.platform.build.dashmate_helper.yml');
  }

  if (!options.platformOnly) {
    dockerComposeFiles.push('docker-compose.core.yml');

    if (config.get('core.masternode.enable') === true) {
      dockerComposeFiles.push('docker-compose.sentinel.yml');
    }
  }

  if (config.get('platform.enable')) {
    dockerComposeFiles.push('docker-compose.platform.yml');

    if (config.get('platform.drive.abci.dockerBuild.context') !== null) {
      dockerComposeFiles.push('docker-compose.platform.build.drive_abci.yml');
    }

    if (config.get('platform.dapi.api.dockerBuild.context') !== null) {
      dockerComposeFiles.push('docker-compose.platform.build.dapi_api.yml');
    }

    if (config.get('platform.dapi.api.dockerBuild.context') !== null) {
      dockerComposeFiles.push('docker-compose.platform.build.dapi_tx_filter_stream.yml');
    }

    if (config.get('platform.dapi.envoy.dockerBuild.context') !== null) {
      dockerComposeFiles.push('docker-compose.platform.build.dapi_envoy.yml');
    }
  }

  // we need this for compatibility with old configs
  const projectIdWithPrefix = configFile.getProjectId() ? `_${configFile.getProjectId()}` : '';

  let envs = {
    COMPOSE_PROJECT_NAME: `dashmate${projectIdWithPrefix}_${config.getName()}`,
    CONFIG_NAME: config.getName(),
    COMPOSE_FILE: dockerComposeFiles.join(':'),
    COMPOSE_PATH_SEPARATOR: ':',
    DOCKER_BUILDKIT: 1,
    COMPOSE_DOCKER_CLI_BUILD: 1,
    CORE_LOG_DIRECTORY_PATH: nodePath.dirname(
      config.get('core.log.file.path'),
    ),
    ...convertObjectToEnvs(config.getOptions()),
  };

  if (config.get('platform.enable')) {
    envs = {
      ...envs,

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
    };
  }

  return envs;
}

module.exports = generateEnvs;
