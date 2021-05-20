const { exec } = require('child_process');
const { promisify } = require('util');

const { Listr } = require('listr2');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {Docker} docker
 * @return {buildServicesTask}
 */
function buildServicesTaskFactory(
  dockerCompose,
  docker,
) {
  const execAsync = promisify(exec);
  const followDockerProgress = promisify(docker.modem.followProgress.bind(docker.modem));

  /**
   * @typedef {buildServicesTask}
   * @param {Config} config
   * @return {Listr}
   */
  function buildServicesTask(config) {
    const serviceBuildConfigs = [
      {
        name: 'Drive',
        buildOptions: config.get('platform.drive.abci.docker.build'),
        serviceName: 'drive_abci',
      },
      {
        name: 'DAPI',
        buildOptions: config.get('platform.dapi.api.docker.build'),
        serviceName: 'dapi_api',
      },
    ];

    const buildTasks = serviceBuildConfigs
      .filter(({ buildOptions }) => buildOptions.path !== null)
      .map(({
        name,
        buildOptions,
        serviceName,
      }) => ({
        title: `Build ${name}`,
        task: () => (
          new Listr([
            {
              title: 'Build Docker image',
              task: async () => {
                const envs = config.toEnvs();

                await dockerCompose.build(envs, serviceName);
              },
            },
            {
              title: 'Update NPM cache',
              task: async () => {
                // Build node_modules stage only to access to npm cache
                const buildStream = await docker.buildImage({
                  context: buildOptions.path,
                  src: ['Dockerfile', 'docker/cache', 'package.json', 'package-lock.json'],
                }, {
                  target: 'node_modules',
                });

                const output = await followDockerProgress(buildStream);

                const buildError = output.find(({ error }) => error);

                if (buildError) {
                  throw new Error(buildError.error);
                }

                const {
                  aux: {
                    ID: nodeModulesImageId,
                  },
                } = output.find(({ aux }) => aux && aux.ID);

                // Copy npm cache from node_modules stage image back to cache dir
                const container = await docker.createContainer({
                  Image: nodeModulesImageId,
                });

                await Promise.all([
                  execAsync(`docker cp ${container.id}:/root/.cache ${buildOptions.path}/docker/cache`),
                  execAsync(`docker cp ${container.id}:/root/.npm ${buildOptions.path}/docker/cache`),
                ]);

                // Remove node_modules stage container and image
                await container.remove();

                const nodeModulesImage = docker.getImage(nodeModulesImageId);
                await nodeModulesImage.remove();
              },
            },
          ])
        ),
      }));

    return new Listr(buildTasks);
  }

  return buildServicesTask;
}

module.exports = buildServicesTaskFactory;
