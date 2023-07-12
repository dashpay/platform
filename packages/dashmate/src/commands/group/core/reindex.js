const { Listr } = require('listr2');

const { Flags } = require('@oclif/core');

const MuteOneLineError = require('../../../oclif/errors/MuteOneLineError');
const GroupBaseCommand = require('../../../oclif/command/GroupBaseCommand');

class GroupReindexCommand extends GroupBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {isSystemConfig} isSystemConfig
   * @param {reindexNodeTask} reindexNodeTask
   * @param {createRpcClient} createRpcClient
   * @param {dockerCompose} dockerCompose
   * @param {Config[]} configGroup
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
      detach: isDetached,
    },
    isSystemConfig,
    reindexNodeTask,
    createRpcClient,
    dockerCompose,
    configGroup,
  ) {
    const groupName = configGroup[0].get('group');

    const tasks = new Listr({
      title: `Reindex ${groupName} nodes`,
      task: async () => (
        new Listr([
          {
            title: 'Reindex core nodes',
            task: () => (
              new Listr(configGroup.map((config) => ({
                task: () => reindexNodeTask(config),
              })))
            ),
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
      await tasks.run({
        isDetached,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

GroupReindexCommand.description = 'Reindex group Core data';

GroupReindexCommand.flags = {
  ...GroupBaseCommand.flags,
  verbose: Flags.boolean({char: 'v', description: 'use verbose mode for output', default: false}),
  detach: Flags.boolean({
    char: 'd',
    description: 'detach from the process and keep reindexing in the background',
    default: false,
  }),
};

module.exports = GroupReindexCommand;
