const { Listr } = require('listr2');
const fs = require('node:fs');
const path = require('node:path');
const wait = require('../../util/wait');

/**
 * @param {DockerCompose} dockerCompose
 * @param {Docker} docker
 * @param {startNodeTask} startNodeTask
 * @param {generateToAddressTask} generateToAddressTask
 * @param {DefaultConfigs} defaultConfigs
 * @param {ConfigFile} configFile
 * @param {HomeDir} homeDir
 * @param {generateEnvs} generateEnvs
 * @return {resetNodeTask}
 */
function resetNodeTaskFactory(
  dockerCompose,
  docker,
  startNodeTask,
  generateToAddressTask,
  defaultConfigs,
  configFile,
  homeDir,
  generateEnvs,
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
        task: async (ctx) => {
          if (await dockerCompose.isNodeRunning(config,
            { profiles: ctx.isPlatformOnlyReset ? ['platform'] : [] })) {
            throw new Error('Running services detected. Please ensure all services are stopped for this config before starting');
          }
        },
      },
      {
        title: 'Remove all services and associated data',
        enabled: (ctx) => !ctx.isPlatformOnlyReset,
        task: async () => dockerCompose.down(config),
      },
      {
        title: 'Remove platform services and associated data',
        enabled: (ctx) => ctx.isPlatformOnlyReset,
        task: async () => {
          await dockerCompose.rm(config, { profiles: ['platform'] });

          // Remove volumes
          const { COMPOSE_PROJECT_NAME: composeProjectName } = generateEnvs(config);

          const projectVolumeNames = await dockerCompose.getVolumeNames(
            config,
            { profiles: ['platform'] },
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
                      await dockerCompose.rm(config, { profiles: ['platform'] });

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
            // TODO: We should remove it from config
            config.set('core.miner.mediantime', null);
          }
        },
      },
      {
        title: `Reset config ${config.getName()}`,
        enabled: (ctx) => ctx.isHardReset,
        task: (ctx) => {
          const baseConfigName = config.get('group') || config.getName();

          if (defaultConfigs.has(baseConfigName)) {
            // Reset config if the corresponding base config exists
            if (ctx.isPlatformOnlyReset) {
              const defaultPlatformConfig = defaultConfigs.get(baseConfigName).get('platform');
              config.set('platform', defaultPlatformConfig);
            } else {
              config.setOptions(defaultConfigs.get(baseConfigName).getOptions());
            }
          } else {
            // Delete config if no base config
            configFile.removeConfig(config.getName());
          }

          // Remove service configs
          let serviceConfigsPath = homeDir.joinPath(baseConfigName);

          if (ctx.isPlatformOnlyReset) {
            serviceConfigsPath = path.join(serviceConfigsPath, 'platform');
          }

          fs.rmSync(serviceConfigsPath, {
            recursive: true,
            force: true,
          });

          // Remove SSL files
          fs.rmSync(homeDir.joinPath('ssl', baseConfigName), {
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
