const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

const { NETWORK_LOCAL } = require('../../constants');

class MintCommand extends ConfigBaseCommand {
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
      verbose: isVerbose,
    },
    generateToAddressTask,
    config,
  ) {
    const network = config.get('network');

    if (network !== NETWORK_LOCAL) {
      throw new Error('Only local network supports generation of dash');
    }

    const tasks = new Listr([
      {
        title: `Generate ${amount} dash to address`,
        task: () => generateToAddressTask(config, amount),
      },
    ],
    {
      renderer: isVerbose ? 'verbose' : 'default',
      rendererOptions: {
        showTimer: isVerbose,
        clearOutput: false,
        collapse: false,
        showSubtasks: true,
      },
    });

    try {
      await tasks.run({
        address,
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
  ...ConfigBaseCommand.flags,
  address: flagTypes.string({ char: 'a', description: 'recipient address instead of a new one', default: null }),
};

MintCommand.args = [{
  name: 'amount',
  required: true,
  description: 'amount of dash to be generated to address',
  parse: (input) => parseInt(input, 10),
}];

module.exports = MintCommand;
