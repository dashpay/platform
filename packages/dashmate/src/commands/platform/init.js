const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../../oclif/command/BaseCommand');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

const PRESETS = require('../../presets');

class InitCommand extends BaseCommand {
  /**
   *
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {initTask} initTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      preset,
      'funding-private-key': fundingPrivateKeyString,
      'operator-private-key': operatorPrivateKey,
      'external-ip': externalIp,
      'core-p2p-port': coreP2pPort,
    },
    {
      'drive-image-build-path': driveImageBuildPath,
      'dapi-image-build-path': dapiImageBuildPath,
    },
    dockerCompose,
    initTask,
  ) {
    const network = preset;

    const tasks = new Listr([{
      title: `Initialize Platform for ${preset} preset`,
      task: () => initTask(preset),
    },
    ],
    {
      rendererOptions: {
        clearOutput: false,
        collapse: false,
        showSubtasks: true,
      },
    });

    try {
      await tasks.run({
        fundingPrivateKeyString,
        externalIp,
        coreP2pPort,
        operator: {
          privateKey: operatorPrivateKey,
        },
        network,
        driveImageBuildPath,
        dapiImageBuildPath,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

InitCommand.description = `Initialize platform
...
Register DPNS Contract and "dash" top-level domain
`;

InitCommand.args = [{
  name: 'preset',
  required: true,
  description: 'preset to use',
  options: Object.values(PRESETS),
}, {
  name: 'funding-private-key',
  required: true,
  description: 'private key with dash for funding account',
}, {
  name: 'operator-private-key',
  required: true,
  description: 'operator private key',
}, {
  name: 'external-ip',
  required: true,
  description: 'masternode external IP',
}, {
  name: 'core-p2p-port',
  required: true,
  description: 'Core P2P port',
}];

InitCommand.flags = {
  'drive-image-build-path': flagTypes.string({ description: 'drive\'s docker image build path', default: null }),
  'dapi-image-build-path': flagTypes.string({ description: 'dapi\'s docker image build path', default: null }),
};

module.exports = InitCommand;
