const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../oclif/command/BaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

class RestartCommand extends BaseCommand {
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
      update: isUpdate,
      'drive-image-build-path': driveImageBuildPath,
      'dapi-image-build-path': dapiImageBuildPath,
    },
    dockerCompose,
    startNodeTask,
    config,
  ) {
    const isMasternode = config.get('core.masternode.enable');

    const tasks = new Listr(
      [
        {
          title: 'Stop node',
          task: async () => dockerCompose.stop(config.toEnvs()),
        },
        {
          title: `Start ${isMasternode ? 'masternode' : 'full node'}`,
          task: () => startNodeTask(
            config,
            {
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

RestartCommand.description = `Restart masternode
...
Restart masternode with specific preset
`;

RestartCommand.flags = {
  ...BaseCommand.flags,
  update: flagTypes.boolean({ char: 'u', description: 'download updated services before start', default: false }),
  'drive-image-build-path': flagTypes.string({ description: 'drive\'s docker image build path', default: null }),
  'dapi-image-build-path': flagTypes.string({ description: 'dapi\'s docker image build path', default: null }),
};

module.exports = RestartCommand;
