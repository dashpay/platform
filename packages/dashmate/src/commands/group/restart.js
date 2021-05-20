const { Listr } = require('listr2');
const GroupBaseCommand = require('../../oclif/command/GroupBaseCommand');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

class GroupRestartCommand extends GroupBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {stopNodeTask} stopNodeTask
   * @param {startGroupNodesTask} startGroupNodesTask
   * @param {Config[]} configGroup
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
    },
    dockerCompose,
    stopNodeTask,
    startGroupNodesTask,
    configGroup,
  ) {
    const groupName = configGroup[0].get('group');

    const tasks = new Listr({
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
    });

    try {
      await tasks.run();
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

GroupRestartCommand.description = 'Restart group nodes';

GroupRestartCommand.flags = {
  ...GroupBaseCommand.flags,
};

module.exports = GroupRestartCommand;
