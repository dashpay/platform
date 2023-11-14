import { Listr } from 'listr2';

import { Flags } from '@oclif/core';
import { ConfigBaseCommand } from '../../oclif/command/ConfigBaseCommand.js';
import { MuteOneLineError } from '../../oclif/errors/MuteOneLineError.js';

export class ReindexCommand extends ConfigBaseCommand {
  static description = 'Reindex Core data';

  static flags = {
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

  /**
   * @param {Object} args
   * @param {Object} flags
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
    config,
    reindexNodeTask,
  ) {
    const tasks = new Listr(
      [
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
      },
    );

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
