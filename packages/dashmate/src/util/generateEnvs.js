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

  if (!options.platformOnly) {
    // TODO: it should contain only the dashmate helper that must be ran always
    dockerComposeFiles.push('docker-compose.yml');
  }

  if (config.get('platform.enable')) {
    dockerComposeFiles.push('docker-compose.platform.yml');

    if (config.get('platform.sourcePath') !== null) {
      dockerComposeFiles.push('docker-compose.platform.build.yml');
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
