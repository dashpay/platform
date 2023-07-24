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
        // prebuild dependencies

        // const envs = {
        //   ...generateEnvs(configFile, config),
        //   COMPOSE_FILE: 'docker-compose.platform.deps.yml',
        // };
        //
        // let obs = await dockerCompose.build(config, 'deps');
        //
        // await new Promise((res, rej) => {
        //   obs
        //     .subscribe((msg) => ctx.isVerbose && task.stdout().write(msg), rej, res);
        // });

        const obs = await dockerCompose.build(config);

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
