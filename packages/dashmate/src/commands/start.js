const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const ConfigBaseCommand = require('../oclif/command/ConfigBaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

class StartCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {startNodeTask} startNodeTask
   * @param {waitForNodeToBeReadyTask} waitForNodeToBeReadyTask
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      update: isUpdate,
      'drive-image-build-path': driveImageBuildPath,
      'dapi-image-build-path': dapiImageBuildPath,
      'wait-for-readiness': waitForReadiness,
      verbose: isVerbose,
    },
    dockerCompose,
    startNodeTask,
    waitForNodeToBeReadyTask,
    config,
  ) {
    const tasks = new Listr(
      [
        {
          title: `Start ${config.getName()} node`,
          task: () => startNodeTask(
            config,
            {
              driveImageBuildPath,
              dapiImageBuildPath,
              isUpdate,
            },
          ),
        },
        {
          title: 'Wait for nodes to be ready',
          enabled: waitForReadiness,
          task: () => waitForNodeToBeReadyTask(config),
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
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

StartCommand.description = `Start node
...
Start node
`;

StartCommand.flags = {
  ...ConfigBaseCommand.flags,
  update: flagTypes.boolean({ char: 'u', description: 'download updated services before start', default: false }),
  'drive-image-build-path': flagTypes.string({ description: 'drive\'s docker image build path', default: null }),
  'dapi-image-build-path': flagTypes.string({ description: 'dapi\'s docker image build path', default: null }),
  'wait-for-readiness': flagTypes.boolean({ description: 'wait for nodes to be ready', default: false }),
};

module.exports = StartCommand;
