import { Args, Flags } from '@oclif/core';
import lodash from 'lodash';
import chalk from 'chalk';
import { inspect } from 'util';
import { OUTPUT_FORMATS } from '../../constants.js';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';

export default class ConfigGetCommand extends ConfigBaseCommand {
  static description = `Get config option

Gets a configuration option from the specified config
`;

  static flags = {
    raw: Flags.boolean({
      description: 'show the stored value instead of the effective one',
      default: false,
    }),
    format: Flags.string({
      description: 'display output format',
      default: OUTPUT_FORMATS.PLAIN,
      options: Object.values(OUTPUT_FORMATS),
    }),
    ...ConfigBaseCommand.flags,
  };

  static args = {
    option: Args.string(
      {
        name: 'option', // name of arg to show in help and reference with args[name]
        required: true, // make the arg required with `required: true`
        description: 'option path', // help description
      },
    ),
  };

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
      raw,
    },
    config,
  ) {
    // An option left unset resolves to a default when read normally; --raw shows
    // that it is unset, which is what tells an operator they are tracking rather
    // than pinned.
    const value = raw ? config.getStored(optionPath) : config.get(optionPath);

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
