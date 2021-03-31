const { Listr } = require('listr2');

/**
 *
 * @param {startNodeTask} startNodeTask
 * @param {initTask} initTask
 * @param {waitForNodeToBeReadyTask} waitForNodeToBeReadyTask
 * @param {DockerCompose} dockerCompose
 * @return {initializePlatformTask}
 */
function initializePlatformTaskFactory(
  startNodeTask,
  initTask,
  waitForNodeToBeReadyTask,
  dockerCompose,
) {
  /**
   * @typedef initializePlatformTask
   * @param {Config[]} configGroup
   * @return {Listr}
   */
  function initializePlatformTask(configGroup) {
    return new Listr([
      {
        title: 'Starting nodes',
        task: async (ctx) => {
          const startNodeTasks = configGroup.map((config) => ({
            title: `Starting ${config.getName()} node`,
            task: () => startNodeTask(
              config,
              {
                driveImageBuildPath: ctx.driveImageBuildPath,
                dapiImageBuildPath: ctx.dapiImageBuildPath,
                // run miner only at seed node
                isMinerEnabled: !config.has('platform'),
              },
            ),
          }));

          return new Listr(startNodeTasks);
        },
      },
      {
        title: 'Wait for nodes to be ready',
        task: () => {
          const waitForNodeToBeReadyTasks = configGroup
            .filter((config) => config.has('platform'))
            .map((config) => ({
              task: () => waitForNodeToBeReadyTask(config),
            }));

          return new Listr(waitForNodeToBeReadyTasks);
        },
      },
      {
        task: () => initTask(configGroup[0]),
      },
      {
        task: () => {
          // set platform data contracts
          const [initializedConfig, ...otherConfigs] = configGroup;

          otherConfigs
            .filter((config) => config.has('platform'))
            .forEach((config) => {
              config.set('platform.dpns', initializedConfig.get('platform.dpns'));
              config.set('platform.dashpay', initializedConfig.get('platform.dashpay'));
            });
        },
      },
      {
        title: 'Stopping nodes',
        task: async () => {
          // So we stop the miner first, as there's a chance that MNs will get banned
          // if the miner is still running when stopping them
          const stopNodeTasks = configGroup.reverse().map((config) => ({
            title: `Stop ${config.getName()} node`,
            task: () => dockerCompose.stop(config.toEnvs()),
          }));

          return new Listr(stopNodeTasks);
        },
      },
    ]);
  }

  return initializePlatformTask;
}

module.exports = initializePlatformTaskFactory;
