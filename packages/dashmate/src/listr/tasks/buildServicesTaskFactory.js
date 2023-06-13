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
        const envs = generateEnvs(configFile, config);

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

        const obs = await dockerCompose.build(envs, undefined, buildArgs);

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
