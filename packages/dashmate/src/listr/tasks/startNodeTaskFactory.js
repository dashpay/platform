const { Listr } = require('listr2');

const NETWORKS = require('../../networks');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @return {startNodeTask}
 */
function startNodeTaskFactory(dockerCompose) {
  /**
   * @typedef {startNodeTask}
   * @param {Config} config
   * @param {Object} options
   * @param {boolean} [options.isFullNode]
   * @param {string} [options.driveImageBuildPath]
   * @param {string} [options.dapiImageBuildPath]
   * @param {boolean} [options.isUpdate]
   * @return {Object}
   */
  function startNodeTask(
    config,
    {
      isFullNode,
      driveImageBuildPath = undefined,
      dapiImageBuildPath = undefined,
      isUpdate = undefined,
    },
  ) {
    return new Listr([
      {
        title: 'Download updated services',
        enabled: () => isUpdate === true,
        task: async () => dockerCompose.pull(config.toEnvs()),
      },
      {
        title: 'Start services',
        task: async () => {
          if (!isFullNode) {
            config.get('core.masternode.operator.privateKey', true);
          }

          const envs = config.toEnvs();

          if (driveImageBuildPath || dapiImageBuildPath) {
            if (config.get('network') === NETWORKS.TESTNET) {
              throw new Error('You can\'t use drive-image-build-path and dapi-image-build-path options with testnet network');
            }

            if (driveImageBuildPath) {
              envs.COMPOSE_FILE += ':docker-compose.platform.build-drive.yml';
              envs.PLATFORM_DRIVE_DOCKER_IMAGE_BUILD_PATH = driveImageBuildPath;
            }

            if (dapiImageBuildPath) {
              envs.COMPOSE_FILE += ':docker-compose.platform.build-dapi.yml';
              envs.PLATFORM_DAPI_DOCKER_IMAGE_BUILD_PATH = dapiImageBuildPath;
            }
          }

          await dockerCompose.up(envs);
        },
      }]);
  }

  return startNodeTask;
}

module.exports = startNodeTaskFactory;
