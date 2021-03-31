const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const GroupBaseCommand = require('../../oclif/command/GroupBaseCommand');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

class GroupStartCommand extends GroupBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {startNodeTask} startNodeTask
   * @param {Config[]} configGroup
   * @param {waitForNodeToBeReadyTask} waitForNodeToBeReadyTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      update: isUpdate,
      'wait-for-readiness': waitForReadiness,
      verbose: isVerbose,
    },
    dockerCompose,
    startNodeTask,
    configGroup,
    waitForNodeToBeReadyTask,
  ) {
    const groupName = configGroup[0].get('group');

    const tasks = new Listr(
      [
        {
          title: `Start ${groupName} nodes`,
          task: async () => (
            new Listr(configGroup.map((config) => (
              {
                title: `Starting ${config.getName()} node`,
                task: () => startNodeTask(
                  config,
                  {
                    isUpdate,
                  },
                ),
              }
            )))
          ),
        },
        {
          title: 'Wait for nodes to be ready',
          enabled: () => waitForReadiness,
          task: () => {
            const waitForNodeToBeReadyTasks = configGroup
              .filter((config) => config.has('platform'))
              .map((config) => ({
                task: () => waitForNodeToBeReadyTask(config),
              }));

            return new Listr(waitForNodeToBeReadyTasks);
          },
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

GroupStartCommand.description = 'Start group nodes';

GroupStartCommand.flags = {
  ...GroupBaseCommand.flags,
  update: flagTypes.boolean({ char: 'u', description: 'download updated services before start', default: false }),
  'wait-for-readiness': flagTypes.boolean({ description: 'wait for nodes to be ready', default: false }),
};

module.exports = GroupStartCommand;
