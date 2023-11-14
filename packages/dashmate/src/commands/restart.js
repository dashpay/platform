import { Listr } from 'listr2';

import { Flags } from '@oclif/core';
import { ConfigBaseCommand } from '../oclif/command/ConfigBaseCommand.js';
import { MuteOneLineError } from '../oclif/errors/MuteOneLineError.js';

export class RestartCommand extends ConfigBaseCommand {
  static description = 'Restart node';

  static flags = {
    ...ConfigBaseCommand.flags,
    platform: Flags.boolean({ char: 'p', description: 'restart only platform', default: false }),
  };

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
      platform: platformOnly,
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
      await tasks.run({
        isVerbose,
        platformOnly: platformOnly === true,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
