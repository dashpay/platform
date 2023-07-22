const { Listr } = require('listr2');

const { Flags } = require('@oclif/core');

const MuteOneLineError = require('../../../oclif/errors/MuteOneLineError');
const GroupBaseCommand = require('../../../oclif/command/GroupBaseCommand');
const generateEnvs = require('../../../util/generateEnvs');

class GroupReindexCommand extends GroupBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {reindexNodeTask} reindexNodeTask
   * @param {createRpcClient} createRpcClient
   * @param {dockerCompose} dockerCompose
   * @param {ConfigFile} configFile
   * @param {Config[]} configGroup
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
      detach: isDetach,
      force: isForce,
    },
    reindexNodeTask,
    createRpcClient,
    dockerCompose,
    configFile,
    configGroup,
  ) {
    const tasks = new Listr([
      {
        title: 'Check services are not running',
        skip: (ctx) => ctx.isForce,
        task: async (ctx, task) => {
          // Check if any of group nodes started
          const isRunning = await configGroup
            .reduce(async (acc, config) => (await acc || dockerCompose
              .isNodeRunning(generateEnvs(configFile, config))), false);

          let header;

          if (isRunning) {
            header = 'Node group is running. The group nodes will be unavailable until reindex is complete.\n';
          } else {
            header = 'Node group is stopped. The group nodes will be started in order to complete reindex.\n';
          }

          const agreement = await task.prompt({
            type: 'toggle',
            name: 'confirm',
            header,
            message: 'Start reindex?',
            enabled: 'Yes',
            disabled: 'No',
          });

          if (!agreement) {
            throw new Error('Operation is cancelled');
          }
        },
      },
      {
        title: 'Reindex Core services',
        task: (ctx, task) => {
          if (ctx.isDetach) {
            // eslint-disable-next-line no-param-reassign
            task.title = 'Start Core services in reindex mode';
          }

          // Skip prompt for specific node
          ctx.isForce = true;

          return new Listr(configGroup.map((config) => ({
            task: () => reindexNodeTask(config),
          })));
        },
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
    });

    try {
      await tasks.run({
        isForce,
        isDetach,
        isVerbose,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

GroupReindexCommand.description = 'Reindex group Core data';

GroupReindexCommand.flags = {
  ...GroupBaseCommand.flags,
  verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
  detach: Flags.boolean({
    char: 'd',
    description: 'run the reindex process in the background',
    default: false,
  }),
  force: Flags.boolean({
    char: 'f',
    description: 'reindex already running node without confirmation',
    default: false,
  }),
};

module.exports = GroupReindexCommand;
