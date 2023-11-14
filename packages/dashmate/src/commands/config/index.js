import { Flags } from '@oclif/core';
import chalk from 'chalk';
import { inspect } from 'util';
import { OUTPUT_FORMATS } from '../../constants.js';
import { ConfigBaseCommand } from '../../oclif/command/ConfigBaseCommand.js';

export class ConfigCommand extends ConfigBaseCommand {
  static description = 'Show default config';

  static flags = {
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
    },
    config,
  ) {
    let configOptions;
    if (format === OUTPUT_FORMATS.JSON) {
      configOptions = JSON.stringify(config.getOptions(), null, 2);
    } else {
      configOptions = inspect(
        config.getOptions(),
        { depth: Infinity, colors: chalk.supportsColor },
      );
    }

    const output = `${config.getName()} config:\n\n${configOptions}`;

    // eslint-disable-next-line no-console
    console.log(output);

    return config.getOptions();
  }
}
