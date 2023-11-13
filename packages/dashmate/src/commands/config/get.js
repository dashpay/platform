import { Flags } from '@oclif/core';
import lodash from 'lodash';
import chalk from 'chalk'
import {inspect} from 'util';
import { OUTPUT_FORMATS } from '../../constants';
import {ConfigBaseCommand} from "../../oclif/command/ConfigBaseCommand.js";

export class ConfigGetCommand extends ConfigBaseCommand {
  static description = `Get config option

Gets a configuration option from the specified config
`;

  static flags = {
    format: Flags.string({
      description: 'display output format',
      default: OUTPUT_FORMATS.PLAIN,
      options: Object.values(OUTPUT_FORMATS),
    }),
    ...ConfigBaseCommand.flags,
  };

  static args = [{
    name: 'option',
    required: true,
    description: 'option path',
  }];

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      option: optionPath,
    },
    {
      format,
    },
    config,
  ) {
    const value = config.get(optionPath);

    let output = value;

    if (format === OUTPUT_FORMATS.JSON) {
      output = JSON.stringify(value, null, 2);
    } else if (Array.isArray(value) || lodash.isPlainObject(value)) {
      output = inspect(value, { depth: Infinity, colors: chalk.supportsColor });
    }

    // eslint-disable-next-line no-console
    console.log(output);

    return value;
  }
}
