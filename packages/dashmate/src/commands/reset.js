const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../oclif/command/BaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

class ResetCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {resetSystemConfig} resetSystemConfig
   * @param {isSystemConfig} isSystemConfig
   * @param {Config} config
   * @param {ConfigCollection} configCollection
   * @param {DockerCompose} dockerCompose
   * @param {Docker} docker
   * @param {tenderdashInitTask} tenderdashInitTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
      hard: isHardReset,
      'platform-only': isPlatformOnlyReset,
    },
    resetSystemConfig,
    isSystemConfig,
    config,
    configCollection,
    dockerCompose,
    docker,
    tenderdashInitTask,
  ) {
    if (isHardReset && !isSystemConfig(config.getName())) {
      throw new Error(`Cannot hard reset non-system config "${config.getName()}"`);
    }

    const tasks = new Listr([
      {
        title: 'Stop services',
        task: async () => dockerCompose.stop(config.toEnvs()),
      },
      {
        title: 'Remove all services and associated data',
        enabled: () => !isPlatformOnlyReset,
        task: async () => dockerCompose.down(config.toEnvs()),
      },
      {
        title: 'Remove platform services and associated data',
        enabled: () => isPlatformOnlyReset,
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

          const projectvolumeNames = await dockerCompose.getVolumeNames(config.toEnvs());

          const volumeRemovePromises = projectvolumeNames
            .filter((volumeName) => !coreVolumeNames.includes(volumeName))
            .map((volumeName) => `${composeProjectName}_${volumeName}`)
            .map((volumeName) => docker.getVolume(volumeName).remove());

          await Promise.all(volumeRemovePromises);
        },
      },
      {
        title: `Reset config ${config.getName()}`,
        enabled: () => isHardReset,
        task: () => resetSystemConfig(configCollection, config.getName(), isPlatformOnlyReset),
      },
      {
        title: 'Initialize Tenderdash',
        enabled: () => !isHardReset,
        task: () => tenderdashInitTask(config),
      },
    ],
    {
      renderer: isVerbose ? 'verbose' : 'default',
      rendererOptions: {
        clearOutput: false,
        collapse: false,
        showSubtasks: true,
      },
    });

    try {
      await tasks.run();
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

ResetCommand.description = `Reset node data

Reset node data
`;

ResetCommand.flags = {
  ...BaseCommand.flags,
  hard: flagTypes.boolean({ char: 'h', description: 'reset config as well as data', default: false }),
  'platform-only': flagTypes.boolean({ char: 'p', description: 'reset platform data only', default: false }),
};

module.exports = ResetCommand;
