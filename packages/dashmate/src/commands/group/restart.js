import { Flags } from '@oclif/core';
import { Listr } from 'listr2';
import GroupBaseCommand from '../../oclif/command/GroupBaseCommand.js';
import MuteOneLineError from '../../oclif/errors/MuteOneLineError.js';
import printUnwrapped from '../../oclif/errors/printUnwrapped.js';

export default class GroupRestartCommand extends GroupBaseCommand {
  static description = 'Restart group nodes';

  static flags = {
    ...GroupBaseCommand.flags,
    safe: Flags.boolean({
      char: 's',
      description: 'wait for dkg before stop',
      default: false,
    }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {stopNodeTask} stopNodeTask
   * @param {startGroupNodesTask} startGroupNodesTask
   * @param {Config[]} configGroup
   * @param {assertPlatformCanStart} assertPlatformCanStart
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      safe: isSafe,
      verbose: isVerbose,
    },
    dockerCompose,
    stopNodeTask,
    startGroupNodesTask,
    configGroup,
    assertPlatformCanStart,
  ) {
    // Every member before any is stopped: a restart that cannot start again
    // would leave the group down.
    try {
      await configGroup.reduce(
        (checked, config) => checked.then(() => assertPlatformCanStart(config)),
        Promise.resolve(),
      );
    } catch (e) {
      throw printUnwrapped(e);
    }

    const groupName = configGroup[0].get('group');

    const tasks = new Listr(
      {
        title: `Restart ${groupName} nodes`,
        task: async () => (
          new Listr([
            {
              title: 'Stop nodes',
              task: () => (
                // So we stop the miner first, as there's a chance that MNs will get banned
                // if the miner is still running when stopping them
                new Listr(configGroup.reverse().map((config) => ({
                  title: `Stop ${config.getName()} node`,
                  task: () => stopNodeTask(config),
                })))
              ),
            },
            {
              title: 'Start nodes',
              task: () => startGroupNodesTask(configGroup),
            },
          ])
        ),
      },
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
        isSafe,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
