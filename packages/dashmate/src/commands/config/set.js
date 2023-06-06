const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

class ConfigSetCommand extends ConfigBaseCommand {
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
    // check for existence
    config.get(optionPath);

    let value;

    try {
      value = JSON.parse(optionValue);
    } catch (e) {
      value = optionValue;
    }

    config.set(optionPath, value);

    // eslint-disable-next-line no-console
    console.log(`${optionPath} set to ${optionValue}`);
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
  ...ConfigBaseCommand.flags,
};

module.exports = ConfigSetCommand;
