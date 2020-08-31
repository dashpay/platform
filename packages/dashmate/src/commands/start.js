const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const { PrivateKey } = require('@dashevo/dashcore-lib');

const BaseCommand = require('../oclif/command/BaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

const NETWORKS = require('../networks');

class StartCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {startNodeTask} startNodeTask
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      'full-node': isFullNode,
      update: isUpdate,
      'drive-image-build-path': driveImageBuildPath,
      'dapi-image-build-path': dapiImageBuildPath,
    },
    dockerCompose,
    startNodeTask,
    config,
  ) {
    const isMinerEnabled = config.get('core.miner.enable');

    if (isMinerEnabled === true && config.get('network') !== NETWORKS.LOCAL) {
      this.error(`'core.miner.interval' option supposed to work only with local network. Your network is ${config.get('network')}`, { exit: true });
    }

    const tasks = new Listr(
      [
        {
          title: `Start ${isFullNode ? 'full node' : 'masternode'}`,
          task: () => startNodeTask(
            config,
            {
              isFullNode,
              driveImageBuildPath,
              dapiImageBuildPath,
              isUpdate,
            },
          ),
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
        },
      ],
      {
        rendererOptions: {
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
        },
      },
    );

    try {
      await tasks.run();
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

StartCommand.description = `Start masternode
...
Start masternode with specific preset
`;

StartCommand.flags = {
  ...BaseCommand.flags,
  'full-node': flagTypes.boolean({ char: 'f', description: 'start as full node', default: false }),
  update: flagTypes.boolean({ char: 'u', description: 'download updated services before start', default: false }),
  'drive-image-build-path': flagTypes.string({ description: 'drive\'s docker image build path', default: null }),
  'dapi-image-build-path': flagTypes.string({ description: 'dapi\'s docker image build path', default: null }),
};

module.exports = StartCommand;
