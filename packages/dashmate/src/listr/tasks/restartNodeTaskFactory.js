const { Listr } = require('listr2');

/**
 * @param {startNodeTask} startNodeTask
 * @param {stopNodeTask} stopNodeTask
 *
 * @return {restartNodeTask}
 */
function restartNodeTaskFactory(startNodeTask, stopNodeTask) {
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
        task: () => stopNodeTask(config),
      },
      {
        task: () => startNodeTask(config),
      },
    ]);
  }

  return restartNodeTask;
}

module.exports = restartNodeTaskFactory;
