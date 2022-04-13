const { Listr } = require('listr2');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @return {buildServicesTask}
 */
function buildServicesTaskFactory(
  dockerCompose,
) {
  /**
   * @typedef {buildServicesTask}
   * @param {Config} config
   * @return {Listr}
   */
  function buildServicesTask(config) {
    return new Listr({
      title: 'Build services',
      task: async (ctx, task) => {
        const envs = config.toEnvs();

        const buildProcess = await dockerCompose.build(envs);

        if (ctx.isVerbose) {
          buildProcess.stdout.pipe(task.stdout());
          buildProcess.stderr.pipe(task.stdout());
        }

        await buildProcess.isReady;
      },
    });
  }

  return buildServicesTask;
}

module.exports = buildServicesTaskFactory;
