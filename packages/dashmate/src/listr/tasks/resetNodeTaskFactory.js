const { Listr } = require('listr2');
const fs = require('node:fs');
const path = require('node:path');
const wait = require('../../util/wait');
const generateEnvs = require('../../util/generateEnvs');
const { HOME_DIR_PATH } = require('../../constants');

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
          if (await dockerCompose.isNodeRunning(generateEnvs(configFile, config))) {
            throw new Error('Running services detected. Please ensure all services are stopped for this config before starting');
          }
        },
      },
      {
        title: 'Remove all services and associated data',
        enabled: (ctx) => !ctx.isPlatformOnlyReset,
        task: async () => dockerCompose.down(generateEnvs(configFile, config)),
      },
      {
        title: 'Remove platform services and associated data',
        enabled: (ctx) => ctx.isPlatformOnlyReset,
        task: async () => {
          const nonPlatformServices = ['core', 'sentinel'];
          const envs = generateEnvs(configFile, config);

          // Remove containers
          const serviceNames = (await dockerCompose
            .getContainersList(
              envs,
              { returnServiceNames: true },
            ))
            .filter((serviceName) => !nonPlatformServices.includes(serviceName));

          await dockerCompose.rm(generateEnvs(configFile, config), serviceNames);

          // Remove volumes
          const { COMPOSE_PROJECT_NAME: composeProjectName } = envs;

          const projectVolumeNames = await dockerCompose.getVolumeNames(
            generateEnvs(configFile, config, { platformOnly: true }),
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
                      await dockerCompose.rm(generateEnvs(configFile, config), serviceNames);

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

          // Remove service configs
          let serviceConfigsPath = path.join(HOME_DIR_PATH, baseConfigName);

          if (ctx.isPlatformOnlyReset) {
            serviceConfigsPath = path.join(serviceConfigsPath, 'platform');
          }

          fs.rmSync(serviceConfigsPath, {
            recursive: true,
            force: true,
          });

          // Remove SSL files
          fs.rmSync(path.join(HOME_DIR_PATH, 'ssl', baseConfigName), {
            recursive: true,
            force: true,
          });
        },
      },
    ]);
  }

  return resetNodeTask;
}

module.exports = resetNodeTaskFactory;
