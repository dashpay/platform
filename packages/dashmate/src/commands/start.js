const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../oclif/command/BaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

const PRESETS = require('../presets');

class StartCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {startNodeTask} startNodeTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      preset,
      'external-ip': externalIp,
      'core-p2p-port': coreP2pPort,
    },
    {
      'full-node': isFullNode,
      update: isUpdate,
      'operator-private-key': operatorPrivateKey,
      'dpns-contract-id': dpnsContractId,
      'dpns-top-level-identity': dpnsTopLevelIdentity,
      'drive-image-build-path': driveImageBuildPath,
      'dapi-image-build-path': dapiImageBuildPath,
    },
    dockerCompose,
    startNodeTask,
  ) {
    const tasks = new Listr(
      [
        {
          title: `Start ${isFullNode ? 'full node' : 'masternode'} with ${preset} preset`,
          task: () => startNodeTask(
            preset,
            {
              externalIp,
              coreP2pPort,
              isFullNode,
              operatorPrivateKey,
              dpnsContractId,
              dpnsTopLevelIdentity,
              driveImageBuildPath,
              dapiImageBuildPath,
              isUpdate,
            },
          ),
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

StartCommand.args = [{
  name: 'preset',
  required: true,
  description: 'preset to use',
  options: Object.values(PRESETS),
}, {
  name: 'external-ip',
  required: true,
  description: 'masternode external IP',
}, {
  name: 'core-p2p-port',
  required: true,
  description: 'Core P2P port',
}];

StartCommand.flags = {
  'full-node': flagTypes.boolean({ char: 'f', description: 'start as full node', default: false }),
  update: flagTypes.boolean({ char: 'u', description: 'download updated services before start', default: false }),
  'operator-private-key': flagTypes.string({ char: 'p', description: 'operator private key', default: null }),
  'dpns-contract-id': flagTypes.string({ description: 'DPNS contract ID', default: null }),
  'dpns-top-level-identity': flagTypes.string({ description: 'DPNS top level identity', default: null }),
  'drive-image-build-path': flagTypes.string({ description: 'drive\'s docker image build path', default: null }),
  'dapi-image-build-path': flagTypes.string({ description: 'dapi\'s docker image build path', default: null }),
};

module.exports = StartCommand;
