import { Args } from '@oclif/core';
import BaseCommand from '../../oclif/command/BaseCommand.js';

export default class ConfigCreateCommand extends BaseCommand {
  static description = 'Create new config';

  static args = {
    config: Args.string({
      name: 'config',
      required: true,
      description: 'config name',
    }),
    from: Args.string({
      name: 'from',
      required: false,
      description: 'base new config on existing config',
      default: 'base',
    }),
  };

  /**
   * @param {Object} args
   * @param {ConfigFile} configFile
   * @return {Promise<void>}
   */
  async runWithDependencies({
    args: {
      config: configName,
      from: fromConfigName,
    },
    configFile,
  }) {
    configFile.createConfig(configName, fromConfigName);

    // eslint-disable-next-line no-console
    console.log(`${configName} created`);
  }
}
