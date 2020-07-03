const Listr = require('listr');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../../oclif/command/BaseCommand');
const UpdateRendererWithOutput = require('../../oclif/renderer/UpdateRendererWithOutput');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

const PRESETS = require('../../presets');

class GenerateToAddressCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {generateToAddressTask} generateToAddressTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    { preset, amount },
    { address },
    generateToAddressTask,
  ) {
    const tasks = new Listr([
      {
        title: `Generate ${amount} dash to address using ${preset} preset`,
        task: () => generateToAddressTask(preset, amount),
      },
    ],
    {
      collapse: false,
      renderer: UpdateRendererWithOutput,
    });

    try {
      await tasks.run({
        fundingAddress: address,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

GenerateToAddressCommand.description = `Generate dash to address
...
Generate specified amount of dash to a new address or specified one
`;

GenerateToAddressCommand.flags = {
  address: flagTypes.string({ char: 'a', description: 'recipient address instead of a new one', default: null }),
};

GenerateToAddressCommand.args = [{
  name: 'preset',
  required: true,
  description: 'preset to use',
  options: [
    PRESETS.EVONET,
    PRESETS.LOCAL,
  ],
}, {
  name: 'amount',
  required: true,
  description: 'amount of dash to be generated to address',
  parse: (input) => parseInt(input, 10),
}];

module.exports = GenerateToAddressCommand;
