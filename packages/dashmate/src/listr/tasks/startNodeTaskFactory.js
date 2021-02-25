const fs = require('fs');

const { Listr } = require('listr2');

const { PrivateKey } = require('@dashevo/dashcore-lib');
const { NETWORK_LOCAL, NETWORK_MAINNET } = require('../../constants');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @return {startNodeTask}
 */
function startNodeTaskFactory(dockerCompose) {
  /**
   * @typedef {startNodeTask}
   * @param {Config} config
   * @param {Object} [options]
   * @param {string} [options.driveImageBuildPath]
   * @param {string} [options.dapiImageBuildPath]
   * @param {boolean} [options.isUpdate]
   * @param {boolean} [options.isMinerEnabled]
   * @return {Object}
   */
  function startNodeTask(
    config,
    {
      driveImageBuildPath = undefined,
      dapiImageBuildPath = undefined,
      isUpdate = undefined,
      isMinerEnabled = undefined,
    } = {},
  ) {
    // Check external IP is set
    config.get('externalIp', true);

    if (isMinerEnabled === undefined) {
      // eslint-disable-next-line no-param-reassign
      isMinerEnabled = config.get('core.miner.enable');
    }

    if (isMinerEnabled === true && config.get('network') !== NETWORK_LOCAL) {
      throw new Error(`'core.miner.enabled' option only works with local network. Your network is ${config.get('network')}.`);
    }

    // Check Drive log files are created
    const prettyFilePath = config.get('platform.drive.abci.log.prettyFile.path');

    if (!fs.existsSync(prettyFilePath)) {
      fs.writeFileSync(prettyFilePath, '');
    }

    const jsonFilePath = config.get('platform.drive.abci.log.jsonFile.path');

    if (!fs.existsSync(jsonFilePath)) {
      fs.writeFileSync(jsonFilePath, '');
    }

    return new Listr([
      {
        title: 'Download updated services',
        enabled: () => isUpdate === true,
        task: async () => dockerCompose.pull(config.toEnvs()),
      },
      {
        title: 'Check node is not started',
        task: async () => {
          if (await dockerCompose.isServiceRunning(config.toEnvs())) {
            throw new Error('Running services detected. Please ensure all services are stopped for this config before starting');
          }
        },
      },
      {
        title: 'Start services',
        task: async () => {
          const isMasternode = config.get('core.masternode.enable');
          if (isMasternode) {
            // Check operatorPrivateKey is set
            config.get('core.masternode.operator.privateKey', true);
          }

          const envs = config.toEnvs();

          if (driveImageBuildPath || dapiImageBuildPath) {
            if (config.get('network') === NETWORK_MAINNET) {
              throw new Error('You can\'t use drive-image-build-path and dapi-image-build-path options with mainnet network');
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
