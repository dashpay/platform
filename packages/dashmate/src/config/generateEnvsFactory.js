const nodePath = require('path');
const os = require('os');
const convertObjectToEnvs = require('./convertObjectToEnvs');
const { DASHMATE_HELPER_DOCKER_IMAGE } = require('../constants');

/**
 * @param {ConfigFile} configFile
 * @param {HomeDir} homeDir
 * @param {getConfigProfiles} getConfigProfiles
 * @return {generateEnvs}
 */
function generateEnvsFactory(configFile, homeDir, getConfigProfiles) {
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
        dockerComposeFiles.push('docker-compose.build.dapi_tx_filter_stream.yml');
      }

      const fileLogs = Object.entries(config.get('platform.drive.abci.logs')).filter(([, settings]) => (
        settings.destination !== 'stdout' && settings.destination !== 'stderr'
      ));

      if (fileLogs.length > 0) {
        const composeVolumesPath = homeDir.joinPath(
          config.getName(),
          'platform',
          'drive',
          'abci',
          'compose-volumes.yml',
        );

        dockerComposeFiles.push(composeVolumesPath);
      }
    }

    // we need this for compatibility with old configs
    const projectIdWithPrefix = configFile.getProjectId() ? `_${configFile.getProjectId()}` : '';

    const { uid, gid } = os.userInfo();

    // Determine logs directory to mount into tenderdash container
    let tenderdashLogDirectoryPath = homeDir.joinPath('logs', config.get('network'));
    const tenderdashLogFilePath = config.get('platform.drive.tenderdash.log.path');
    if (tenderdashLogFilePath !== null) {
      tenderdashLogDirectoryPath = nodePath.dirname(tenderdashLogFilePath);
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
      CORE_LOG_DIRECTORY_PATH: nodePath.dirname(
        config.get('core.log.file.path'),
      ),
      DASHMATE_HELPER_DOCKER_IMAGE,
      PLATFORM_DRIVE_TENDERDASH_LOG_DIRECTORY_PATH: tenderdashLogDirectoryPath,
      ...convertObjectToEnvs(config.getOptions()),
    };
  }

  return generateEnvs;
}

module.exports = generateEnvsFactory;
