const { Listr } = require('listr2');
const wait = require('../../util/wait');

/**
 * @param {DockerCompose} dockerCompose
 * @param {Docker} docker
 * @param {startNodeTask} startNodeTask
 * @param {generateToAddressTask} generateToAddressTask
 * @param {systemConfigs} systemConfigs
 * @param {ConfigFile} configFile
 * @return {resetNodeTask}
 */
function resetNodeTaskFactory(
  dockerCompose,
  docker,
  startNodeTask,
  generateToAddressTask,
  systemConfigs,
  configFile,
) {
  /**
   * @typedef {resetNodeTask}
   * @param {Config} config
   */
  function resetNodeTask(config) {
    return new Listr([
      {
        task: (ctx) => {
          if (!config.get('platform.enable') && ctx.isPlatformOnlyReset) {
            throw new Error('Cannot reset platform only if platform services are not enabled in config');
          }
        },
      },
      {
        title: 'Check services are not running',
        skip: (ctx) => ctx.isForce,
        task: async () => {
          if (await dockerCompose.isServiceRunning(config.toEnvs())) {
            throw new Error('Running services detected. Please ensure all services are stopped for this config before starting');
          }
        },
      },
      {
        title: 'Remove all services and associated data',
        enabled: (ctx) => !ctx.isPlatformOnlyReset,
        task: async () => dockerCompose.down(config.toEnvs()),
      },
      {
        title: 'Remove platform services and associated data',
        enabled: (ctx) => ctx.isPlatformOnlyReset,
        task: async () => {
          const nonPlatformServices = ['core', 'sentinel'];
          const envs = config.toEnvs();

          // Remove containers
          const serviceNames = (await dockerCompose
            .getContainersList(
              envs,
              undefined,
              true,
            ))
            .filter((serviceName) => !nonPlatformServices.includes(serviceName));

          await dockerCompose.rm(config.toEnvs(), serviceNames);

          // Remove volumes
          const { COMPOSE_PROJECT_NAME: composeProjectName } = envs;

          const projectVolumeNames = await dockerCompose.getVolumeNames(
            config.toEnvs({ platformOnly: true }),
          );

          await Promise.all(
            projectVolumeNames
              .map((volumeName) => `${composeProjectName}_${volumeName}`)
              .map(async (volumeName) => {
                const volume = await docker.getVolume(volumeName);

                let isRetry;
                do {
                  isRetry = false;

                  try {
                    await volume.remove({ force: true });
                  } catch (e) {
                    // volume is in use
                    if (e.statusCode === 409) {
                      await wait(1000);

                      // Remove containers
                      await dockerCompose.rm(config.toEnvs(), serviceNames);

                      isRetry = true;

                      continue;
                    }

                    // volume does not exist
                    if (e.statusCode === 404) {
                      break;
                    }

                    throw e;
                  }
                } while (isRetry);
              }),
          );
        },
      },
      {
        title: 'Reset dashmate\'s ephemeral data',
        task: (ctx) => {
          if (!ctx.isPlatformOnlyReset) {
            config.set('core.miner.mediantime', null);
          }
        },
      },
      {
        title: `Reset config ${config.getName()}`,
        enabled: (ctx) => ctx.isHardReset,
        task: (ctx) => {
          const baseConfigName = config.get('group') || config.getName();

          if (systemConfigs[baseConfigName]) {
            // Reset config if the corresponding base config exists
            if (ctx.isPlatformOnlyReset) {
              const { platform: systemPlatformConfig } = systemConfigs[baseConfigName];
              config.set('platform', systemPlatformConfig);
            } else {
              config.setOptions(systemConfigs[baseConfigName]);
            }
          } else {
            // Delete config if no base config
            configFile.removeConfig(config.getName());
          }
        },
      },
    ]);
  }

  return resetNodeTask;
}

module.exports = resetNodeTaskFactory;
