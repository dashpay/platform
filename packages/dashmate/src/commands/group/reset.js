import { Listr } from 'listr2';
import { Flags } from '@oclif/core';
import { GroupBaseCommand } from '../../oclif/command/GroupBaseCommand.js';
import { MuteOneLineError } from '../../oclif/errors/MuteOneLineError.js';
import { PRESET_LOCAL } from '../../constants.js';

export class GroupResetCommand extends GroupBaseCommand {
  static description = 'Reset group nodes';

  static flags = {
    ...GroupBaseCommand.flags,
    hard: Flags.boolean({
      description: 'reset config as well as data',
      default: false,
    }),
    force: Flags.boolean({
      char: 'f',
      description: 'reset even running node',
      default: false,
    }),
    platform: Flags.boolean({
      char: 'p',
      description: 'reset platform services and data only',
      default: false,
    }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {resetNodeTask} resetNodeTask
   * @param {Config[]} configGroup
   * @param {configureCoreTask} configureCoreTask
   * @param {configureTenderdashTask} configureTenderdashTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
      hard: isHardReset,
      force: isForce,
      platform: isPlatformOnlyReset,
    },
    resetNodeTask,
    configGroup,
    configureCoreTask,
    configureTenderdashTask,
  ) {
    const groupName = configGroup[0].get('group');

    const tasks = new Listr(
      [
        {
          title: `Reset ${groupName} nodes`,
          task: (ctx) => {
            ctx.removeConfig = ctx.isHardReset && groupName === PRESET_LOCAL;

            const resetTasks = configGroup.map((config) => ({
              title: `Reset ${config.getName()} node`,
              task: () => resetNodeTask(config),
            }));

            return new Listr(resetTasks);
          },
        },
        {
          enabled: (ctx) => !ctx.isHardReset
            && !ctx.isPlatformOnlyReset && groupName === PRESET_LOCAL,
          title: 'Configure Core nodes',
          task: () => configureCoreTask(configGroup),
        },
        {
          enabled: (ctx) => !ctx.isHardReset && groupName === PRESET_LOCAL,
          title: 'Configure Tenderdash nodes',
          task: () => configureTenderdashTask(configGroup),
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
        isForce,
        isPlatformOnlyReset,
        isVerbose,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
