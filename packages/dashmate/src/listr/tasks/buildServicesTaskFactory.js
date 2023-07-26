const { Listr } = require('listr2');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {generateEnvs} generateEnvs
 * @return {buildServicesTask}
 */
function buildServicesTaskFactory(
  dockerCompose,
  generateEnvs,
) {
  /**
   * @typedef {buildServicesTask}
   * @param {Config} config
   * @return {Listr}
   */
  function buildServicesTask(config) {
    return new Listr([{
      title: 'Build dependencies',
      enabled: () => config.get('docker.baseImage.build.enabled'),
      task: async (ctx, task) => {
        const envs = {
          ...generateEnvs(config),
          COMPOSE_FILE: 'docker-compose.build.base.yml',
        };

        const obs = await dockerCompose.buildWithEnvs(
          envs,
          { serviceName: 'deps' },
        );

        await new Promise((res, rej) => {
          obs
            .subscribe((msg) => ctx.isVerbose && task.stdout().write(msg), rej, res);
        });
      },
    }, {
      title: 'Build services',
      task: async (ctx, task) => {
        const obs = await dockerCompose.build(config);

        await new Promise((res, rej) => {
          obs
            .subscribe((msg) => ctx.isVerbose && task.stdout().write(msg), rej, res);
        });
      },
    }]);
  }

  return buildServicesTask;
}

module.exports = buildServicesTaskFactory;
