const BaseCommand = require('../../oclif/command/BaseCommand');

class ConfigCreateCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {renderServiceTemplates} renderServiceTemplates
   * @param {writeServiceConfigs} writeServiceConfigs
   * @param {ConfigFileJsonRepository} configFileRepository
   * @param {ConfigFile} configFile
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      config: configName,
      from: fromConfigName,
    },
    flags,
    renderServiceTemplates,
    writeServiceConfigs,
    configFileRepository,
    configFile,
  ) {
    configFile.createConfig(configName, fromConfigName);

    configFileRepository.write(configFile);

    const serviceConfigs = renderServiceTemplates(configFile.getConfig(configName));
    writeServiceConfigs(configName, serviceConfigs);

    // eslint-disable-next-line no-console
    console.log(`${configName} created`);
  }
}

ConfigCreateCommand.description = 'Create new config';

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
