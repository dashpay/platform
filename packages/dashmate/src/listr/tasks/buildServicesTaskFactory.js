const { Listr } = require('listr2');
const generateEnvs = require('../../util/generateEnvs');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {ConfigFile} configFile
 * @return {buildServicesTask}
 */
function buildServicesTaskFactory(
  dockerCompose,
  configFile,
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

        const envs = {
          ...generateEnvs(configFile, config),
          COMPOSE_FILE: 'docker-compose.platform.deps.yml',
        };

        let obs = await dockerCompose.build(envs, 'deps');

        await new Promise((res, rej) => {
          obs
            .subscribe((msg) => ctx.isVerbose && task.stdout().write(msg), rej, res);
        });

        obs = await dockerCompose.build(generateEnvs(configFile, config));

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
