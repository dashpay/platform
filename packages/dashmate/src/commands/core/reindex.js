const { Listr } = require('listr2');

const { Flags } = require('@oclif/core');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

class ReindexCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {isSystemConfig} isSystemConfig
   * @param {Config} config
   * @param {reindexNodeTask} reindexNodeTask
   *
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
      force: isForce,
      detach: isDetached,
    },
    isSystemConfig,
    config,
    reindexNodeTask,
  ) {
    const tasks = new Listr([
      {
        title: `Reindex ${config.getName()} node`,
        task: () => reindexNodeTask(config),
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
        isDetached,
        isForce,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

ReindexCommand.description = 'Reindex Core data';

ReindexCommand.flags = {
  ...ConfigBaseCommand.flags,
  verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
  detach: Flags.boolean({
    char: 'd',
    description: 'run the reindex process in the background',
    default: false,
  }),
  force: Flags.boolean({
    char: 'f',
    description: 'reindex already running node without confirmation',
    default: false,
  }),
};

module.exports = ReindexCommand;
