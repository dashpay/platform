const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../oclif/command/BaseCommand');
const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

const PRESETS = require('../presets');

class SetupForLocalDevelopmentCommand extends BaseCommand {
  /**
   *
   * @param {Object} args
   * @param {Object} flags
   * @param {generateToAddressTask} generateToAddressTask
   * @param {registerMasternodeTask} registerMasternodeTask
   * @param {initTask} initTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    { port: coreP2pPort, 'external-ip': externalIp },
    {
      'drive-image-build-path': driveImageBuildPath,
      'dapi-image-build-path': dapiImageBuildPath,
    },
    generateToAddressTask,
    registerMasternodeTask,
    initTask,
  ) {
    const preset = PRESETS.LOCAL;
    const network = preset;
    const amount = 10000;

    const tasks = new Listr(
      [
        {
          title: 'Setup masternode for local development',
          task: () => new Listr([
            {
              title: `Generate ${amount} dash to address`,
              task: () => generateToAddressTask(preset, amount),
            },
            {
              title: 'Register masternode',
              task: () => registerMasternodeTask(preset),
            },
            {
              title: 'Initialize Platform',
              task: () => initTask(preset),
            },
          ]),
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
      await tasks.run({
        externalIp,
        coreP2pPort,
        network,
        driveImageBuildPath,
        dapiImageBuildPath,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

SetupForLocalDevelopmentCommand.description = `Setup for development
...
Generate some dash, register masternode and populate node with data required for local development
`;

SetupForLocalDevelopmentCommand.args = [{
  name: 'external-ip',
  required: true,
  description: 'masternode external IP',
}, {
  name: 'port',
  required: true,
  description: 'masternode P2P port',
}];

SetupForLocalDevelopmentCommand.flags = {
  'drive-image-build-path': flagTypes.string({ description: 'drive\'s docker image build path', default: null }),
  'dapi-image-build-path': flagTypes.string({ description: 'dapi\'s docker image build path', default: null }),
};

module.exports = SetupForLocalDevelopmentCommand;
