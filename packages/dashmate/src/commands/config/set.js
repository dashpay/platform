const BaseCommand = require('../../oclif/command/BaseCommand');

class ConfigSetCommand extends BaseCommand {
  /**
   * @param args
   * @param flags
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      option: optionPath,
      value: optionValue,
    },
    flags,
    config,
  ) {
    if (optionValue === 'null') {
      // eslint-disable-next-line no-param-reassign
      optionValue = null;
    }

    config.set(optionPath, optionValue);

    // eslint-disable-next-line no-console
    console.log(`${optionPath} set to ${config.get(optionPath)}`);
  }
}

ConfigSetCommand.description = `Set config option

Sets a configuration option in the default config
`;

ConfigSetCommand.args = [{
  name: 'option',
  required: true,
  description: 'option path',
}, {
  name: 'value',
  required: true,
  description: 'the option value',
}];

ConfigSetCommand.flags = {
  ...BaseCommand.flags,
};

module.exports = ConfigSetCommand;
