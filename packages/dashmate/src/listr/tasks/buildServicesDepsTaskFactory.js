const { Listr } = require('listr2');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {Docker} docker
 * @return {buildServicesDepsTask}
 */
function buildServicesDepsTaskFactory(
  dockerCompose,
) {
  /**
   * @typedef {buildServicesDepsTask}
   * @param {Config} config
   * @return {Listr}
   */
  function buildServicesDepsTask(config) {
    return new Listr({
      title: 'Build services dependencies',
      task: async (ctx, task) => {
        const envs = {
          ...config.toEnvs(),
          COMPOSE_FILE: 'docker-compose.platform.deps.yml',
        };

        const obs = await dockerCompose.build(envs, 'deps');

        await new Promise((res, rej) => {
          obs
            .subscribe((msg) => ctx.isVerbose && task.stdout().write(msg), rej, res);
        });
      },
    });
  }

  return buildServicesDepsTask;
}

module.exports = buildServicesDepsTaskFactory;
