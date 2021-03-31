const { Listr } = require('listr2');

/**
 * @param {DockerCompose} dockerCompose
 * @param {Docker} docker
 * @param {tenderdashInitTask} tenderdashInitTask
 * @param {initTask} initTask
 * @param {startNodeTask} startNodeTask
 * @param {generateToAddressTask} generateToAddressTask
 * @param {systemConfigs} systemConfigs
 * @return {resetNodeTask}
 */
function resetNodeTaskFactory(
  dockerCompose,
  docker,
  tenderdashInitTask,
  initTask,
  startNodeTask,
  generateToAddressTask,
  systemConfigs,
) {
  /**
   * @typedef {resetNodeTask}
   * @param {Config} config
   */
  function resetNodeTask(config) {
    return new Listr([
      {
        title: 'Check services are not running',
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
        enabled: (ctx) => ctx.isPlatformOnlyReset && config.isPlatformServicesEnabled(),
        task: async () => {
          // Remove containers
          const coreContainerNames = ['core', 'sentinel'];
          const containerNames = await dockerCompose
            .getContainersList(config.toEnvs(), undefined, true);
          const platformContainerNames = containerNames
            .filter((containerName) => !coreContainerNames.includes(containerName));

          await dockerCompose.rm(config.toEnvs(), platformContainerNames);

          // Remove volumes
          const coreVolumeNames = ['core_data'];
          const { COMPOSE_PROJECT_NAME: composeProjectName } = config.toEnvs();

          const projectVolumeNames = await dockerCompose.getVolumeNames(config.toEnvs());

          await Promise.all(
            projectVolumeNames
              .filter((volumeName) => !coreVolumeNames.includes(volumeName))
              .map((volumeName) => `${composeProjectName}_${volumeName}`)
              .map(async (volumeName) => docker.getVolume(volumeName).remove()),
          );
        },
      },
      {
        title: `Reset config ${config.getName()}`,
        enabled: (ctx) => ctx.isHardReset,
        task: (ctx) => {
          const name = config.get('group') || config.getName();

          if (ctx.isPlatformOnlyReset) {
            // TODO: This won't work for user created configs
            const { platform: systemPlatformConfig } = systemConfigs[name];
            config.set('platform', systemPlatformConfig);
          } else {
            config.setOptions(systemConfigs[name]);
          }
        },
      },
      {
        title: 'Initialize Tenderdash',
        enabled: (ctx) => (
          !ctx.isHardReset && !ctx.skipPlatformInitialization && config.isPlatformServicesEnabled()
        ),
        task: () => tenderdashInitTask(config),
      },
    ]);
  }

  return resetNodeTask;
}

module.exports = resetNodeTaskFactory;
