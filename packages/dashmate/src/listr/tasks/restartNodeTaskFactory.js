const { Listr } = require('listr2');

/**
 * @param {startNodeTask} startNodeTask
 * @param {stopNodeTask} stopNodeTask
 * @param {buildServicesTask} buildServicesTask
 * @return {restartNodeTask}
 */
function restartNodeTaskFactory(startNodeTask, stopNodeTask, buildServicesTask) {
  /**
   * Restart node
   * @typedef {restartNodeTask}
   *
   * @param {Config} config
   * @param {Object} [options={}]
   * @param {boolean} [options.platformOnly=false]
   *
   * @return {Listr}
   */
  function restartNodeTask(config, options = {}) {
    return new Listr([
      {
        enabled: () => config.get('platform.enable') && config.get('platform.sourcePath') !== null,
        task: (ctx) => {
          ctx.skipBuildServices = true;

          return buildServicesTask(config);
        },
      },
      {
        task: () => stopNodeTask(config, options),
      },
      {
        task: () => startNodeTask(config, options),
      },
    ]);
  }

  return restartNodeTask;
}

module.exports = restartNodeTaskFactory;
