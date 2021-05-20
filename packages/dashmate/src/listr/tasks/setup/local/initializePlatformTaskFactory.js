const { Listr } = require('listr2');

/**
 * @param {initTask} initTask
 * @param {waitForNodeToBeReadyTask} waitForNodeToBeReadyTask
 * @param {DockerCompose} dockerCompose
 * @param {startGroupNodesTask} startGroupNodesTask
 * @return {initializePlatformTask}
 */
function initializePlatformTaskFactory(
  initTask,
  waitForNodeToBeReadyTask,
  dockerCompose,
  startGroupNodesTask,
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
        title: 'Activating feature flags',
        task: async (ctx) => {
          const enableAtHeight = parseInt(ctx.featureFlagsContractBlockHeight, 10) + 1;

          const cumulativeFeesDocument = await ctx.client.platform.documents.create(
            'featureFlags.fixCumulativeFeesBug',
            ctx.featureFlagsIdentity,
            {
              enabled: true,
              enableAtHeight,
            },
          );

          const verifyLLMQDocument = await ctx.client.platform.documents.create(
            'featureFlags.verifyLLMQSignaturesWithCore',
            ctx.featureFlagsIdentity,
            {
              enabled: true,
              enableAtHeight,
            },
          );

          await ctx.client.platform.documents.broadcast({
            create: [cumulativeFeesDocument, verifyLLMQDocument],
          }, ctx.featureFlagsIdentity);
        },
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
