import { Listr } from 'listr2';
import GroupBaseCommand from '../../oclif/command/GroupBaseCommand.js';
import MuteOneLineError from '../../oclif/errors/MuteOneLineError.js';
import isServiceBuildRequired from '../../util/isServiceBuildRequired.js';

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
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {stopNodeTask} stopNodeTask
   * @param {startGroupNodesTask} startGroupNodesTask
   * @param {buildServicesTask} buildServicesTask
   * @param {Config[]} configGroup
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
    buildServicesTask,
    configGroup,
  ) {
    const groupName = configGroup[0].get('group');

    // The whole group shares one set of locally built images, so one config
    // describes the build for all of them
    const buildConfig = configGroup.find(isServiceBuildRequired);

    const tasks = new Listr(
      {
        title: `Restart ${groupName} nodes`,
        task: async () => (
          new Listr([
            {
              // An image built from local sources is in no registry, so the
              // pull below cannot confirm it. Building before the group is
              // stopped is what keeps a build failure from leaving it down
              enabled: () => Boolean(buildConfig),
              title: 'Build services',
              task: (ctx) => {
                // The group start builds the same images, and would otherwise
                // repeat the whole build on every restart
                ctx.skipBuildServices = true;

                return buildServicesTask(buildConfig);
              },
            },
            {
              // Every node's images must be fetched before the first node is
              // stopped, otherwise a failed pull leaves the group stopped
              title: 'Pull missing images',
              task: () => (
                new Listr(configGroup.map((config) => ({
                  task: (ctx, task) => dockerCompose.pullMissingImages(config, {
                    onProgress: (message) => {
                      // eslint-disable-next-line no-param-reassign
                      task.output = message;
                    },
                  }),
                })))
              ),
            },
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
