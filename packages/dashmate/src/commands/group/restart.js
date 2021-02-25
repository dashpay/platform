const { Listr } = require('listr2');
const GroupBaseCommand = require('../../oclif/command/GroupBaseCommand');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

class GroupRestartCommand extends GroupBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {restartNodeTask} restartNodeTask
   * @param {Config[]} configGroup
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
    },
    dockerCompose,
    restartNodeTask,
    configGroup,
  ) {
    const groupName = configGroup[0].get('group');

    const tasks = new Listr(
      [
        {
          title: `Restart ${groupName} nodes`,
          task: async () => (
            new Listr(configGroup.map((config) => (
              {
                title: `Restarting ${config.getName()} node`,
                task: () => restartNodeTask(config),
              }
            )))
          ),
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
        },
      },
    );

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
