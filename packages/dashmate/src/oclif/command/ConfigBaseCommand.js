const { flags: flagTypes } = require('@oclif/command');

const { asValue } = require('awilix');

const BaseCommand = require('./BaseCommand');
const ConfigIsNotPresentError = require('../../config/errors/ConfigIsNotPresentError');

/**
 * @abstract
 */
class GroupBaseCommand extends BaseCommand {
  async run() {
    const configFile = this.container.resolve('configFile');

    let configName;
    if (this.parsedFlags.config !== null) {
      if (!configFile.isConfigExists(this.parsedFlags.config)) {
        throw new ConfigIsNotPresentError(this.parsedFlags.config);
      }

      configName = this.parsedFlags.config;
    } else {
      const defaultConfigName = configFile.getDefaultConfigName();

      if (defaultConfigName === null) {
        throw new Error('Default config is not set. Please use \'--config\' option or set default config');
      }

      if (!configFile.isConfigExists(defaultConfigName)) {
        throw new Error(`Default config ${defaultConfigName} is not exist. Please use '--config' option or change default config`);
      }

      configName = defaultConfigName;
    }

    const config = configFile.getConfig(configName);

    if (config.get('group') !== null) {
      throw new Error(`${config.getName()} config belongs to a group ${config.get('group')}. Please, consider using 'group' commands`);
    }

    this.container.register({
      config: asValue(config),
    });

    const renderServiceTemplates = this.container.resolve('renderServiceTemplates');
    const writeServiceConfigs = this.container.resolve('writeServiceConfigs');

    const serviceConfigFiles = renderServiceTemplates(config);
    writeServiceConfigs(config.getName(), serviceConfigFiles);

    return super.run();
  }
}

GroupBaseCommand.flags = {
  config: flagTypes.string({
    description: 'configuration name to use',
    default: null,
  }),
  ...BaseCommand.flags,
};

module.exports = GroupBaseCommand;
