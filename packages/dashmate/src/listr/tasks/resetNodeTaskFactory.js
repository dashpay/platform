const { Listr } = require('listr2');

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
          // Remove containers
          const serviceNames = await dockerCompose
            .getContainersList(
              config.toEnvs({ platformOnly: true }),
              undefined,
              true,
            );

          await dockerCompose.rm(config.toEnvs(), serviceNames);

          // Remove volumes
          const { COMPOSE_PROJECT_NAME: composeProjectName } = config.toEnvs();

          const projectVolumeNames = await dockerCompose.getVolumeNames(
            config.toEnvs({ platformOnly: true }),
          );

          await Promise.all(
            projectVolumeNames
              .map((volumeName) => `${composeProjectName}_${volumeName}`)
              .map(async (volumeName) => docker.getVolume(volumeName).remove()),
          );
        },
      },
      {
        title: `Reset config ${config.getName()}`,
        enabled: (ctx) => ctx.isHardReset,
        task: (ctx) => {
          const baseConfigName = config.get('group') || config.getName();

          if (systemConfigs[baseConfigName]) {
            // Reset config if has a base config
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
