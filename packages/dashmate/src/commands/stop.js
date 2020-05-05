const Listr = require('listr');

const BaseCommand = require('../oclif/command/BaseCommand');

const UpdateRendererWithOutput = require('../oclif/renderer/UpdateRendererWithOutput');

const MutedError = require('../oclif/errors/MutedError');

const PRESETS = require('../presets');

class StopCommand extends BaseCommand {
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
        title: `Stop masternode with ${preset} preset`,
        task: async () => dockerCompose.stop(preset),
      },
    ],
    { collapse: false, renderer: UpdateRendererWithOutput });

    try {
      await tasks.run();
    } catch (e) {
      // we already output errors through listr
      throw new MutedError(e);
    }
  }
}

StopCommand.description = `Stop masternode
...
Stop masternode with specific preset
`;

StopCommand.args = [{
  name: 'preset',
  required: true,
  description: 'preset to use',
  options: Object.values(PRESETS),
}];

module.exports = StopCommand;
