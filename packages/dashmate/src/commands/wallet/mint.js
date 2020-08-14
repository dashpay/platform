const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../../oclif/command/BaseCommand');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

const NETWORKS = require('../../networks');

class MintCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {generateToAddressTask} generateToAddressTask
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      amount,
    },
    {
      address,
    },
    generateToAddressTask,
    config,
  ) {
    const network = config.get('network');

    if (network !== NETWORKS.LOCAL) {
      throw new Error('Only local network supports generation of dash');
    }

    const tasks = new Listr([
      {
        title: `Generate ${amount} dash to address`,
        task: () => generateToAddressTask(config, amount),
      },
    ],
    {
      rendererOptions: {
        clearOutput: false,
        collapse: false,
        showSubtasks: true,
      },
    });

    try {
      await tasks.run({
        fundingAddress: address,
        network,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

MintCommand.description = `Mint dash
...
Mint specified amount of dash to a new address or specified one
`;

MintCommand.flags = {
  ...BaseCommand.flags,
  address: flagTypes.string({ char: 'a', description: 'recipient address instead of a new one', default: null }),
};

MintCommand.args = [{
  name: 'amount',
  required: true,
  description: 'amount of dash to be generated to address',
  parse: (input) => parseInt(input, 10),
}];

module.exports = MintCommand;
