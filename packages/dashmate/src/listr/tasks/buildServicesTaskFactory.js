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

        const doDriveBuild = config.get('platform.drive.abci.docker.build.path');
        const doDAPIBuild = config.get('platform.dapi.api.docker.build.path');

        let serviceName;
        if (doDriveBuild && doDAPIBuild) {
          serviceName = null;
        } else if (!doDriveBuild) {
          serviceName = 'dapi_api';
        } else if (!doDAPIBuild) {
          serviceName = 'drive_abci';
        }

        const buildProcess = await dockerCompose.build(envs, serviceName);

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
