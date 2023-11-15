const { Flags } = require('@oclif/core');
const chalk = require('chalk');
const { inspect } = require('util');
const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const { OUTPUT_FORMATS } = require('../../constants');

class ConfigCommand extends ConfigBaseCommand {
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

ConfigCommand.description = 'Show default config';

ConfigCommand.flags = {
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
  ...ConfigBaseCommand.flags,
};

module.exports = ConfigCommand;
