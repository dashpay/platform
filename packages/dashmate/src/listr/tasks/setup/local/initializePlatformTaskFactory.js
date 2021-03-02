const { Listr } = require('listr2');

const wait = require('../../../../util/wait');
const baseConfig = require('../../../../../configs/system/base');
const isSeedNode = require('../../../../util/isSeedNode');
const getSeedNodeConfig = require('../../../../util/getSeedNodeConfig');

/**
 *
 * @param {startNodeTask} startNodeTask
 * @param {initTask} initTask
 * @param {activateSporksTask} activateSporksTask
 * @param {DockerCompose} dockerCompose
 * @return {initializePlatformTask}
 */
function initializePlatformTaskFactory(
  startNodeTask,
  initTask,
  activateSporksTask,
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
        task: () => {
          // to activate sporks faster, set miner interval to 2s
          const seedNodeConfig = getSeedNodeConfig(configGroup);
          seedNodeConfig.set('core.miner.interval', '2s');
        },
      },
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
                isMinerEnabled: isSeedNode(config),
              },
            ),
          }));

          return new Listr(startNodeTasks);
        },
      },
      {
        title: 'Wait 20 seconds to ensure all services are running',
        task: () => wait(20000),
      },
      {
        title: 'Activate sporks',
        task: () => activateSporksTask(configGroup),
      },
      {
        task: () => initTask(configGroup[0]),
      },
      {
        task: () => {
          // back to default
          const seedNodeConfig = getSeedNodeConfig(configGroup);
          seedNodeConfig.set('core.miner.interval', baseConfig.core.miner.interval);
        },
      },
      {
        title: 'Stopping nodes',
        task: async () => {
          const stopNodeTasks = configGroup.map((config) => ({
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
