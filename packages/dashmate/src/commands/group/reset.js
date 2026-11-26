import { Listr } from 'listr2';
import { Flags } from '@oclif/core';
import GroupBaseCommand from '../../oclif/command/GroupBaseCommand.js';
import MuteOneLineError from '../../oclif/errors/MuteOneLineError.js';
import { PRESET_LOCAL } from '../../constants.js';

export default class GroupResetCommand extends GroupBaseCommand {
  // Reconfigures the node: changes configuration repeatedly while doing long,
  // partly irreversible work, so it holds the config lock for its whole run.
  static mutatesConfig = true;

  // A total reset can repair configs whose old options no longer validate.
  static shouldSkipConfigValidation({
    force, hard, platform, group,
  }) {
    if (!force || !hard || platform) {
      return false;
    }

    return ({ options, configFileData }) => (
      options?.group === (group ?? configFileData.defaultGroupName)
    );
  }

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
          // Members are reset one after another, so a member that cannot be
          // reset would otherwise be found only after the others were.
          title: 'Check the reset can be done',
          task: (ctx) => Promise.all(
            configGroup.map((config) => resetNodeTask.assertCanBeReset(config, ctx)),
          ),
        },
        {
          title: `Reset ${groupName} nodes`,
          task: () => {
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
          // Errors explain what to do next on several lines; truncating them
          // to the terminal width would cut the commands off.
          formatOutput: 'wrap',
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
        removeConfig: isHardReset && groupName === PRESET_LOCAL,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
