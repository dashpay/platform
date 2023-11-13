import { Listr }  from 'listr2';
import { Flags } from '@oclif/core';

import { NETWORK_LOCAL } from '../../constants';
import {ConfigBaseCommand} from "../../oclif/command/ConfigBaseCommand.js";
import {MuteOneLineError} from "../../oclif/errors/MuteOneLineError.js";

export class MintCommand extends ConfigBaseCommand {
  static description = `Mint tDash

Mint given amount of tDash to a new or specified address
`;

  static args = [{
    name: 'amount',
    required: true,
    description: 'amount of tDash to be generated to address',
    parse: (input) => parseInt(input, 10),
  }];

  static flags = {
    ...ConfigBaseCommand.flags,
    address: Flags.string({ char: 'a', description: 'use recipient address instead of creating new', default: null }),
  };

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
