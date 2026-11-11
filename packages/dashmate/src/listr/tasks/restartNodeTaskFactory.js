import { Listr } from 'listr2';
import isServiceBuildRequired from '../../util/isServiceBuildRequired.js';

/**
 * @param {startNodeTask} startNodeTask
 * @param {stopNodeTask} stopNodeTask
 * @param {buildServicesTask} buildServicesTask
 * @param {DockerCompose} dockerCompose
 * @param {getConfigProfiles} getConfigProfiles
 * @return {restartNodeTask}
 */
export default function restartNodeTaskFactory(
  startNodeTask,
  stopNodeTask,
  buildServicesTask,
  dockerCompose,
  getConfigProfiles,
) {
  function selectPlatformProfiles(config) {
    return getConfigProfiles(config)
      .filter((profile) => profile.startsWith('platform'));
  }

  /**
   * Restart node
   * @typedef {restartNodeTask}
   *
   * @param {Config} config
   *
   * @return {Listr}
   */
  function restartNodeTask(config) {
    return new Listr([
      {
        enabled: () => isServiceBuildRequired(config),
        task: (ctx) => {
          ctx.skipBuildServices = true;

          return buildServicesTask(config);
        },
      },
      {
        // Missing images must be fetched while the node is still running,
        // otherwise a failed pull leaves it stopped
        title: 'Pull missing images',
        task: (ctx, task) => {
          // Pull only what the following start is going to create
          const profiles = ctx.platformOnly ? selectPlatformProfiles(config) : [];

          return dockerCompose.pullMissingImages(config, {
            profiles,
            onProgress: (message) => {
              // eslint-disable-next-line no-param-reassign
              task.output = message;
            },
          });
        },
      },
      {
        task: () => stopNodeTask(config),
      },
      {
        task: () => startNodeTask(config),
      },
    ]);
  }

  return restartNodeTask;
}
