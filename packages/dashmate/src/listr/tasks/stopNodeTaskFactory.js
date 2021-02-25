const { Listr } = require('listr2');

/**
 * @param {DockerCompose} dockerCompose
 *
 * @return {stopNodeTask}
 */
function stopNodeTaskFactory(dockerCompose) {
  /**
   * Stop node
   * @typedef stopNodeTask
   * @param {Config} config
   *
   * @return {Listr}
   */
  function stopNodeTask(config) {
    return new Listr([
      {
        title: `Stopping ${config.getName()} node`,
        task: async () => dockerCompose.stop(config.toEnvs()),
      },
    ]);
  }

  return stopNodeTask;
}

module.exports = stopNodeTaskFactory;
