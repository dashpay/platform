const { Flags } = require('@oclif/core');
const { Listr } = require('listr2');
const GroupBaseCommand = require('../../oclif/command/GroupBaseCommand');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

class GroupStopCommand extends GroupBaseCommand {
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
        isForce
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

GroupStopCommand.description = 'Stop group nodes';

GroupStopCommand.flags = {
  ...GroupBaseCommand.flags,
  force: Flags.boolean({
    char: 'f',
    description: 'force stop even if any is running',
    default: false,
  }),
};

module.exports = GroupStopCommand;
