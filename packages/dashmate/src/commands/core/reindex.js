import { Listr } from 'listr2';

import { Flags } from '@oclif/core';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';
import MuteOneLineError from '../../oclif/errors/MuteOneLineError.js';
import printUnwrapped from '../../oclif/errors/printUnwrapped.js';

export default class ReindexCommand extends ConfigBaseCommand {
  // Reindex temporarily replaces the Core service configuration, so other
  // configuration renderers must wait until it restores the ordinary files.
  static mutatesConfig = true;

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
   * @param {assertPlatformCanStart} assertPlatformCanStart
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
    assertPlatformCanStart,
  ) {
    // The reindex renders its own Core config and then starts the whole node.
    try {
      await assertPlatformCanStart(config);
    } catch (e) {
      throw printUnwrapped(e);
    }

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
