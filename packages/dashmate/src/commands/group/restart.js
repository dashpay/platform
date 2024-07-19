import { Listr } from 'listr2';
import GroupBaseCommand from '../../oclif/command/GroupBaseCommand.js';
import MuteOneLineError from '../../oclif/errors/MuteOneLineError.js';

export default class GroupRestartCommand extends GroupBaseCommand {
  static description = 'Restart group nodes';

  static flags = {
    ...GroupBaseCommand.flags,
    safe: {
      char: 's',
      description: 'wait for dkg before stop',
      default: false,
    },
  };

  /**
   * @param {Object} flags
   * @param {stopNodeTask} stopNodeTask
   * @param {startGroupNodesTask} startGroupNodesTask
   * @param {Config[]} configGroup
   * @return {Promise<void>}
   */
  async runWithDependencies({
    flags: {
      safe: isSafe,
      verbose: isVerbose,
    },
    stopNodeTask,
    startGroupNodesTask,
    configGroup,
  }) {
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
