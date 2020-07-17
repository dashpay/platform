const { Listr } = require('listr2');
const { Observable } = require('rxjs');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../oclif/command/BaseCommand');
const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

const PRESETS = require('../presets');

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
   * @return {Promise<void>}
   */
  async runWithDependencies(
    { port: coreP2pPort, 'external-ip': externalIp },
    {
      'drive-image-build-path': driveImageBuildPath,
      'dapi-image-build-path': dapiImageBuildPath,
    },
    generateToAddressTask,
    registerMasternodeTask,
    initTask,
    generateBlocksWithSDK,
    startNodeTask,
    dockerCompose,
  ) {
    const preset = PRESETS.LOCAL;
    const network = preset;
    const amount = 10000;
    const seed = '127.0.0.1';

    const tasks = new Listr(
      [
        {
          title: 'Setup masternode for local development',
          task: () => new Listr([
            {
              title: `Generate ${amount} dash to address`,
              task: () => generateToAddressTask(preset, amount),
            },
            {
              title: 'Register masternode',
              task: () => registerMasternodeTask(preset),
            },
            {
              title: `Start masternode with ${preset} preset`,
              task: async (ctx) => startNodeTask(
                preset,
                {
                  externalIp: ctx.externalIp,
                  coreP2pPort: ctx.coreP2pPort,
                  operatorPrivateKey: ctx.operator.privateKey,
                  driveImageBuildPath: ctx.driveImageBuildPath,
                  dapiImageBuildPath: ctx.dapiImageBuildPath,
                },
              ),
            },
            {
              title: 'Initialize Platform',
              task: () => initTask(preset),
            },
            {
              title: 'Mine 100 blocks',
              enabled: () => preset === PRESETS.LOCAL,
              task: async (ctx) => (
                new Observable(async (observer) => {
                  await generateBlocksWithSDK(
                    ctx.client.getDAPIClient(),
                    ctx.network,
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
              task: async () => dockerCompose.stop(preset),
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
        externalIp,
        coreP2pPort,
        network,
        seed,
        driveImageBuildPath,
        dapiImageBuildPath,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

SetupForLocalDevelopmentCommand.description = `Setup for development
...
Generate some dash, register masternode and populate node with data required for local development
`;

SetupForLocalDevelopmentCommand.args = [{
  name: 'external-ip',
  required: true,
  description: 'masternode external IP',
}, {
  name: 'port',
  required: true,
  description: 'masternode P2P port',
}];

SetupForLocalDevelopmentCommand.flags = {
  'drive-image-build-path': flagTypes.string({ description: 'drive\'s docker image build path', default: null }),
  'dapi-image-build-path': flagTypes.string({ description: 'dapi\'s docker image build path', default: null }),
};

module.exports = SetupForLocalDevelopmentCommand;
