const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

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
    flags,
    config,
  ) {
    // eslint-disable-next-line no-console
    console.log(
      config.get(optionPath),
    );
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
  ...ConfigBaseCommand.flags,
};

module.exports = ConfigGetCommand;
