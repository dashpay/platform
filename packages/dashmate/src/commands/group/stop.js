import { Flags } from '@oclif/core';
import { Listr } from 'listr2';
import GroupBaseCommand from '../../oclif/command/GroupBaseCommand.js';
import MuteOneLineError from '../../oclif/errors/MuteOneLineError.js';

export default class GroupStopCommand extends GroupBaseCommand {
  static description = 'Stop group nodes';

  static flags = {
    ...GroupBaseCommand.flags,
    force: Flags.boolean({
      char: 'f',
      description: 'force stop even if any is running',
      default: false,
    }),
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
   * @param {Config[]} configGroup
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      force: isForce,
      safe: isSafe,
      verbose: isVerbose,
    },
    dockerCompose,
    stopNodeTask,
    configGroup,
  ) {
    const groupName = configGroup[0].get('group');

    const tasks = new Listr(
      [
        {
          title: `Stop ${groupName} nodes`,
          task: () => (
            // So we stop the miner first, as there's a chance that MNs will get banned
            // if the miner is still running when stopping them
            new Listr(configGroup.reverse().map((config) => ({
              task: () => stopNodeTask(config),
            })))
          ),
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
        isForce,
        isSafe,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
