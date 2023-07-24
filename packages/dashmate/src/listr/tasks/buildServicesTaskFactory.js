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
        let buildArgs = [];
        if (process.env.SCCACHE_GHA_ENABLED === 'true') {
          buildArgs = buildArgs.concat([
            '--build-arg',
            'SCCACHE_GHA_ENABLED=true',
            '--build-arg',
            `ACTIONS_CACHE_URL=${process.env.ACTIONS_CACHE_URL}`,
            '--build-arg',
            `ACTIONS_RUNTIME_TOKEN=${process.env.ACTIONS_RUNTIME_TOKEN}`,
          ]);
        }

        const obs = await dockerCompose.build(config, undefined, buildArgs);

        await new Promise((res, rej) => {
          obs
            .subscribe((msg) => ctx.isVerbose && task.stdout().write(msg), rej, res);
        });
      },
    });
  }

  return buildServicesTask;
}

module.exports = buildServicesTaskFactory;
