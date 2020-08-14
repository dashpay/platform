const { Listr } = require('listr2');
const { Observable } = require('rxjs');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../oclif/command/BaseCommand');
const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

const NETWORKS = require('../networks');

class SetupForLocalDevelopmentCommand extends BaseCommand {
  /**
   *
   * @param {Object} args
   * @param {Object} flags
   * @param {generateToAddressTask} generateToAddressTask
   * @param {registerMasternodeTask} registerMasternodeTask
   * @param {initTask} initTask
   * @param {generateBlocksWithSDK} generateBlocksWithSDK
   * @param {startNodeTask} startNodeTask
   * @param {DockerCompose} dockerCompose
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      update: isUpdate,
      'drive-image-build-path': driveImageBuildPath,
      'dapi-image-build-path': dapiImageBuildPath,
    },
    generateToAddressTask,
    registerMasternodeTask,
    initTask,
    generateBlocksWithSDK,
    startNodeTask,
    dockerCompose,
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
              title: 'Start masternode',
              task: async (ctx) => startNodeTask(
                config,
                {
                  driveImageBuildPath: ctx.driveImageBuildPath,
                  dapiImageBuildPath: ctx.dapiImageBuildPath,
                  isUpdate,
                },
              ),
            },
            {
              title: 'Initialize Platform',
              task: () => initTask(config),
            },
            {
              title: 'Mine 100 blocks',
              task: async (ctx) => (
                new Observable(async (observer) => {
                  await generateBlocksWithSDK(
                    ctx.client.getDAPIClient(),
                    config.get('network'),
                    100,
                    (blocks) => {
                      observer.next(`${blocks} ${blocks > 1 ? 'blocks' : 'block'} mined`);
                    },
                  );

                  observer.complete();
                })
              ),
            },
            {
              title: 'Stop node',
              task: async () => dockerCompose.stop(config.toEnvs()),
            },
          ]),
        },
      ],
      {
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
};

module.exports = SetupForLocalDevelopmentCommand;
