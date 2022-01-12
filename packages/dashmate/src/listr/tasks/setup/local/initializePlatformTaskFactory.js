const { Listr } = require('listr2');

/**
 * @param {initTask} initTask
 * @param {waitForNodeToBeReadyTask} waitForNodeToBeReadyTask
 * @param {DockerCompose} dockerCompose
 * @param {startGroupNodesTask} startGroupNodesTask
 * @param {stopNodeTask} stopNodeTask
 * @return {initializePlatformTask}
 */
function initializePlatformTaskFactory(
  initTask,
  waitForNodeToBeReadyTask,
  dockerCompose,
  startGroupNodesTask,
  stopNodeTask,
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
        task: (ctx) => {
          ctx.waitForReadiness = true;

          return startGroupNodesTask(configGroup);
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
              config.set('platform.featureFlags', initializedConfig.get('platform.featureFlags'));
            });
        },
      },
      {
        title: 'Stopping nodes',
        task: () => (
          // So we stop the miner first, as there's a chance that MNs will get banned
          // if the miner is still running when stopping them
          new Listr(configGroup.reverse().map((config) => ({
            task: () => stopNodeTask(config),
          })))
        ),
      },
    ]);
  }

  return initializePlatformTask;
}

module.exports = initializePlatformTaskFactory;
