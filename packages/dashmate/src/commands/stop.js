const { Listr } = require('listr2');

const ConfigBaseCommand = require('../oclif/command/ConfigBaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

class StopCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {stopNodeTask} stopNodeTask
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
    },
    stopNodeTask,
    config,
  ) {
    const tasks = new Listr([
      {
        task: async () => stopNodeTask(config),
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
    });

    try {
      await tasks.run();
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

StopCommand.description = `Stop node

Stop node
`;

StopCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = StopCommand;
