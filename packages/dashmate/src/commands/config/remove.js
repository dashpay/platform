const BaseCommand = require('../../oclif/command/BaseCommand');

const systemConfigs = require('../../../configs/system');

class ConfigRemoveCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {ConfigFile} configFile
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      config: configName,
    },
    flags,
    configFile,
  ) {
    if (Object.keys(systemConfigs).includes(configName)) {
      throw new Error(`system config ${configName} can't be removed`);
    }

    configFile.removeConfig(configName);

    // eslint-disable-next-line no-console
    console.log(`${configName} removed`);
  }
}

ConfigRemoveCommand.description = `Remove config

Removes a configuration
`;

ConfigRemoveCommand.args = [{
  name: 'config',
  required: true,
  description: 'config name',
}];

module.exports = ConfigRemoveCommand;
