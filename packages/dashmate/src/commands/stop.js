const { Listr } = require('listr2');

const { Flags } = require('@oclif/core');

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
      force: isForce,
      verbose: isVerbose,
      platform: platformOnly,
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
      await tasks.run({
        isForce,
        isVerbose,
        platformOnly: platformOnly === true,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

StopCommand.description = 'Stop node';

StopCommand.flags = {
  ...ConfigBaseCommand.flags,
  force: Flags.boolean({
    char: 'f',
    description: 'force stop even if any service is running',
    default: false,
  }),
  platform: Flags.boolean({
    char: 'p',
    description: 'stop only platform',
    default: false,
  }),
};

module.exports = StopCommand;
