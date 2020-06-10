const Listr = require('listr');

const rimraf = require('rimraf');

const BaseCommand = require('../oclif/command/BaseCommand');

const UpdateRendererWithOutput = require('../oclif/renderer/UpdateRendererWithOutput');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

const PRESETS = require('../presets');

class ResetCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      preset,
    },
    flags,
    dockerCompose,
  ) {
    const tasks = new Listr([
      {
        title: `Reset data for ${preset} preset`,
        task: () => (
          new Listr([
            {
              title: 'Remove Tendermint data',
              enabled: () => preset !== PRESETS.TESTNET,
              task: async () => {
                if (await dockerCompose.isServiceRunning(preset)) {
                  throw new Error('You can\'t reset data while MN is running. Please stop it.');
                }

                await dockerCompose.runService(
                  preset,
                  'drive_tendermint',
                  ['tendermint', 'unsafe_reset_all'],
                  ['--entrypoint=""'],
                );
              },
            },
            {
              title: 'Remove Core data',
              task: () => rimraf.sync(`${__dirname}/../../data/${preset}/core/!(.gitignore)`),
            },
            {
              title: 'Remove Drive data',
              enabled: () => preset !== PRESETS.TESTNET,
              task: async () => dockerCompose.down(preset),
            },
          ])
        ),
      },
    ],
    { collapse: false, renderer: UpdateRendererWithOutput });

    try {
      await tasks.run();
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

ResetCommand.description = `Reset masternode data
...
Reset masternode data for specific preset
`;

ResetCommand.args = [{
  name: 'preset',
  required: true,
  description: 'preset to use',
  options: Object.values(PRESETS),
}];

module.exports = ResetCommand;
