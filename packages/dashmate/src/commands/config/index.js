import { Flags } from '@oclif/core';
import chalk from 'chalk';
import { inspect } from 'util';
import { OUTPUT_FORMATS } from '../../constants.js';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';

export default class ConfigCommand extends ConfigBaseCommand {
  static description = 'Show default config';

  static flags = {
    raw: Flags.boolean({
      description: 'show stored values instead of effective ones',
      default: false,
    }),
    format: Flags.string({
      description: 'display output format',
      default: OUTPUT_FORMATS.PLAIN,
      options: Object.values(OUTPUT_FORMATS),
    }),
    ...ConfigBaseCommand.flags,
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      format,
      raw,
    },
    config,
  ) {
    const options = raw ? config.getStoredOptions() : config.getOptions();

    let configOptions;
    if (format === OUTPUT_FORMATS.JSON) {
      configOptions = JSON.stringify(options, null, 2);
    } else {
      configOptions = inspect(
        options,
        { depth: Infinity, colors: chalk.supportsColor },
      );
    }

    const output = `${config.getName()} config:\n\n${configOptions}`;

    // eslint-disable-next-line no-console
    console.log(output);

    return options;
  }
}
