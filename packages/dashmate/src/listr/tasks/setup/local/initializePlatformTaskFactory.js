const { Listr } = require('listr2');

/**
 * @param {waitForNodeToBeReadyTask} waitForNodeToBeReadyTask
 * @param {DockerCompose} dockerCompose
 * @param {startGroupNodesTask} startGroupNodesTask
 * @param {stopNodeTask} stopNodeTask
 * @return {initializePlatformTask}
 */
function initializePlatformTaskFactory(
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
