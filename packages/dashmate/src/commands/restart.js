const { Listr } = require('listr2');

const ConfigBaseCommand = require('../oclif/command/ConfigBaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

class RestartCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {restartNodeTask} restartNodeTask
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
    },
    dockerCompose,
    restartNodeTask,
    config,
  ) {
    const tasks = new Listr(
      [
        {
          title: `Restarting ${config.getName()} node`,
          task: () => restartNodeTask(config),
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

RestartCommand.description = `Restart node
...
Restart node
`;

RestartCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = RestartCommand;
