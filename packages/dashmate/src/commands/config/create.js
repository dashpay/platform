import BaseCommand from '../../oclif/command/BaseCommand.js';

export default class ConfigCreateCommand extends BaseCommand {
  static description = 'Create new config';

  static args = [{
    name: 'config',
    required: true,
    description: 'config name',
  }, {
    name: 'from',
    required: false,
    description: 'base new config on existing config',
    default: 'base',
  }];

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
