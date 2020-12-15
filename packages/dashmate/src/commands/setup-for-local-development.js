const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../oclif/command/BaseCommand');
const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

const NETWORKS = require('../networks');
const wait = require('../util/wait');

class SetupForLocalDevelopmentCommand extends BaseCommand {
  /**
   *
   * @param {Object} args
   * @param {Object} flags
   * @param {generateToAddressTask} generateToAddressTask
   * @param {registerMasternodeTask} registerMasternodeTask
   * @param {initTask} initTask
   * @param {startNodeTask} startNodeTask
   * @param {DockerCompose} dockerCompose
   * @param {renderServiceTemplates} renderServiceTemplates
   * @param {writeServiceConfigs} writeServiceConfigs
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      update: isUpdate,
      'drive-image-build-path': driveImageBuildPath,
      'dapi-image-build-path': dapiImageBuildPath,
      verbose: isVerbose,
    },
    generateToAddressTask,
    registerMasternodeTask,
    initTask,
    startNodeTask,
    dockerCompose,
    renderServiceTemplates,
    writeServiceConfigs,
    config,
  ) {
    if (config.get('network') !== NETWORKS.LOCAL) {
      throw new Error(`This command supposed to work only with local network. Your network is ${config.get('network')}`);
    }

    const amount = 10000;

    const tasks = new Listr(
      [
        {
          title: 'Setup masternode for local development',
          task: () => new Listr([
            {
              title: `Generate ${amount} dash to address`,
              task: () => generateToAddressTask(config, amount),
            },
            {
              title: 'Register masternode',
              task: () => registerMasternodeTask(config),
            },
            {
              title: 'Set initial core chain locked height',
              task: async () => {
                config.set('platform.drive.tenderdash.genesis.initial_core_chain_locked_height', 1000);

                const configFiles = renderServiceTemplates(config);
                writeServiceConfigs(config.getName(), configFiles);
              },
            },
            {
              title: 'Start masternode',
              task: async (ctx) => startNodeTask(
                config,
                {
                  driveImageBuildPath: ctx.driveImageBuildPath,
                  dapiImageBuildPath: ctx.dapiImageBuildPath,
                  isUpdate,
                  isMinerEnabled: true,
                },
              ),
            },
            {
              title: 'Wait 20 seconds to ensure all services are running',
              task: async () => {
                await wait(20000);
              },
            },
            {
              title: 'Initialize Platform',
              task: () => initTask(config),
            },
            {
              title: 'Stop node',
              task: async () => dockerCompose.stop(config.toEnvs()),
            },
          ]),
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
        },
      },
    );

    try {
      await tasks.run({
        driveImageBuildPath,
        dapiImageBuildPath,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

SetupForLocalDevelopmentCommand.description = `Setup for development

Generate some dash, register masternode and populate node with data required for local development
`;

SetupForLocalDevelopmentCommand.flags = {
  ...BaseCommand.flags,
  update: flagTypes.boolean({ char: 'u', description: 'download updated services before start', default: false }),
  'drive-image-build-path': flagTypes.string({ description: 'drive\'s docker image build path', default: null }),
  'dapi-image-build-path': flagTypes.string({ description: 'dapi\'s docker image build path', default: null }),
  verbose: flagTypes.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
};

module.exports = SetupForLocalDevelopmentCommand;
