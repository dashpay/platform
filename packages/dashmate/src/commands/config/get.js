const { Flags } = require('@oclif/core');
const lodash = require('lodash');
const chalk = require('chalk');
const { inspect } = require('util');
const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const { OUTPUT_FORMATS } = require('../../constants');

class ConfigGetCommand extends ConfigBaseCommand {
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

ConfigGetCommand.description = `Get config option

Gets a configuration option from the specified config
`;

ConfigGetCommand.args = [{
  name: 'option',
  required: true,
  description: 'option path',
}];

ConfigGetCommand.flags = {
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
  ...ConfigBaseCommand.flags,
};

module.exports = ConfigGetCommand;
