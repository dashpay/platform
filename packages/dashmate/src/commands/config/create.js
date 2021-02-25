const BaseCommand = require('../../oclif/command/BaseCommand');

class ConfigCreateCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {ConfigFile} configFile
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      config: configName,
      from: fromConfigName,
    },
    flags,
    configFile,
  ) {
    configFile.createConfig(configName, fromConfigName);

    // eslint-disable-next-line no-console
    console.log(`${configName} created`);
  }
}

ConfigCreateCommand.description = `Create config

Creates a new configuration
`;

ConfigCreateCommand.args = [{
  name: 'config',
  required: true,
  description: 'config name',
}, {
  name: 'from',
  required: false,
  description: 'base new config on existing config',
  default: 'base',
}];

module.exports = ConfigCreateCommand;
