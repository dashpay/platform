const { Listr } = require('listr2');

const { PrivateKey } = require('@dashevo/dashcore-lib');

const NETWORKS = require('../../networks');

const wait = require('../../util/wait');

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
   * @param {boolean} [options.isMinerEnabled]
   * @return {Object}
   */
  function startNodeTask(
    config,
    {
      isFullNode,
      driveImageBuildPath = undefined,
      dapiImageBuildPath = undefined,
      isUpdate = undefined,
      isMinerEnabled = undefined,
    },
  ) {
    if (isMinerEnabled === true && config.get('network') !== NETWORKS.LOCAL) {
      this.error(`'core.miner.enabled' option supposed to work only with local network. Your network is ${config.get('network')}`, { exit: true });
    }

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

          // wait 10 seconds to ensure all services are running
          await wait(10000);
        },
      },
      {
        title: 'Start a miner',
        enabled: () => isMinerEnabled === true,
        task: async () => {
          let minerAddress = config.get('core.miner.address');

          if (minerAddress === null) {
            const privateKey = new PrivateKey();
            minerAddress = privateKey.toAddress('regtest').toString();

            config.set('core.miner.address', minerAddress);
          }

          const minerInterval = config.get('core.miner.interval');

          await dockerCompose.execCommand(
            config.toEnvs(),
            'core',
            [
              'bash',
              '-c',
              `while true; do dash-cli generatetoaddress 1 ${minerAddress}; sleep ${minerInterval}; done`,
            ],
            ['--detach'],
          );
        },
      }]);
  }

  return startNodeTask;
}

module.exports = startNodeTaskFactory;
