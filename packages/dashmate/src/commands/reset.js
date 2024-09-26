import { Listr } from 'listr2';

import { Flags } from '@oclif/core';
import ConfigBaseCommand from '../oclif/command/ConfigBaseCommand.js';
import MuteOneLineError from '../oclif/errors/MuteOneLineError.js';

export default class ResetCommand extends ConfigBaseCommand {
  static description = 'Reset node data';

  static flags = {
    ...ConfigBaseCommand.flags,
    hard: Flags.boolean({ char: 'h', description: 'reset config as well as services and data', default: false }),
    force: Flags.boolean({ char: 'f', description: 'skip running services check', default: false }),
    platform: Flags.boolean({ char: 'p', description: 'reset platform services and data only', default: false }),
    verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
    'keep-data': Flags.boolean({ description: 'keep data', default: false }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {resetNodeTask} resetNodeTask
   *
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
      hard: isHardReset,
      force: isForce,
      platform: isPlatformOnlyReset,
      'keep-data': keepData,
    },
    config,
    resetNodeTask,
  ) {
    const tasks = new Listr(
      [
        {
          enabled: (ctx) => !ctx.isForce,
          task: async (ctx, task) => {
            let message;
            if (ctx.isHardReset) {
              if (ctx.keepData) {
                message = 'Are you sure you want to reset your node configuration? Data will be'
                  + ' kept.';
                if (ctx.isPlatformOnlyReset) {
                  message = 'Are you sure you want to reset platform related configuration? Data'
                    + ' will be kept';
                }
              } else {
                message = 'Are you sure you want to reset your node data and configuration?';
                if (ctx.isPlatformOnlyReset) {
                  message = 'Are you sure you want to reset platform related data and configuration?';
                }
              }
            } else if (ctx.keepData) {
              message = 'Are you sure you want to reset docker containers?';
              if (ctx.isPlatformOnlyReset) {
                message = 'Are you sure you want to reset platform related docker containers?';
              }
            } else {
              message = 'Are you sure you want to reset you node data?';
              if (ctx.isPlatformOnlyReset) {
                message = 'Are you sure you want to reset platform related data?';
              }
            }

            const agreement = await task.prompt({
              type: 'toggle',
              name: 'confirm',
              message,
              enabled: 'Yes',
              disabled: 'No',
            });

            if (!agreement) {
              throw new Error('Archive creation was declined');
            }
          },
        },
        {
          title: `Reset ${config.getName()} node`,
          task: () => resetNodeTask(config),
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
        isHardReset,
        isPlatformOnlyReset,
        isForce,
        isVerbose,
        keepData,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
