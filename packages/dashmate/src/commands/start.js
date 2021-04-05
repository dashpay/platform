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
              isUpdate,
            },
          ),
        },
        {
          title: 'Wait for nodes to be ready',
          enabled: () => waitForReadiness,
          task: () => waitForNodeToBeReadyTask(config),
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          showTimer: isVerbose,
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
  'wait-for-readiness': flagTypes.boolean({ description: 'wait for nodes to be ready', default: false }),
};

module.exports = StartCommand;
