const { Listr } = require('listr2');

const wait = require('../../../../util/wait');

function initializePlatformTaskFactory(
  startNodeTask,
  initTask,
  dockerCompose,
) {
  /**
   * @param {Config} config
   * @return {boolean}
   */
  function isSeedNode(config) {
    return config.getName() === 'local_seed';
  }

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
        task: () => initTask(configGroup[0]),
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
